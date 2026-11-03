//! [`Platformer`]: one playable character, plug and play: movement, its animation, and (with
//! the `toolkit` feature) the controls.
//!
//! ```
//! use game_toolkit_platformer::prelude::*;
//!
//! let mut level = Level::demo(); // or your own `Terrain`
//! let mut hero = Platformer::builder()
//!     .preset(Preset::Snappy)
//!     .tune(|t| t.jump_speed = 115.0)
//!     .spawn_on(&level)
//!     .build()
//!     .unwrap();
//!
//! // Each frame: an intent (or `hero.update(&ctx.input, &mut level, dt)` with
//! // game-toolkit), then whatever happened, and what to draw.
//! hero.advance(Intent { move_x: 1.0, ..Intent::default() }, &mut level, 1.0 / 60.0);
//! for event in hero.drain_events() {
//!     let _ = event; // Jumped, Landed, Dashed, ...: sounds, effects
//! }
//! let (_x, _y) = hero.position();
//! let _frame = hero.sprite();
//! ```

use std::sync::Arc;

use crate::anim::{AnimEvent, Animator, Driver, GraphSpec, SpecError};
#[cfg(feature = "toolkit")]
use crate::controls::{Controls, Scheme};
use crate::hero::{HeroDriver, spec};
use crate::sim::{DT, Event, Intent, Level, Player, Preset, Rect, TILE, Terrain, Tuning, World};

/// At most this many fixed steps per frame, so a hitch doesn't spiral.
const MAX_STEPS: u32 = 8;

/// One playable character. The world around it is yours: pass your [`Terrain`] (or a
/// [`Level`]) to each update.
pub struct Platformer {
    /// The simulation: level, body, tuning. Read it for anything the accessors don't cover.
    pub world: World,
    /// The animation graph's runtime.
    pub animator: Animator,
    /// Movement facts and events to graph parameters.
    driver: HeroDriver,
    /// The preset the tuning came from, if any.
    preset: Option<Preset>,
    /// Frame time not yet stepped.
    acc: f32,
    /// Presses seen since the last step.
    latch: Intent,
    /// The body's position before the last step, for drawing between steps.
    prev: (f32, f32),
    /// Movement events since the last drain.
    events: Vec<Event>,
    /// Animation events since the last drain.
    anim_events: Vec<AnimEvent>,
    /// The controls, read from game-toolkit's input in [`Platformer::update`].
    #[cfg(feature = "toolkit")]
    pub controls: Controls,
}

/// Where a [`Builder`] puts the character.
#[derive(Clone, Copy, Debug)]
enum Spawn {
    /// The hitbox's top-left, units.
    At(i32, i32),
    /// Standing on `floor`, centered on `x` (units).
    Standing {
        /// The feet's center.
        x: i32,
        /// The top of the ground.
        floor: i32,
    },
}

/// Sets up a [`Platformer`]: preset, tuning, controls, animation graph, spawn point.
pub struct Builder {
    /// The movement feel.
    tuning: Tuning,
    /// The preset it came from.
    preset: Option<Preset>,
    /// A custom animation graph, or the built-in one.
    graph: Option<GraphSpec>,
    /// Where to start.
    spawn: Spawn,
    /// Put it back at the start after falling out.
    respawn: bool,
    /// The control scheme, or the preset's.
    #[cfg(feature = "toolkit")]
    scheme: Option<Scheme>,
}

impl Builder {
    /// Feel (and, with game-toolkit, controls) of `preset`.
    pub fn preset(mut self, preset: Preset) -> Self {
        self.preset = Some(preset);
        self.tuning = preset.tuning();
        self
    }

    /// A tuning of your own (replaces the preset's numbers).
    pub fn tuning(mut self, tuning: Tuning) -> Self {
        self.tuning = tuning;
        self
    }

    /// Adjust the tuning so far, e.g. `.tune(|t| t.air_jumps = 2)`.
    pub fn tune(mut self, f: impl FnOnce(&mut Tuning)) -> Self {
        f(&mut self.tuning);
        self
    }

    /// An animation graph of your own instead of the built-in one. It has to declare the
    /// parameters the built-in driver writes (see `hero::spec`), though it may ignore any
    /// of them; [`Builder::build`] says which one is missing.
    pub fn graph(mut self, graph: GraphSpec) -> Self {
        self.graph = Some(graph);
        self
    }

    /// Start with the hitbox's top-left at `x, y` (units).
    pub fn spawn_at(mut self, x: i32, y: i32) -> Self {
        self.spawn = Spawn::At(x, y);
        self
    }

    /// Start standing on the ground at height `floor_y` (its top edge), centered on `x`
    /// (units): the way to spawn on a [`Terrain`] of your own.
    pub fn spawn_standing(mut self, x: i32, floor_y: i32) -> Self {
        self.spawn = Spawn::Standing { x, floor: floor_y };
        self
    }

    /// Start standing on `level`'s spawn tile.
    pub fn spawn_on(mut self, level: &Level) -> Self {
        let (tx, ty) = level.spawn;
        self = self.spawn_standing(tx * TILE + TILE / 2, (ty + 1) * TILE);
        self
    }

    /// Whether to put the character back at its start when it falls out of the world
    /// (default: yes). [`Event::FellOut`] is reported either way, so you can do your own.
    pub fn respawn(mut self, on: bool) -> Self {
        self.respawn = on;
        self
    }

    /// A control scheme of your own instead of the preset's.
    #[cfg(feature = "toolkit")]
    pub fn controls(mut self, scheme: Scheme) -> Self {
        self.scheme = Some(scheme);
        self
    }

    /// The character, or why the animation graph doesn't fit.
    pub fn build(self) -> Result<Platformer, SpecError> {
        let graph = self.graph.unwrap_or_else(spec).compile()?;
        let animator = Animator::new(Arc::new(graph));
        let driver = HeroDriver::try_new(&animator.params)?;
        let spawn = match self.spawn {
            Spawn::At(x, y) => (x, y),
            Spawn::Standing { x, floor } => Player::standing_at(x, floor),
        };
        let mut world = World::new(self.tuning, spawn);
        world.respawn = self.respawn;
        Ok(Platformer {
            prev: world.player.exact_pos(),
            world,
            animator,
            driver,
            preset: self.preset,
            acc: 0.0,
            latch: Intent::default(),
            events: Vec::new(),
            anim_events: Vec::new(),
            #[cfg(feature = "toolkit")]
            controls: Controls::new(
                self.scheme
                    .unwrap_or_else(|| crate::controls::Scheme::of(self.preset)),
            ),
        })
    }
}

impl Platformer {
    /// A character on `level`'s spawn tile with `preset`'s feel and controls and the
    /// built-in animation.
    pub fn on(level: &Level, preset: Preset) -> Self {
        Self::builder()
            .preset(preset)
            .spawn_on(level)
            .build()
            .expect("the built-in animation graph fits the built-in driver")
    }

    /// A character with its hitbox's top-left at `x, y` (units), otherwise like
    /// [`Platformer::on`]. On a terrain of your own, [`Builder::spawn_standing`] works out
    /// the top-left from where the feet go.
    pub fn at(x: i32, y: i32, preset: Preset) -> Self {
        Self::builder()
            .preset(preset)
            .spawn_at(x, y)
            .build()
            .expect("the built-in animation graph fits the built-in driver")
    }

    /// Set one up step by step; starts from the hybrid preset at `0, 0`.
    pub fn builder() -> Builder {
        Builder {
            tuning: Preset::Hybrid.tuning(),
            preset: Some(Preset::Hybrid),
            graph: None,
            spawn: Spawn::At(0, 0),
            respawn: true,
            #[cfg(feature = "toolkit")]
            scheme: None,
        }
    }

    /// Read game-toolkit's input with [`Platformer::controls`] and advance by `dt` seconds
    /// of frame time in `terrain`. Returns the fixed steps taken.
    #[cfg(feature = "toolkit")]
    pub fn update(
        &mut self,
        input: &game_toolkit::prelude::Input,
        terrain: &mut dyn Terrain,
        dt: f32,
    ) -> u32 {
        let intent = self.controls.read(input);
        self.advance(intent, terrain, dt)
    }

    /// Advance by `dt` seconds of frame time with `intent` in `terrain`, in fixed steps of
    /// [`DT`].
    /// Presses are kept until the next step, so a frame without one doesn't lose them.
    /// Returns the steps taken.
    pub fn advance(&mut self, intent: Intent, terrain: &mut dyn Terrain, dt: f32) -> u32 {
        self.latch.jump_pressed |= intent.jump_pressed;
        self.latch.dash_pressed |= intent.dash_pressed;
        self.latch.dive_pressed |= intent.dive_pressed;
        self.acc += dt;
        let n = ((self.acc / DT) as u32).min(MAX_STEPS);
        self.acc = (self.acc - n as f32 * DT).clamp(0.0, DT);
        for _ in 0..n {
            let i = Intent {
                jump_pressed: self.latch.jump_pressed,
                dash_pressed: self.latch.dash_pressed,
                dive_pressed: self.latch.dive_pressed,
                ..intent
            };
            self.latch = Intent::default();
            self.step(&i, terrain);
        }
        n
    }

    /// Exactly one fixed step of [`DT`] with `intent` in `terrain` (for your own clock,
    /// replays, tests).
    pub fn step(&mut self, intent: &Intent, terrain: &mut dyn Terrain) {
        self.prev = self.world.player.exact_pos();
        self.world.step(terrain, intent);
        self.events.extend(self.world.events.iter().copied());
        self.driver.drive(&self.world, &mut self.animator.params);
        self.animator.update(DT);
        self.anim_events.extend(self.animator.drain_events());
    }

    /// The movement events since the last call: jumps, landings, dashes, rolls, ...
    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// The animation events since the last call: states entered, markers (`"step"`),
    /// clips finished.
    pub fn drain_anim_events(&mut self) -> Vec<AnimEvent> {
        std::mem::take(&mut self.anim_events)
    }

    /// The sprite number to draw (see `hero::sprite`).
    pub fn sprite(&self) -> u16 {
        self.animator.sprite()
    }

    /// The animation state's name.
    pub fn state(&self) -> &str {
        self.animator.state_name()
    }

    /// 1 facing right, -1 facing left.
    pub fn facing(&self) -> i32 {
        self.world.player.facing
    }

    /// The hitbox's top-left in units, smoothly between fixed steps: where to draw.
    pub fn position(&self) -> (f32, f32) {
        let (x, y) = self.world.player.exact_pos();
        let (px, py) = self.prev;
        if (x - px).abs() > 16.0 || (y - py).abs() > 16.0 {
            return (x, y);
        }
        let a = (self.acc / DT).clamp(0.0, 1.0);
        (px + (x - px) * a, py + (y - py) * a)
    }

    /// The hitbox in units, as of the last step.
    pub fn hitbox(&self) -> Rect {
        self.world.player.rect()
    }

    /// Set the speed from outside: a spring, a bounce pad, knockback (units/s, y down). It
    /// ends a dash or a climb, can't be cut by releasing jump, and going up leaves the
    /// ground. See [`Player::launch`](crate::sim::Player::launch).
    pub fn launch(&mut self, vx: f32, vy: f32) {
        self.world.player.launch(vx, vy);
    }

    /// Put the hitbox's top-left at `x, y` (units) at once, without drawing it sliding
    /// there: a door, a portal. Speed and state stay; see
    /// [`Player::teleport`](crate::sim::Player::teleport).
    pub fn teleport(&mut self, x: i32, y: i32) {
        self.world.player.teleport(x, y);
        self.prev = self.world.player.exact_pos();
    }

    /// Start over at the spawn point: a fresh body, the animation from its entry state, and
    /// an [`Event::Respawned`]. For hazards; see [`Platformer::set_spawn`] for checkpoints.
    pub fn respawn(&mut self) {
        self.respawn_at(self.world.spawn.0, self.world.spawn.1);
    }

    /// Start over with the hitbox's top-left at `x, y` (units), like
    /// [`Platformer::respawn`], without moving the spawn point.
    pub fn respawn_at(&mut self, x: i32, y: i32) {
        self.world.respawn_at((x, y));
        self.events.push(Event::Respawned);
        self.prev = self.world.player.exact_pos();
        self.animator.reset();
    }

    /// The hitbox's top-left (units) for this character standing on `floor_y` (the top of
    /// the ground), centered on `x`: for [`Platformer::set_spawn`],
    /// [`Platformer::teleport`] and [`Platformer::respawn_at`] on a terrain of your own.
    pub fn standing_at(&self, x: i32, floor_y: i32) -> (i32, i32) {
        Player::standing_at(x, floor_y)
    }

    /// Where [`Platformer::respawn`] (and falling out) puts the character from now on
    /// (hitbox top-left, units): a checkpoint.
    pub fn set_spawn(&mut self, x: i32, y: i32) {
        self.world.spawn = (x, y);
    }

    /// Refill dashes, air jumps and climbing stamina: a pickup, a checkpoint.
    pub fn refill(&mut self) {
        self.world.player.refill(&self.world.tuning);
    }

    /// The preset the tuning came from, if any.
    pub fn preset(&self) -> Option<Preset> {
        self.preset
    }

    /// Switch to `preset`'s feel (and, with game-toolkit, its controls), keeping the
    /// character where it is.
    pub fn set_preset(&mut self, preset: Preset) {
        self.preset = Some(preset);
        self.world.tuning = preset.tuning();
        #[cfg(feature = "toolkit")]
        {
            self.controls = Controls::for_preset(preset);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::{Clip, Playback, StateSpec};

    #[test]
    fn a_frame_without_a_step_keeps_the_press() {
        // A fresh clock: no leftover time, so a zero-length frame takes no step.
        let mut level = Level::demo();
        let mut hero = Platformer::on(&level, Preset::Precise);
        let press = Intent {
            jump_pressed: true,
            jump_held: true,
            ..Intent::default()
        };
        assert_eq!(
            hero.advance(press, &mut level, 0.0),
            0,
            "a frame without a step"
        );
        let held = Intent {
            jump_held: true,
            ..Intent::default()
        };
        hero.advance(held, &mut level, 1.0 / 60.0);
        assert!(
            hero.drain_events()
                .iter()
                .any(|e| matches!(e, Event::Jumped { .. }))
        );
        assert_eq!(hero.state(), "takeoff");
    }

    #[test]
    fn tune_and_presets_set_the_feel() {
        let hero = Platformer::builder()
            .preset(Preset::Momentum)
            .tune(|t| t.air_jumps = 2)
            .build()
            .unwrap();
        assert_eq!(hero.world.tuning.air_jumps, 2);
        assert_eq!(
            hero.world.tuning.sprint_speed,
            Preset::Momentum.tuning().sprint_speed
        );
        assert_eq!(hero.preset(), Some(Preset::Momentum));
    }

    #[test]
    fn a_custom_graph_missing_a_driven_parameter_is_an_error() {
        let tiny = GraphSpec::new().state(StateSpec::new(
            "idle",
            Clip::uniform(Playback::Loop, &[0], 0.1),
        ));
        let err = Platformer::builder().graph(tiny).build().err();
        assert!(matches!(err, Some(SpecError::UnknownParam(_))), "{err:?}");
    }

    /// A flat floor at y = 100, wide open above.
    fn floor(r: Rect) -> bool {
        r.bottom() > 100
    }

    /// One frame of 1/60 s without input on the flat floor.
    fn idle(hero: &mut Platformer) -> Vec<Event> {
        hero.advance(Intent::default(), &mut floor, 1.0 / 60.0);
        hero.drain_events()
    }

    #[test]
    fn a_launch_is_not_cut_and_leaves_the_ground() {
        let mut hero = Platformer::at(20, 88, Preset::Precise);
        idle(&mut hero);
        assert!(hero.world.player.grounded);
        hero.launch(0.0, -300.0);
        // Jump isn't held: a jump would be cut now, a spring isn't.
        let events = idle(&mut hero);
        assert!(!events.contains(&Event::JumpCut), "{events:?}");
        assert!(!hero.world.player.grounded);
        assert!(hero.world.player.vy < -250.0, "{}", hero.world.player.vy);
        assert!(hero.hitbox().y < 88);
    }

    #[test]
    fn a_launch_ends_a_dash() {
        let mut hero = Platformer::at(20, 88, Preset::Precise);
        idle(&mut hero);
        let dash = Intent {
            move_x: 1.0,
            dash_pressed: true,
            ..Intent::default()
        };
        hero.advance(dash, &mut floor, 1.0 / 60.0);
        assert!(matches!(
            hero.world.player.mode,
            crate::sim::Mode::Dash { .. }
        ));
        hero.launch(-100.0, -200.0);
        assert_eq!(hero.world.player.mode, crate::sim::Mode::Normal);
        idle(&mut hero);
        assert!(hero.world.player.vx < 0.0, "knocked back");
    }

    #[test]
    fn a_teleport_snaps_without_smearing() {
        let mut hero = Platformer::at(20, 88, Preset::Precise);
        // Half a step of leftover time, so drawing would blend between two positions.
        hero.advance(Intent::default(), &mut floor, 1.5 * DT);
        hero.teleport(150, 40);
        assert_eq!(hero.position(), (150.0, 40.0));
        assert_eq!(hero.hitbox().x, 150);
        // It falls from there to the floor and lands.
        let mut events = Vec::new();
        for _ in 0..60 {
            events.extend(idle(&mut hero));
        }
        assert!(hero.world.player.grounded);
        assert_eq!(hero.hitbox().bottom(), 100);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::Landed { drop, .. } if *drop >= 40)),
            "{events:?}"
        );
    }

    #[test]
    fn respawning_resets_body_and_animation_and_says_so() {
        let mut hero = Platformer::at(20, 88, Preset::Precise);
        let run = Intent {
            move_x: 1.0,
            ..Intent::default()
        };
        for _ in 0..20 {
            hero.advance(run, &mut floor, 1.0 / 60.0);
        }
        hero.world.player.dashes = 0;
        hero.drain_events();
        assert_ne!(hero.state(), "idle");

        hero.set_spawn(60, 88);
        hero.respawn();
        assert_eq!(hero.drain_events(), vec![Event::Respawned]);
        assert_eq!(hero.hitbox().x, 60);
        assert_eq!(hero.position(), (60.0, 88.0));
        assert_eq!(hero.world.player.vx, 0.0);
        assert_eq!(hero.world.player.dashes, hero.world.tuning.dashes);
        assert_eq!(hero.state(), "idle");

        hero.respawn_at(100, 88);
        assert_eq!(hero.hitbox().x, 100);
        assert_eq!(
            hero.world.spawn,
            (60, 88),
            "respawn_at doesn't move the spawn"
        );
    }

    #[test]
    fn refill_restores_dashes_air_jumps_and_stamina() {
        let mut hero = Platformer::at(20, 88, Preset::Snappy);
        let p = &mut hero.world.player;
        (p.dashes, p.air_jumps, p.stamina) = (0, 0, 0.0);
        hero.refill();
        let (p, t) = (&hero.world.player, &hero.world.tuning);
        assert_eq!(
            (p.dashes, p.air_jumps, p.stamina),
            (t.dashes, t.air_jumps, t.stamina)
        );
    }

    #[test]
    fn spawning_standing_on_a_custom_floor_or_a_level_tile() {
        let mut hero = Platformer::builder()
            .spawn_standing(50, 100)
            .build()
            .unwrap();
        let r = hero.hitbox();
        assert_eq!((r.bottom(), r.x + r.w / 2), (100, 50));
        let events = idle(&mut hero);
        assert!(hero.world.player.grounded, "stands at once");
        assert!(events.is_empty(), "no fall, no landing: {events:?}");
        assert_eq!(hero.standing_at(50, 100), (r.x, r.y));

        // On a level, the same as the level's own spawn point.
        let level = Level::demo();
        let hero = Platformer::on(&level, Preset::Hybrid);
        let (x, y) = level.spawn_point(crate::sim::player::W, crate::sim::player::H);
        assert_eq!((hero.hitbox().x, hero.hitbox().y), (x, y));
    }

    #[test]
    fn a_closure_is_enough_terrain_and_falling_out_respawns_or_not() {
        // A floor at y = 100 from x = 0 to 200; beyond it, a drop into nothing.
        let mut floor = |r: Rect| r.bottom() > 100 && r.x < 200;
        let mut hero = Platformer::at(20, 88, Preset::Hybrid);
        for _ in 0..30 {
            hero.advance(Intent::default(), &mut floor, 1.0 / 60.0);
        }
        assert!(hero.world.player.grounded, "stands on the closure's floor");
        assert_eq!(hero.state(), "idle");

        // A terrain that also says where the world ends.
        struct Pit;
        impl Terrain for Pit {
            fn solid(&self, r: Rect) -> bool {
                r.bottom() > 100 && r.x < 200
            }
            fn fell_out(&self, r: Rect) -> bool {
                r.y > 300
            }
        }
        let mut hero = Platformer::builder()
            .spawn_at(220, 50)
            .respawn(false)
            .build()
            .unwrap();
        let mut events = Vec::new();
        for _ in 0..180 {
            hero.advance(Intent::default(), &mut Pit, 1.0 / 60.0);
            events.extend(hero.drain_events());
        }
        assert!(events.contains(&Event::FellOut));
        assert!(!events.contains(&Event::Respawned), "respawning was off");
    }
}
