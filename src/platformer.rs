//! [`Platformer`]: one playable character, plug and play: movement, its animation, and (with
//! the `toolkit` feature) the controls.
//!
//! ```
//! use game_toolkit_platformer::prelude::*;
//!
//! let mut hero = Platformer::builder(Level::demo())
//!     .preset(Preset::Snappy)
//!     .tune(|t| t.jump_speed = 115.0)
//!     .build()
//!     .unwrap();
//!
//! // Each frame: an intent (or `hero.update(&ctx.input, dt)` with game-toolkit), then
//! // whatever happened, and what to draw.
//! hero.advance(Intent { move_x: 1.0, ..Intent::default() }, 1.0 / 60.0);
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
use crate::sim::{DT, Event, Intent, Level, Preset, Rect, Tuning, World};

/// At most this many fixed steps per frame, so a hitch doesn't spiral.
const MAX_STEPS: u32 = 8;

/// One playable character on a level.
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

/// Sets up a [`Platformer`]: preset, tuning, controls, animation graph, spawn point.
pub struct Builder {
    /// The level.
    level: Level,
    /// The movement feel.
    tuning: Tuning,
    /// The preset it came from.
    preset: Option<Preset>,
    /// A custom animation graph, or the built-in one.
    graph: Option<GraphSpec>,
    /// Where to start instead of the level's spawn tile (hitbox top-left, units).
    spawn: Option<(i32, i32)>,
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

    /// Start with the hitbox's top-left at `x, y` (units) instead of the spawn tile.
    pub fn spawn_at(mut self, x: i32, y: i32) -> Self {
        self.spawn = Some((x, y));
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
        let mut world = World::new(self.level, self.tuning);
        if let Some((x, y)) = self.spawn {
            world.player.x = x;
            world.player.y = y;
            world.player.peak_y = y;
        }
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
    /// A character on `level` with `preset`'s feel and controls and the built-in animation.
    pub fn new(level: Level, preset: Preset) -> Self {
        Self::builder(level)
            .preset(preset)
            .build()
            .expect("the built-in animation graph fits the built-in driver")
    }

    /// Set one up step by step; starts from the hybrid preset.
    pub fn builder(level: Level) -> Builder {
        Builder {
            level,
            tuning: Preset::Hybrid.tuning(),
            preset: Some(Preset::Hybrid),
            graph: None,
            spawn: None,
            #[cfg(feature = "toolkit")]
            scheme: None,
        }
    }

    /// Read game-toolkit's input with [`Platformer::controls`] and advance by `dt` seconds
    /// of frame time. Returns the fixed steps taken.
    #[cfg(feature = "toolkit")]
    pub fn update(&mut self, input: &game_toolkit::prelude::Input, dt: f32) -> u32 {
        let intent = self.controls.read(input);
        self.advance(intent, dt)
    }

    /// Advance by `dt` seconds of frame time with `intent`, in fixed steps of [`DT`].
    /// Presses are kept until the next step, so a frame without one doesn't lose them.
    /// Returns the steps taken.
    pub fn advance(&mut self, intent: Intent, dt: f32) -> u32 {
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
            self.step(&i);
        }
        n
    }

    /// Exactly one fixed step of [`DT`] with `intent` (for your own clock, replays, tests).
    pub fn step(&mut self, intent: &Intent) {
        self.prev = self.world.player.exact_pos();
        self.world.step(intent);
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
        let mut hero = Platformer::new(Level::demo(), Preset::Precise);
        let press = Intent {
            jump_pressed: true,
            jump_held: true,
            ..Intent::default()
        };
        assert_eq!(hero.advance(press, 0.0), 0, "a frame without a step");
        hero.advance(
            Intent {
                jump_held: true,
                ..Intent::default()
            },
            1.0 / 60.0,
        );
        assert!(
            hero.drain_events()
                .iter()
                .any(|e| matches!(e, Event::Jumped { .. }))
        );
        assert_eq!(hero.state(), "takeoff");
    }

    #[test]
    fn tune_and_presets_set_the_feel() {
        let hero = Platformer::builder(Level::demo())
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
        let err = Platformer::builder(Level::demo()).graph(tiny).build().err();
        assert!(matches!(err, Some(SpecError::UnknownParam(_))), "{err:?}");
    }
}
