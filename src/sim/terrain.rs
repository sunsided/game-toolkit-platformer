//! [`Terrain`]: what the movement asks about the world around the character. Your game
//! implements it for its own level (a tilemap, a collision grid, anything); only
//! [`Terrain::solid`] is required.
//!
//! Positions and sizes are in the controller's units: the body is 6x12, the tuning is made
//! for 8-unit tiles. If your tiles are 16 pixels, divide by two on the way in (or use
//! [`super::Level`], which already speaks units).
//!
//! To start the character standing on your ground, give the builder the feet's position:
//! `Platformer::builder().spawn_standing(x, floor_y)` (or `Player::standing_at` for the
//! hitbox's top-left).
//!
//! The smallest terrain is a closure:
//!
//! ```
//! use game_toolkit_platformer::sim::{Rect, Terrain};
//!
//! // Solid below y = 100, nothing else.
//! let mut floor = |r: Rect| r.bottom() > 100;
//! assert!(floor.solid(Rect { x: 0, y: 95, w: 6, h: 12 }));
//! ```

use super::Rect;

/// The world around the character, as the movement sees it.
pub trait Terrain {
    /// Whether `r` overlaps anything solid (walls, floors, ceilings, boxes).
    fn solid(&self, r: Rect) -> bool;

    /// Whether `r` stands exactly on top of a one-way platform (its bottom edge on the
    /// platform's top). One-way platforms hold from above, let the character jump up
    /// through them, and drop it on down + jump. Default: there are none.
    fn platform(&self, _r: Rect) -> bool {
        false
    }

    /// The character's body `r` touches a dash refill: use it up and return `true` (it's
    /// only asked when a dash is spent). Default: there are none.
    fn refill(&mut self, _r: Rect) -> bool {
        false
    }

    /// A pushable box at rest that overlaps `r` (the character's side, a unit or two
    /// out), by an id of your choosing. Default: there are none.
    fn pushable(&self, _r: Rect) -> Option<usize> {
        None
    }

    /// Move box `id` sideways by `dx` units (fractions accumulate) and return the whole
    /// units it moved; `hero` is the character's body, which it mustn't move into.
    fn shove(&mut self, _id: usize, _dx: f32, _hero: Rect) -> i32 {
        0
    }

    /// Whether `r` is out of the world (fell into a pit). Default: never.
    fn fell_out(&self, _r: Rect) -> bool {
        false
    }

    /// Once per step after the character moved, with its body `hero`: let things fall,
    /// refills come back, and so on. Default: nothing.
    fn update(&mut self, _dt: f32, _hero: Rect) {}
}

/// Any `Fn(Rect) -> bool` is a terrain of solid ground.
impl<F: Fn(Rect) -> bool> Terrain for F {
    fn solid(&self, r: Rect) -> bool {
        self(r)
    }
}
