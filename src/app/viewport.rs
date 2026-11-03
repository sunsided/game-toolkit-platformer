//! [`Viewport`]: where the pixel grid sits in the window.

use crate::gfx::{H, W};

/// The grid scaled uniformly and centered, with black borders around it.
#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    /// Left edge of the grid in window pixels.
    pub x: f32,
    /// Top edge of the grid in window pixels.
    pub y: f32,
    /// Window pixels per grid pixel.
    pub scale: f32,
}

impl Viewport {
    /// The largest whole-number scale at which the grid fits a `win_w` x `win_h` window,
    /// centered. Below 1x there is no choice but a fractional scale.
    pub fn fit(win_w: u32, win_h: u32) -> Self {
        let s = (win_w as f32 / W as f32).min(win_h as f32 / H as f32);
        let scale = if s >= 1.0 { s.floor() } else { s };
        Self {
            x: ((win_w as f32 - W as f32 * scale) / 2.0).floor(),
            y: ((win_h as f32 - H as f32 * scale) / 2.0).floor(),
            scale,
        }
    }
}
