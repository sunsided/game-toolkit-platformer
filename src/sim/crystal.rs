//! [`Crystal`]: a dash refill floating in the air; used up on touch, back after a while.

use super::Rect;

/// A dash refill.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crystal {
    /// The area that collects it (its tile).
    pub area: Rect,
    /// Seconds until it is back; 0 while it is there.
    pub cooldown: f32,
}

impl Crystal {
    /// A refill filling tile `tx, ty` of `size` units (a [`super::Level`] makes them, with
    /// [`super::Level::add_refill`]).
    pub(crate) fn at((tx, ty): (i32, i32), size: i32) -> Self {
        Self {
            area: Rect {
                x: tx * size,
                y: ty * size,
                w: size,
                h: size,
            },
            cooldown: 0.0,
        }
    }

    /// Whether it is there to be taken.
    pub fn ready(&self) -> bool {
        self.cooldown <= 0.0
    }
}
