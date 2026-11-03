//! [`Block`]: a box the hero can push, pull and stand on. It falls, and collides with
//! tiles, other blocks and the hero.

use super::{Level, Rect, TILE};

/// How fast a box falls, units/s².
const GRAVITY: f32 = 900.0;
/// The fastest a box falls, units/s.
const MAX_FALL: f32 = 160.0;

/// Block size in pixels (square).
pub const SIZE: i32 = 12;

/// A pushable box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    /// Left edge, pixels.
    pub x: i32,
    /// Top edge, pixels.
    pub y: i32,
    /// Sub-pixel horizontal position not yet moved.
    rem_x: f32,
    /// Sub-pixel vertical position not yet moved.
    rem_y: f32,
    /// Falling speed, px/s.
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

    /// Whether block `me` would hit something at `r`: tiles, other blocks, the hero.
    fn blocked(level: &Level, me: usize, hero: Rect, r: Rect) -> bool {
        level.overlaps_except(r, Some(me)) || r.intersects(hero)
    }

    /// Whether block `me` rests on something.
    fn rests(&self, level: &Level, me: usize, hero: Rect) -> bool {
        let r = self.rect();
        Self::blocked(level, me, hero, r.shifted(0, 1)) || level.on_platform(r)
    }

    /// Slide sideways by `amount` pixels (sub-pixels accumulate), stopping at anything
    /// solid. Returns the whole pixels moved.
    pub fn shove(&mut self, level: &Level, me: usize, hero: Rect, amount: f32) -> i32 {
        self.rem_x += amount;
        let n = self.rem_x.round() as i32;
        self.rem_x -= n as f32;
        let s = n.signum();
        for k in 0..n.abs() {
            if Self::blocked(level, me, hero, self.rect().shifted(s, 0)) {
                self.rem_x = 0.0;
                return k * s;
            }
            self.x += s;
        }
        n
    }

    /// Gravity and falling for one step: lands on tiles, platforms, blocks and the hero.
    pub fn fall(&mut self, level: &Level, me: usize, hero: Rect, dt: f32) {
        if self.rests(level, me, hero) {
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
            if self.rests(level, me, hero) {
                self.vy = 0.0;
                self.rem_y = 0.0;
                self.grounded = true;
                return;
            }
            self.y += 1;
        }
    }
}
