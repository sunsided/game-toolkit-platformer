//! The test bench for [`game_toolkit_platformer`]: the Cheesed heroine in a moonlit level.
//!
//! The movement, the animation controller and the hero's graph come from the library
//! (re-exported here as [`sim`], [`anim`] and [`hero`]); this crate adds what a game puts
//! around them:
//!
//! - [`session`]: a game in progress: the library's world and animator plus a fixed clock
//!   with slow motion, effects, wind, plants and the cape rig.
//! - [`gfx`]: the 384x216 pixel grid: [`gfx::Raster`], CPU [`gfx::Canvas`], GPU-bound
//!   [`gfx::DrawList`], palette and bitmap fonts.
//! - [`render`]: the night scene, the heroine as a cut-out puppet, the debug overlay.
//! - [`app`]: the game-toolkit glue: window, input, presenting.

pub use game_toolkit_platformer::{anim, sim};

/// The hero: the library's graph, driver and sprite numbers, plus the demo's look of them
/// (what each sprite means for the body, and the simulated cape, tail and mane).
pub mod hero {
    pub use game_toolkit_platformer::hero::*;

    pub mod pose;
    pub mod rig;
}

pub mod app;
pub mod gfx;
pub mod render;
pub mod session;
