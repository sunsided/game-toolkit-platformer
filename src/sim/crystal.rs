//! [`Crystal`]: a dash refill floating in the air; used up on touch, back after a while.

use super::{Rect, TILE, Tuning};

/// A dash crystal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crystal {
    /// The area that collects it (its tile).
    pub area: Rect,
    /// Seconds until it is back; 0 while it is there.
    pub cooldown: f32,
}

impl Crystal {
    /// A crystal filling tile `tx, ty`.
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

    /// Whether the crystal is there to be taken.
    pub fn ready(&self) -> bool {
        self.cooldown <= 0.0
    }

    /// Advance its clock by `dt`; if it is there, touches `body` and the hero `needs` a
    /// dash, use it up and return true. A hero with full dashes passes through without
    /// wasting it.
    pub fn collect(&mut self, body: Rect, dt: f32, needs: bool, t: &Tuning) -> bool {
        self.cooldown = (self.cooldown - dt).max(0.0);
        let touches = body.x < self.area.right()
            && self.area.x < body.right()
            && body.y < self.area.bottom()
            && self.area.y < body.bottom();
        if self.ready() && touches && needs {
            self.cooldown = t.crystal_respawn;
            return true;
        }
        false
    }
}
