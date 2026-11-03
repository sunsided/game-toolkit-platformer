//! [`Crystal`]: a dash refill floating in the air; used up on touch, back after a while.

use super::{Rect, TILE};

/// A dash refill.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crystal {
    /// The area that collects it (its tile).
    pub area: Rect,
    /// Seconds until it is back; 0 while it is there.
    pub cooldown: f32,
}

impl Crystal {
    /// A refill filling tile `tx, ty`.
    pub fn at((tx, ty): (i32, i32)) -> Self {
        Self {
            area: Rect {
                x: tx * TILE,
                y: ty * TILE,
                w: TILE,
                h: TILE,
            },
            cooldown: 0.0,
        }
    }

    /// Whether it is there to be taken.
    pub fn ready(&self) -> bool {
        self.cooldown <= 0.0
    }
}
