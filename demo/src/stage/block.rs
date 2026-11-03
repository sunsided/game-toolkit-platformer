//! [`Block`]: a box the hero can push, pull and stand on. It falls, and stops at whatever
//! the [`super::Stage`] says is in the way (the level, other boxes, the characters).

use crate::sim::{Rect, TILE};

/// How fast a box falls, units/s².
const GRAVITY: f32 = 900.0;
/// The fastest a box falls, units/s.
const MAX_FALL: f32 = 160.0;

/// Block size in units (square).
pub const SIZE: i32 = 12;

/// A pushable box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    /// Left edge, units.
    pub x: i32,
    /// Top edge, units.
    pub y: i32,
    /// Sub-unit horizontal position not yet moved.
    rem_x: f32,
    /// Sub-unit vertical position not yet moved.
    rem_y: f32,
    /// Falling speed, units/s.
    pub vy: f32,
    /// Resting on something.
    pub grounded: bool,
}

impl Block {
    /// A block standing on the bottom of tile `tx, ty`, centered on it.
    pub fn at((tx, ty): (i32, i32)) -> Self {
        Self {
            x: tx * TILE + (TILE - SIZE) / 2,
            y: (ty + 1) * TILE - SIZE,
            rem_x: 0.0,
            rem_y: 0.0,
            vy: 0.0,
            grounded: true,
        }
    }

    /// Its box.
    pub fn rect(&self) -> Rect {
        Rect {
            x: self.x,
            y: self.y,
            w: SIZE,
            h: SIZE,
        }
    }

    /// Slide sideways by `amount` units (fractions accumulate), stopping where
    /// `blocked(r)` says its box `r` would hit something. Returns the whole units moved.
    pub fn shove(&mut self, amount: f32, blocked: impl Fn(Rect) -> bool) -> i32 {
        self.rem_x += amount;
        let n = self.rem_x.round() as i32;
        self.rem_x -= n as f32;
        let s = n.signum();
        for k in 0..n.abs() {
            if blocked(self.rect().shifted(s, 0)) {
                self.rem_x = 0.0;
                return k * s;
            }
            self.x += s;
        }
        n
    }

    /// Gravity and falling for one step of `dt`, landing where `rests(r)` says its box
    /// `r` has something under it.
    pub fn fall(&mut self, dt: f32, rests: impl Fn(Rect) -> bool) {
        if rests(self.rect()) {
            self.grounded = true;
            self.vy = 0.0;
            self.rem_y = 0.0;
            return;
        }
        self.grounded = false;
        self.vy = (self.vy + GRAVITY * dt).min(MAX_FALL);
        self.rem_y += self.vy * dt;
        let n = self.rem_y.round() as i32;
        self.rem_y -= n as f32;
        for _ in 0..n {
            if rests(self.rect()) {
                self.vy = 0.0;
                self.rem_y = 0.0;
                self.grounded = true;
                return;
            }
            self.y += 1;
        }
    }
}
