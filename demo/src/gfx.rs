//! Pixel graphics on the 384x216 grid.
//!
//! There is no full-frame framebuffer. Everything is drawn through [`Raster`], a target that
//! can fill a horizontal run ("span") of grid pixels with a color and an opacity. Every
//! primitive here is written once against that trait and works on both targets:
//!
//! - [`Canvas`]: a CPU RGB image. The backdrop (sky and level tiles) is drawn into one and
//!   uploaded as a texture when it changes; the headless snapshots also draw into one.
//! - [`DrawList`]: records spans, merging neighbors on a row. The app turns each span into
//!   one GPU rect exactly one grid pixel tall, so moving things stay on the grid without a
//!   CPU redraw of the whole screen.
//!
//! Coordinates are grid pixels, `(0, 0)` top-left, y down. Everything clips.
//!
//! Submodules: [`palette`] (colors), [`font`] (two bitmap fonts) and [`png`] (a tiny PNG
//! encoder for snapshots).

pub mod canvas;
pub mod draw_list;
pub mod font;
pub mod palette;
pub mod png;
pub mod raster;

pub use canvas::Canvas;
pub use draw_list::{DrawList, Span};
pub use palette::Rgb;
pub use raster::{Raster, bayer};

/// Grid width in pixels: 48 tiles of 8 px, one pixel per simulation unit. 16:9, so
/// 1920x1080 is exactly 5x.
pub const W: i32 = 384;
/// Grid height in pixels: 27 tiles of 8 px.
pub const H: i32 = 216;
