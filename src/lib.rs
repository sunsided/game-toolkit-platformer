//! A pluggable platforming controller for [game-toolkit](https://github.com/sunsided/game-toolkit):
//! fluid, snappy movement with feel presets, and an animation state graph that follows it.
//!
//! The quickest start is [`Platformer`]: pick a preset, call `update` each frame with your
//! world as a [`sim::Terrain`] (anything that can say what's solid; [`sim::Level`] is a
//! ready-made one), and draw `sprite()` at `position()` (or `draw_debug` until there is
//! art). Underneath, three
//! layers you can also use on their own:
//!
//! - [`sim`]: the movement. A fixed-step, deterministic body: an [`sim::Intent`] in per
//!   step, [`sim::Event`]s out. Every feel number and ability switch is in [`sim::Tuning`];
//!   [`sim::Preset`]s set them toward the feel of Celeste, Hollow Knight, Silksong or
//!   Super Meat Boy (an independent project, not affiliated with those games' makers).
//! - [`anim`]: the animation controller, independent of any game: parameters, clips,
//!   conditions, a graph spec compiled into a validated graph, the animator, drivers.
//! - [`hero`]: the binding between them for one character: the built-in graph, its sprite
//!   numbers, and the driver that turns movement into graph parameters.
//!
//! With the `toolkit` feature (on by default), [`controls`] reads game-toolkit's keyboard
//! and gamepad through per-preset control schemes. Without it, nothing here depends on
//! anything but std.
//!
//! A complete game with a playable character:
//!
//! ```no_run
//! # #[cfg(feature = "toolkit")] {
//! use game_toolkit::prelude::*;
//! use game_toolkit_platformer::prelude::*;
//!
//! struct MyGame {
//!     level: Level,
//!     hero: Platformer,
//! }
//!
//! impl Game for MyGame {
//!     fn init(_ctx: &mut Context) -> Result<Self> {
//!         let level = Level::demo();
//!         let hero = Platformer::on(&level, Preset::Precise);
//!         Ok(Self { level, hero })
//!     }
//!
//!     fn update(&mut self, ctx: &mut Context, dt: f32) {
//!         self.hero.update(&ctx.input, &mut self.level, dt);
//!         for event in self.hero.drain_events() {
//!             match event {
//!                 Event::Jumped { .. } => { /* a sound */ }
//!                 Event::Landed { drop, .. } if drop > 48 => { /* a thud, some dust */ }
//!                 _ => {}
//!             }
//!         }
//!     }
//!
//!     fn render(&mut self, ctx: &mut Context, frame: &mut Frame) {
//!         let mut p = frame.painter(&mut ctx.gfx);
//!         p.clear([0.05, 0.04, 0.12, 1.0]);
//!         self.level.draw_debug(&mut p, [0.0, 0.0], 3.0);
//!         self.hero.draw_debug(&mut p, [0.0, 0.0], 3.0);
//!     }
//! }
//!
//! run::<MyGame>(AppConfig::default()).unwrap();
//! # }
//! ```
//!
//! Configured through the builder:
//!
//! ```
//! # #[cfg(feature = "toolkit")] {
//! use game_toolkit_platformer::prelude::*;
//!
//! let hero = Platformer::builder()
//!     .preset(Preset::Snappy)
//!     .tune(|t| {
//!         t.air_jumps = 2;
//!         t.dash_speed = 260.0;
//!     })
//!     .controls(Scheme::PRECISE)
//!     .spawn_at(40, 160)
//!     .respawn(false)
//!     .build()
//!     .unwrap();
//! assert_eq!(hero.world.tuning.air_jumps, 2);
//! # }
//! ```

pub mod anim;
#[cfg(feature = "toolkit")]
pub mod controls;
#[cfg(feature = "toolkit")]
mod debug_draw;
pub mod hero;
pub mod platformer;
pub mod sim;

pub use platformer::{Builder, Platformer};

/// Everything a game needs to get started: `use game_toolkit_platformer::prelude::*;`.
pub mod prelude {
    pub use crate::anim::{AnimEvent, GraphSpec};
    #[cfg(feature = "toolkit")]
    pub use crate::controls::{Controls, Scheme};
    pub use crate::platformer::{Builder, Platformer};
    pub use crate::sim::{Event, Intent, JumpKind, Level, Preset, Rect, Terrain, Tile, Tuning};
}

/// The README's code samples, compiled as doc tests.
#[cfg(all(doctest, feature = "toolkit"))]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
