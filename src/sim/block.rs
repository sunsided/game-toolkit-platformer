//! [`Block`]: a box the hero can push, pull and stand on. It falls, and collides with
//! the terrain, other boxes and the hero.
//!
//! [`super::Level`] keeps its boxes this way; a [`Terrain`] of your own can too. It keeps a
//! `Vec<Block>`, counts them as solid, answers [`Terrain::solid_except`] so a box doesn't
//! collide with itself, and forwards the box methods:
//!
//! ```
//! use game_toolkit_platformer::sim::{Block, Intent, Preset, Rect, Terrain, World};
//!
//! /// A flat floor at y = 100 with boxes on it.
//! struct Yard {
//!     boxes: Vec<Block>,
//! }
//!
//! impl Terrain for Yard {
//!     fn solid(&self, r: Rect) -> bool {
//!         self.solid_except(r, None)
//!     }
//!     fn solid_except(&self, r: Rect, skip: Option<usize>) -> bool {
//!         r.bottom() > 100 || Block::any_except(&self.boxes, r, skip)
//!     }
//!     fn pushable(&self, r: Rect) -> Option<usize> {
//!         Block::pushable(&self.boxes, r)
//!     }
//!     fn shove(&mut self, id: usize, dx: f32, hero: Rect) -> i32 {
//!         // A copy moves while the terrain still holds the old one, which it skips.
//!         let mut b = self.boxes[id];
//!         let moved = b.shove(self, id, hero, dx);
//!         self.boxes[id] = b;
//!         moved
//!     }
//!     fn update(&mut self, dt: f32, hero: Rect) {
//!         for i in 0..self.boxes.len() {
//!             let mut b = self.boxes[i];
//!             b.fall(self, i, hero, dt);
//!             self.boxes[i] = b;
//!         }
//!     }
//! }
//!
//! // A box standing on the floor at x = 40, the character left of it walking right.
//! let mut yard = Yard { boxes: vec![Block::standing(40, 100)] };
//! let mut world = World::new(Preset::Hybrid.tuning(), (20, 88));
//! let right = Intent { move_x: 1.0, ..Intent::default() };
//! for _ in 0..240 {
//!     world.step(&mut yard, &right);
//! }
//! assert!(yard.boxes[0].x > 40, "pushed");
//! assert_eq!(yard.boxes[0].rect().bottom(), 100, "still on the floor");
//! ```

use super::{Rect, TILE, Terrain};

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
        Self::standing(tx * TILE + (TILE - SIZE) / 2, (ty + 1) * TILE)
    }

    /// A block with its left edge at `x`, standing on the ground at `floor_y` (units).
    pub fn standing(x: i32, floor_y: i32) -> Self {
        Self {
            x,
            y: floor_y - SIZE,
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

    /// Whether `r` overlaps any of `blocks` other than number `skip`: the boxes' part of
    /// [`Terrain::solid_except`].
    pub fn any_except(blocks: &[Block], r: Rect, skip: Option<usize>) -> bool {
        blocks
            .iter()
            .enumerate()
            .any(|(i, b)| Some(i) != skip && b.rect().intersects(r))
    }

    /// The first of `blocks` at rest that overlaps `r`, by index: [`Terrain::pushable`]
    /// for a terrain that keeps its boxes in a list.
    pub fn pushable(blocks: &[Block], r: Rect) -> Option<usize> {
        blocks
            .iter()
            .position(|b| b.grounded && b.rect().intersects(r))
    }

    /// Whether block `me` would hit something at `r`: the terrain (other boxes included,
    /// itself not), the hero.
    fn blocked(terrain: &dyn Terrain, me: usize, hero: Rect, r: Rect) -> bool {
        terrain.solid_except(r, Some(me)) || r.intersects(hero)
    }

    /// Whether block `me` rests on something.
    fn rests(&self, terrain: &dyn Terrain, me: usize, hero: Rect) -> bool {
        let r = self.rect();
        Self::blocked(terrain, me, hero, r.shifted(0, 1)) || terrain.platform(r)
    }

    /// Slide sideways by `amount` units (fractions accumulate), stopping at anything
    /// solid in `terrain` other than itself (it is box `me` there) and at the hero's body
    /// `hero`. Returns the whole units moved.
    pub fn shove(&mut self, terrain: &dyn Terrain, me: usize, hero: Rect, amount: f32) -> i32 {
        self.rem_x += amount;
        let n = self.rem_x.round() as i32;
        self.rem_x -= n as f32;
        let s = n.signum();
        for k in 0..n.abs() {
            if Self::blocked(terrain, me, hero, self.rect().shifted(s, 0)) {
                self.rem_x = 0.0;
                return k * s;
            }
            self.x += s;
        }
        n
    }

    /// Gravity and falling for one step in `terrain` (where it is box `me`): lands on the
    /// ground, platforms, other boxes and the hero's body `hero`.
    pub fn fall(&mut self, terrain: &dyn Terrain, me: usize, hero: Rect, dt: f32) {
        if self.rests(terrain, me, hero) {
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
            if self.rests(terrain, me, hero) {
                self.vy = 0.0;
                self.rem_y = 0.0;
                self.grounded = true;
                return;
            }
            self.y += 1;
        }
    }
}
