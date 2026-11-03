//! A small retro platformer built around a pluggable, configurable animation controller.
//!
//! Everything except [`app`] is free of toolkit types, so it unit tests and renders
//! headlessly (see `examples/snapshot.rs`):
//!
//! - [`sim`]: the deterministic simulation: tiles, the hero's movement (run, jump with
//!   buffer, coyote time, hangtime and cut, double jump, wall slide/jump, climb, 8-way
//!   dash, dive, land-and-roll, drop-through platforms, dash crystals).
//! - [`anim`]: the animation controller, independent of the game: parameters, clips,
//!   conditions, a named graph spec compiled into a validated graph, the animator.
//! - [`hero`]: the hero's binding: its animation graph and the driver from the sim.
//! - [`session`]: a game in progress: fixed-step clock with slow motion, pause and
//!   stepping, input latching, world plus animator.
//! - [`gfx`]: the 384x216 pixel grid: [`gfx::Raster`], CPU [`gfx::Canvas`], GPU-bound
//!   [`gfx::DrawList`], palette and bitmap fonts.
//! - [`render`]: draws a session: the cached backdrop, the hero, the debug overlay.
//! - [`app`]: the game-toolkit glue: window, input, presenting.
//!
//! See `docs/` for the design.

pub mod anim;
pub mod app;
pub mod gfx;
pub mod hero;
pub mod render;
pub mod session;
pub mod sim;
