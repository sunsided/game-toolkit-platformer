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

mod scaled;

use std::sync::Arc;

#[cfg(test)]
use crate::anim::Params;
use crate::anim::{AnimEvent, Animator, Driver, GraphSpec, SpecError};
#[cfg(feature = "toolkit")]
use crate::controls::{Controls, Scheme};
use crate::hero::{HeroDriver, spec};
use crate::sim::{
    Ability, DT, Event, Intent, Level, Mode, Player, Preset, Rect, TILE, Terrain, Tuning, World,
};

pub(crate) use scaled::Scaled;

/// At most this many fixed steps per frame, so a hitch doesn't spiral.
const MAX_STEPS: u32 = 8;

/// Add `dt` seconds of frame time to the clock `acc` and take the fixed steps due: how
/// many (at most [`MAX_STEPS`]).
pub(crate) fn steps(acc: &mut f32, dt: f32) -> u32 {
    *acc += dt;
    let n = ((*acc / DT) as u32).min(MAX_STEPS);
    *acc = (*acc - n as f32 * DT).clamp(0.0, DT);
    n
}

/// One playable character. The world around it is yours: pass your [`Terrain`] (or a
/// [`Level`]) to each update. For spikes, springs, checkpoints and pickups, test
/// [`Platformer::hitbox`] after the update and answer with [`Platformer::respawn`],
/// [`Platformer::launch`], [`Platformer::set_spawn`] or [`Platformer::refill`]
/// (`docs/hazards.md` has the recipe).
pub struct Platformer {
    /// The simulation: level, body, tuning. Read it for anything the accessors don't cover.
    pub world: World,
    /// The animation graph's runtime.
    pub animator: Animator,
    /// Movement facts and events to graph parameters (unless turned off).
    driver: Option<HeroDriver>,
    /// Your own drivers, run after the built-in one each step.
    drivers: Vec<Box<dyn Driver<World>>>,
    /// The preset the tuning came from, if any.
    preset: Option<Preset>,
    /// Pixels of your terrain per unit of the movement (see [`Builder::scale`]).
    scale: i32,
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
    /// The hitbox's top-left, in the terrain's pixels.
    At(i32, i32),
    /// Standing on `floor`, centered on `x`, in the terrain's pixels.
    Standing {
        /// The feet's center.
        x: i32,
        /// The top of the ground.
        floor: i32,
    },
    /// Standing on `floor`, centered on `x`, in units (a [`Level`]'s spawn tile).
    Units {
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
    /// The graph may leave out the built-in driver's parameters.
    partial: bool,
    /// Run the built-in driver.
    hero_driver: bool,
    /// Your own drivers, run after the built-in one.
    drivers: Vec<Box<dyn Driver<World>>>,
    /// Moves of your own.
    abilities: Vec<Box<dyn Ability>>,
    /// Where to start.
    spawn: Spawn,
    /// Put it back at the start after falling out.
    respawn: bool,
    /// Pixels of the terrain per unit.
    scale: i32,
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
    /// of them; [`Builder::build`] says which one is missing (a typo, say). For a graph
    /// that declares only what it uses, see [`Builder::partial_graph`].
    pub fn graph(mut self, graph: GraphSpec) -> Self {
        self.graph = Some(graph);
        self.partial = false;
        self
    }

    /// An animation graph of your own that declares only the built-in driver's
    /// parameters it uses: the rest are written but never read. A misspelled one goes
    /// unnoticed, which [`Builder::graph`] would catch.
    pub fn partial_graph(mut self, graph: GraphSpec) -> Self {
        self.graph = Some(graph);
        self.partial = true;
        self
    }

    /// A driver of your own, run after the built-in one every step (several run in the
    /// order added): for parameters of your game (`hurt`, `attacking`, `carrying`), or to
    /// override what the built-in one wrote. A closure `|world: &World, params: &mut
    /// Params| ...` is a driver. Parameters only your game sets (not the built-in driver)
    /// can also be set between updates through [`Platformer::animator`].
    pub fn driver(mut self, driver: impl Driver<World> + 'static) -> Self {
        self.drivers.push(Box::new(driver));
        self
    }

    /// A move of your own (an attack lunge, a ledge grab), run in every step; several run
    /// in the order added. See [`Ability`].
    pub fn ability(mut self, ability: impl Ability + Clone + 'static) -> Self {
        self.abilities.push(Box::new(ability));
        self
    }

    /// Run without the built-in driver: your drivers ([`Builder::driver`]) set every
    /// parameter, and the graph needn't declare the built-in ones.
    pub fn without_hero_driver(mut self) -> Self {
        self.hero_driver = false;
        self
    }

    /// Your terrain speaks pixels, `k` of them per unit of the movement (default 1): with
    /// 16-pixel tiles, say, `scale(2)`, so the feel made for 8-unit tiles fits. The
    /// character then converts at its edges: the terrain is asked in your pixels, and
    /// positions ([`Platformer::position`], [`Platformer::hitbox`], spawn points,
    /// teleports) and speeds ([`Platformer::velocity`], [`Platformer::launch`]) are in
    /// them too. [`Platformer::world`], the [`Tuning`], events and drivers stay in units.
    ///
    /// A [`Level`] already speaks units: keep the scale at 1 with one and draw it scaled.
    pub fn scale(mut self, k: i32) -> Self {
        assert!(k >= 1, "a scale of {k} pixels per unit");
        self.scale = k;
        self
    }

    /// Start with the hitbox's top-left at `x, y` (units, or pixels with a
    /// [`Builder::scale`]).
    pub fn spawn_at(mut self, x: i32, y: i32) -> Self {
        self.spawn = Spawn::At(x, y);
        self
    }

    /// Start standing on the ground at height `floor_y` (its top edge), centered on `x`
    /// (units, or pixels with a [`Builder::scale`]): the way to spawn on a [`Terrain`] of
    /// your own.
    pub fn spawn_standing(mut self, x: i32, floor_y: i32) -> Self {
        self.spawn = Spawn::Standing { x, floor: floor_y };
        self
    }

    /// Start standing on `level`'s spawn tile.
    pub fn spawn_on(mut self, level: &Level) -> Self {
        let (tx, ty) = level.spawn;
        self.spawn = Spawn::Units {
            x: tx * TILE + TILE / 2,
            floor: (ty + 1) * TILE,
        };
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
        let mut animator = Animator::new(Arc::new(graph));
        let driver = match (self.hero_driver, self.partial) {
            (false, _) => None,
            (true, false) => Some(HeroDriver::try_new(&animator.params)?),
            (true, true) => Some(HeroDriver::declaring(&mut animator.params)),
        };
        let k = self.scale;
        let spawn = match self.spawn {
            Spawn::At(x, y) => (x.div_euclid(k), y.div_euclid(k)),
            Spawn::Standing { x, floor } => {
                Player::standing_at(x.div_euclid(k), floor.div_euclid(k), self.tuning.size)
            }
            Spawn::Units { x, floor } => Player::standing_at(x, floor, self.tuning.size),
        };
        let mut world = World::new(self.tuning, spawn);
        world.respawn = self.respawn;
        world.abilities = self.abilities;
        Ok(Platformer {
            prev: world.player.exact_pos(),
            world,
            animator,
            driver,
            drivers: self.drivers,
            preset: self.preset,
            scale: k,
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
    /// [`Platformer::on`]. On a terrain of your own, [`Platformer::standing`] works out
    /// the top-left from where the feet go.
    pub fn at(x: i32, y: i32, preset: Preset) -> Self {
        Self::builder()
            .preset(preset)
            .spawn_at(x, y)
            .build()
            .expect("the built-in animation graph fits the built-in driver")
    }

    /// A character standing on the ground at height `floor_y` (its top edge), centered on
    /// `x` (units), otherwise like [`Platformer::on`]: the one-liner for a terrain of your
    /// own (see [`Builder::spawn_standing`]).
    pub fn standing(x: i32, floor_y: i32, preset: Preset) -> Self {
        Self::builder()
            .preset(preset)
            .spawn_standing(x, floor_y)
            .build()
            .expect("the built-in animation graph fits the built-in driver")
    }

    /// Set one up step by step; starts from the hybrid preset at `0, 0`.
    pub fn builder() -> Builder {
        Builder {
            tuning: Preset::Hybrid.tuning(),
            preset: Some(Preset::Hybrid),
            graph: None,
            partial: false,
            hero_driver: true,
            drivers: Vec::new(),
            abilities: Vec::new(),
            spawn: Spawn::At(0, 0),
            respawn: true,
            scale: 1,
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
        self.hold_presses(&intent);
        let n = steps(&mut self.acc, dt);
        for _ in 0..n {
            let i = self.with_presses(intent);
            self.step(&i, terrain);
        }
        n
    }

    /// Keep `intent`'s presses until the next step.
    pub(crate) fn hold_presses(&mut self, intent: &Intent) {
        self.latch.jump_pressed |= intent.jump_pressed;
        self.latch.dash_pressed |= intent.dash_pressed;
        self.latch.dive_pressed |= intent.dive_pressed;
        self.latch.custom_pressed |= intent.custom_pressed;
    }

    /// `intent` with the presses kept since the last step, which are then used up.
    pub(crate) fn with_presses(&mut self, intent: Intent) -> Intent {
        let i = Intent {
            jump_pressed: self.latch.jump_pressed,
            dash_pressed: self.latch.dash_pressed,
            dive_pressed: self.latch.dive_pressed,
            custom_pressed: self.latch.custom_pressed,
            ..intent
        };
        self.latch = Intent::default();
        i
    }

    /// Exactly one fixed step of [`DT`] with `intent` in `terrain` (for your own clock,
    /// replays, tests).
    pub fn step(&mut self, intent: &Intent, terrain: &mut dyn Terrain) {
        self.begin_step();
        if self.scale == 1 {
            self.world.step(terrain, intent);
        } else {
            let mut scaled = Scaled {
                inner: terrain,
                k: self.scale,
            };
            self.world.step(&mut scaled, intent);
        }
        self.end_step();
    }

    /// Pixels of the terrain per unit of the movement (see [`Builder::scale`]).
    pub fn scale(&self) -> i32 {
        self.scale
    }

    /// Set the clock's leftover time (a [`crate::Party`] keeps one clock for everyone), so
    /// [`Platformer::position`] draws between steps.
    pub(crate) fn set_clock(&mut self, acc: f32) {
        self.acc = acc;
    }

    /// Before a step: remember where the body was, to draw between steps.
    pub(crate) fn begin_step(&mut self) {
        self.prev = self.world.player.exact_pos();
    }

    /// After a step: collect its events, drive the animation. A respawn in the step (the
    /// character fell out) starts the animation over and snaps the drawing, as
    /// [`Platformer::respawn`] does.
    pub(crate) fn end_step(&mut self) {
        self.events.extend(self.world.events.iter().copied());
        if self.world.events.contains(&Event::Respawned) {
            self.prev = self.world.player.exact_pos();
            self.animator.reset();
            return;
        }
        if let Some(d) = &mut self.driver {
            d.drive(&self.world, &mut self.animator.params);
        }
        for d in &mut self.drivers {
            d.drive(&self.world, &mut self.animator.params);
        }
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

    /// Standing on something (the ground, a platform, moving ground, another character).
    pub fn grounded(&self) -> bool {
        self.world.player.grounded
    }

    /// The speed in units/s (pixels/s with a [`Builder::scale`]), y down.
    pub fn velocity(&self) -> (f32, f32) {
        let k = self.scale as f32;
        (self.world.player.vx * k, self.world.player.vy * k)
    }

    /// What the body is busy with: running and jumping freely, dashing, climbing, a move
    /// of your own, ...
    pub fn mode(&self) -> Mode {
        self.world.player.mode
    }

    /// Dashes left before touching the ground (or a refill) again.
    pub fn dashes(&self) -> u8 {
        self.world.player.dashes
    }

    /// Air jumps left.
    pub fn air_jumps(&self) -> u8 {
        self.world.player.air_jumps
    }

    /// Climbing stamina left (out of [`Tuning::stamina`]).
    pub fn stamina(&self) -> f32 {
        self.world.player.stamina
    }

    /// The side of the wall it holds on to or slides down (-1 left, 1 right), if any.
    pub fn on_wall(&self) -> Option<i32> {
        let p = &self.world.player;
        match (p.mode, p.wall_slide) {
            (Mode::Climb, _) => Some(p.facing),
            (_, 0) => None,
            (_, side) => Some(side),
        }
    }

    /// The hitbox's top-left in units (pixels with a [`Builder::scale`]), smoothly between
    /// fixed steps: where to draw.
    pub fn position(&self) -> (f32, f32) {
        self.position_at(self.acc)
    }

    /// [`Platformer::position`] with `acc` seconds into the next step.
    pub(crate) fn position_at(&self, acc: f32) -> (f32, f32) {
        let (x, y) = self.world.player.exact_pos();
        let (px, py) = self.prev;
        let k = self.scale as f32;
        if (x - px).abs() > 16.0 || (y - py).abs() > 16.0 {
            return (x * k, y * k);
        }
        let a = (acc / DT).clamp(0.0, 1.0);
        ((px + (x - px) * a) * k, (py + (y - py) * a) * k)
    }

    /// The hitbox in units (pixels with a [`Builder::scale`]), as of the last step.
    pub fn hitbox(&self) -> Rect {
        self.world.player.rect().scaled(self.scale)
    }

    /// A position in the terrain's pixels, in units.
    fn units(&self, (x, y): (i32, i32)) -> (i32, i32) {
        (x.div_euclid(self.scale), y.div_euclid(self.scale))
    }

    /// Set the speed from outside: a spring, a bounce pad, knockback (units/s, y down). It
    /// ends a dash or a climb, can't be cut by releasing jump, and going up leaves the
    /// ground. See [`Player::launch`](crate::sim::Player::launch).
    pub fn launch(&mut self, vx: f32, vy: f32) {
        let k = self.scale as f32;
        self.world.player.launch(vx / k, vy / k);
    }

    /// Put the hitbox's top-left at `x, y` (units) at once, without drawing it sliding
    /// there: a door, a portal. Speed and state stay; see
    /// [`Player::teleport`](crate::sim::Player::teleport).
    pub fn teleport(&mut self, x: i32, y: i32) {
        let (x, y) = self.units((x, y));
        self.world.player.teleport(x, y);
        self.prev = self.world.player.exact_pos();
    }

    /// Start over at the spawn point: a fresh body, the animation from its entry state, and
    /// an [`Event::Respawned`]. For hazards; see [`Platformer::set_spawn`] for checkpoints.
    pub fn respawn(&mut self) {
        self.respawn_in_units(self.world.spawn);
    }

    /// Start over with the hitbox's top-left at `x, y` (units), like
    /// [`Platformer::respawn`], without moving the spawn point.
    pub fn respawn_at(&mut self, x: i32, y: i32) {
        self.respawn_in_units(self.units((x, y)));
    }

    /// [`Platformer::respawn_at`] with the hitbox's top-left `at` in units.
    fn respawn_in_units(&mut self, at: (i32, i32)) {
        self.world.respawn_at(at);
        self.events.push(Event::Respawned);
        self.prev = self.world.player.exact_pos();
        self.animator.reset();
    }

    /// The hitbox's top-left (units) for this character standing on `floor_y` (the top of
    /// the ground), centered on `x`: for [`Platformer::set_spawn`],
    /// [`Platformer::teleport`] and [`Platformer::respawn_at`] on a terrain of your own.
    pub fn standing_at(&self, x: i32, floor_y: i32) -> (i32, i32) {
        let (x, floor_y) = self.units((x, floor_y));
        let (x, y) = Player::standing_at(x, floor_y, self.world.tuning.size);
        (x * self.scale, y * self.scale)
    }

    /// Where [`Platformer::respawn`] (and falling out) puts the character from now on
    /// (hitbox top-left, units): a checkpoint.
    pub fn set_spawn(&mut self, x: i32, y: i32) {
        self.world.spawn = self.units((x, y));
    }

    /// Refill dashes, air jumps and climbing stamina: a pickup, a checkpoint.
    pub fn refill(&mut self) {
        self.world.player.refill(&self.world.tuning);
    }

    /// The preset the tuning came from, if any.
    pub fn preset(&self) -> Option<Preset> {
        self.preset
    }

    /// Switch to `preset`'s feel (and, with game-toolkit, its controls, keeping your custom
    /// buttons), keeping the character where it is and its size.
    pub fn set_preset(&mut self, preset: Preset) {
        self.preset = Some(preset);
        self.world.tuning = Tuning {
            size: self.world.tuning.size,
            ..preset.tuning()
        };
        #[cfg(feature = "toolkit")]
        {
            // The game's own buttons stay bound.
            let custom = self.controls.scheme.custom;
            self.controls = Controls::for_preset(preset);
            self.controls.scheme.custom = custom;
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
    fn custom_presses_are_latched_like_the_built_in_ones() {
        /// The game's attack button.
        const ATTACK: u8 = 3;
        let mut level = Level::demo();
        let mut hero = Platformer::on(&level, Preset::Precise);
        let press = Intent {
            custom: 1 << ATTACK,
            custom_pressed: 1 << ATTACK,
            ..Intent::default()
        };
        assert_eq!(hero.advance(press, &mut level, 0.0), 0);
        assert!(hero.latch.pressed(ATTACK), "kept for the next step");
        hero.advance(Intent::default(), &mut level, 1.0 / 60.0);
        assert!(!hero.latch.pressed(ATTACK), "used up by the step");
        assert!(press.held(ATTACK) && !press.held(ATTACK + 1));
    }

    #[cfg(feature = "toolkit")]
    #[test]
    fn switching_presets_keeps_the_custom_buttons() {
        use crate::controls::Custom;
        use game_toolkit::prelude::Key;
        let attack = &[Custom {
            bit: 0,
            keys: &[Key::KeyX],
            pad: &[],
        }];
        let mut hero = Platformer::builder()
            .controls(Scheme {
                custom: attack,
                ..Scheme::SNAPPY
            })
            .build()
            .unwrap();
        hero.set_preset(Preset::Precise);
        assert_eq!(hero.controls.scheme.jump, Scheme::PRECISE.jump);
        assert_eq!(hero.controls.scheme.custom.len(), 1);
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

    /// Idle while grounded, air otherwise, by the `grounded` bool alone.
    fn two_state_graph() -> GraphSpec {
        use crate::anim::{Cond, TransitionSpec};
        let c = |s: u16| Clip::uniform(Playback::Loop, &[s], 0.1);
        GraphSpec::new()
            .bool("grounded")
            .state(StateSpec::new("idle", c(0)))
            .state(StateSpec::new("air", c(1)))
            .transition(TransitionSpec::new(
                ["idle"],
                "air",
                Cond::not(Cond::Is("grounded".into())),
            ))
            .transition(TransitionSpec::new(
                ["air"],
                "idle",
                Cond::Is("grounded".into()),
            ))
    }

    #[test]
    fn a_partial_graph_gets_only_what_it_declares() {
        assert!(
            Platformer::builder()
                .graph(two_state_graph())
                .build()
                .is_err(),
            "strict by default"
        );
        let mut hero = Platformer::builder()
            .partial_graph(two_state_graph())
            .spawn_at(20, 40)
            .build()
            .unwrap();
        idle(&mut hero);
        assert_eq!(hero.state(), "air");
        for _ in 0..60 {
            idle(&mut hero);
        }
        assert_eq!(hero.state(), "idle");
        // Starting over clears the parameters the driver added too.
        hero.respawn();
        idle(&mut hero);
    }

    #[test]
    fn own_drivers_run_after_the_built_in_one() {
        let hero = |b: Builder| {
            let mut h = b
                .graph(spec().bool("fast"))
                .driver(|w: &World, p: &mut Params| {
                    let id = p.id("fast").unwrap();
                    p.set_bool(id, w.player.vx.abs() > 50.0);
                })
                .spawn_at(20, 88)
                .build()
                .unwrap();
            for _ in 0..30 {
                h.advance(
                    Intent {
                        move_x: 1.0,
                        ..Intent::default()
                    },
                    &mut floor,
                    1.0 / 60.0,
                );
            }
            h
        };
        let h = hero(Platformer::builder());
        let fast = h.animator.params.id("fast").unwrap();
        assert!(h.animator.params.get_bool(fast), "set by the game's driver");

        // Without the built-in driver, nothing else is written.
        let h = hero(Platformer::builder().without_hero_driver());
        let grounded = h.animator.params.id("grounded").unwrap();
        assert!(!h.animator.params.get_bool(grounded));
        assert!(h.animator.params.get_bool(fast));

        // Later drivers win.
        let mut h = Platformer::builder()
            .driver(|_: &World, p: &mut Params| {
                let id = p.id("grounded").unwrap();
                p.set_bool(id, false);
            })
            .spawn_at(20, 88)
            .build()
            .unwrap();
        idle(&mut h);
        assert!(h.world.player.grounded);
        assert!(!h.animator.params.get_bool(grounded));
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
    fn the_hitbox_size_comes_from_the_tuning() {
        let mut level = Level::demo();
        let mut hero = Platformer::builder()
            .tune(|t| t.size = (10, 20))
            .spawn_on(&level)
            .build()
            .unwrap();
        let r = hero.hitbox();
        assert_eq!((r.w, r.h), (10, 20));
        let (sx, sy) = level.spawn;
        assert_eq!(r.bottom(), (sy + 1) * TILE, "stands on the spawn tile");
        assert_eq!(r.x + r.w / 2, sx * TILE + TILE / 2, "centered on it");
        for _ in 0..30 {
            hero.advance(Intent::default(), &mut level, 1.0 / 60.0);
        }
        assert!(hero.world.player.grounded);
        assert_eq!(hero.hitbox().bottom(), (sy + 1) * TILE);

        hero.set_preset(Preset::Momentum);
        hero.respawn();
        assert_eq!(hero.hitbox().h, 20, "a preset is a feel, not a body");
    }

    #[test]
    fn abilities_from_the_builder_run_with_custom_buttons() {
        use crate::sim::{Mode, Step};
        /// Custom button 0 stops the character dead, in a mode of its own for 0.1 s.
        #[derive(Clone, Debug)]
        struct Brake;
        impl Ability for Brake {
            fn before(&mut self, s: &mut Step<'_>) {
                match s.player.mode {
                    Mode::Normal if s.intent.pressed(0) => {
                        s.player.mode = Mode::Custom { id: 9, t: 0.0 };
                        s.player.vx = 0.0;
                        s.events.push(Event::Custom(9));
                    }
                    Mode::Custom { id: 9, t } if t >= 0.1 => s.player.mode = Mode::Normal,
                    _ => {}
                }
            }
        }
        let mut hero = Platformer::builder()
            .ability(Brake)
            .spawn_at(20, 88)
            .build()
            .unwrap();
        let run = Intent {
            move_x: 1.0,
            ..Intent::default()
        };
        for _ in 0..20 {
            hero.advance(run, &mut floor, 1.0 / 60.0);
        }
        hero.drain_events();
        // The press lands in a frame without a step and is still seen.
        let brake = Intent {
            custom_pressed: 1,
            ..run
        };
        hero.advance(brake, &mut floor, 0.0);
        hero.advance(run, &mut floor, DT);
        assert!(hero.drain_events().contains(&Event::Custom(9)));
        assert_eq!(hero.world.player.vx, 0.0, "the run doesn't accelerate it");
        for _ in 0..10 {
            hero.advance(run, &mut floor, 1.0 / 60.0);
        }
        assert_eq!(hero.world.player.mode, Mode::Normal, "handed back");
        assert!(hero.world.player.vx > 0.0, "running again");
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
    #[test]
    fn standing_spawns_like_the_builder_route() {
        let mut hero = Platformer::standing(50, 100, Preset::Snappy);
        let built = Platformer::builder()
            .preset(Preset::Snappy)
            .spawn_standing(50, 100)
            .build()
            .unwrap();
        assert_eq!(hero.hitbox(), built.hitbox());
        assert_eq!(hero.preset(), Some(Preset::Snappy));
        let events = idle(&mut hero);
        assert!(hero.grounded(), "stands on the closure's floor at once");
        assert!(events.is_empty(), "{events:?}");
    }

    #[test]
    fn accessors_match_the_body() {
        let mut hero = Platformer::at(20, 88, Preset::Precise);
        let run = Intent {
            move_x: 1.0,
            ..Intent::default()
        };
        for _ in 0..10 {
            hero.advance(run, &mut floor, 1.0 / 60.0);
        }
        let p = &hero.world.player;
        assert_eq!(hero.grounded(), p.grounded);
        assert_eq!(hero.velocity(), (p.vx, p.vy));
        assert!(hero.velocity().0 > 0.0);
        assert_eq!(hero.mode(), p.mode);
        assert_eq!(hero.dashes(), p.dashes);
        assert_eq!(hero.air_jumps(), p.air_jumps);
        assert_eq!(hero.stamina(), p.stamina);
        assert_eq!(hero.on_wall(), None);

        // Against a wall at x = 60, grabbing it.
        let mut wall = |r: Rect| r.bottom() > 100 || r.right() > 60;
        let grab = Intent {
            move_x: 1.0,
            grab: true,
            ..Intent::default()
        };
        for _ in 0..30 {
            hero.advance(grab, &mut wall, 1.0 / 60.0);
        }
        assert_eq!(hero.mode(), Mode::Climb, "{:?}", hero.hitbox());
        assert_eq!(hero.on_wall(), Some(1));
        assert!(hero.stamina() < hero.world.tuning.stamina);
    }

    #[test]
    fn falling_out_starts_the_animation_over() {
        // A floor up to x = 200, the character running off its end; the spawn on it.
        struct Pit;
        impl Terrain for Pit {
            fn solid(&self, r: Rect) -> bool {
                r.bottom() > 100 && r.x < 200
            }
            fn fell_out(&self, r: Rect) -> bool {
                r.y > 300
            }
        }
        let mut hero = Platformer::at(180, 88, Preset::Hybrid);
        let run = Intent {
            move_x: 1.0,
            ..Intent::default()
        };
        let entry = {
            let mut h = Platformer::at(20, 88, Preset::Hybrid);
            h.respawn();
            h.state().to_owned()
        };
        let mut respawned = false;
        for _ in 0..600 {
            hero.step(&run, &mut Pit);
            if hero.drain_events().contains(&Event::Respawned) {
                respawned = true;
                break;
            }
            assert!(hero.world.tick < 590, "never fell out");
        }
        assert!(respawned);
        assert_eq!(
            hero.state(),
            entry,
            "from the entry state, as after respawn()"
        );
        assert_eq!(
            hero.position(),
            (180.0, 88.0),
            "drawn at the spawn, not sliding there"
        );
    }
    #[test]
    fn a_scaled_terrain_plays_out_like_one_in_units() {
        // The same room in units and in pixels at 2 per unit: a floor, a wall, a ledge.
        let units = |r: Rect| r.bottom() > 100 || r.right() > 200 || (r.x < 60 && r.bottom() > 76);
        let pixels =
            |r: Rect| r.bottom() > 200 || r.right() > 400 || (r.x < 120 && r.bottom() > 152);
        let mut a = Platformer::standing(100, 100, Preset::Snappy);
        let mut b = Platformer::builder()
            .preset(Preset::Snappy)
            .scale(2)
            .spawn_standing(200, 200)
            .build()
            .unwrap();
        assert_eq!(b.hitbox(), a.hitbox().scaled(2));
        let script = |k: u32| Intent {
            move_x: if k % 120 < 70 { 1.0 } else { -1.0 },
            jump_pressed: k.is_multiple_of(50),
            jump_held: k % 50 < 25,
            dash_pressed: k % 80 == 30,
            grab: k % 200 > 150,
            ..Intent::default()
        };
        for k in 0..600 {
            a.advance(script(k), &mut units.clone(), 1.0 / 60.0);
            b.advance(script(k), &mut pixels.clone(), 1.0 / 60.0);
            assert_eq!(b.hitbox(), a.hitbox().scaled(2), "frame {k}");
            assert_eq!(a.drain_events(), b.drain_events(), "frame {k}");
            let ((ax, ay), (bx, by)) = (a.position(), b.position());
            assert!((bx - 2.0 * ax).abs() < 1e-3 && (by - 2.0 * ay).abs() < 1e-3);
            let ((avx, avy), (bvx, bvy)) = (a.velocity(), b.velocity());
            assert!((bvx - 2.0 * avx).abs() < 1e-3 && (bvy - 2.0 * avy).abs() < 1e-3);
        }

        // In and out of the facade: pixels.
        b.teleport(240, 100);
        assert_eq!((b.hitbox().x, b.hitbox().y), (240, 100));
        assert_eq!((b.world.player.x, b.world.player.y), (120, 50));
        b.launch(-200.0, 0.0);
        assert_eq!(b.velocity(), (-200.0, 0.0));
        assert_eq!(b.world.player.vx, -100.0);
        let (sx, sy) = b.standing_at(200, 200);
        let (ux, uy) = a.standing_at(100, 100);
        assert_eq!((sx, sy), (ux * 2, uy * 2));
        b.set_spawn(sx, sy);
        b.respawn();
        assert_eq!((b.hitbox().x, b.hitbox().y), (sx, sy));
    }
}
