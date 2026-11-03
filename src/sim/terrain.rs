//! [`Terrain`]: what the movement asks about the world around the character. Your game
//! implements it for its own level (a tilemap, a collision grid, anything); only
//! [`Terrain::solid`] is required.
//!
//! Positions and sizes are in the controller's units: the body is 6x12 unless
//! [`super::Tuning::size`] says otherwise, the tuning is made
//! for 8-unit tiles. If your tiles are 16 pixels, divide by two on the way in (or use
//! [`super::Level`], which already speaks units).
//!
//! Hazards, pickups and checkpoints aren't the terrain's business: test the character's
//! hitbox against them after each update (`docs/hazards.md` has the recipe).
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

    /// Like [`Terrain::solid`], but not counting box `skip` (an id from
    /// [`Terrain::pushable`]): a [`Block`](super::Block) asks this while it moves, so it
    /// doesn't collide with itself. Default: [`Terrain::solid`], which is right for a
    /// terrain without boxes; one with boxes has to leave `skip` out, or its boxes can't
    /// move.
    fn solid_except(&self, r: Rect, _skip: Option<usize>) -> bool {
        self.solid(r)
    }

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

    /// A pushable box at rest that overlaps `r`, by an id of your choosing. Default:
    /// there are none.
    ///
    /// The contract (see [`super::block`] for a whole terrain with boxes):
    ///
    /// - `r` is the character's hitbox shifted one, then two units to the side it pushes
    ///   or pulls toward, so a box that just moved a unit away still counts. It is only
    ///   asked while the character stands on the ground.
    /// - Only boxes at rest count: a falling box isn't pushable.
    /// - Ids are yours, but keep them stable from step to step: the world compares them to
    ///   tell whether the character keeps straining against the same box, which is what
    ///   breaks it loose ([`super::Tuning::push_breakaway`]).
    fn pushable(&self, _r: Rect) -> Option<usize> {
        None
    }

    /// Move box `id` (from [`Terrain::pushable`]) sideways by `dx` units and return the
    /// whole units it moved, signed. Default: boxes don't move.
    ///
    /// - `dx` is a fraction of a unit per step at push speeds: keep the remainder and
    ///   move whole units when it adds up (as [`Block::shove`](super::Block::shove) does).
    /// - Stop at anything solid, other boxes included, and at `hero`, the character's
    ///   body, which the box must not move into.
    /// - Pushing, the world shoves the box first and then moves the character into the
    ///   room it made; pulling, it moves the character first and then shoves the box
    ///   after it by the whole units the character moved.
    fn shove(&mut self, _id: usize, _dx: f32, _hero: Rect) -> i32 {
        0
    }

    /// Whether `r` is out of the world (fell into a pit). Default: never.
    fn fell_out(&self, _r: Rect) -> bool {
        false
    }

    /// Once per step after the characters moved, with their `bodies` (one, unless several
    /// share the terrain through a [`crate::Party`]): let things fall onto them, refills
    /// come back, platforms move, and so on. Default: nothing.
    fn update(&mut self, _dt: f32, _bodies: &[Rect]) {}

    /// How far the solid (or one-way platform) overlapping `r` moved in the last
    /// [`Terrain::update`], in units: moving platforms, elevators. Default: nothing moves.
    ///
    /// Asked right after `update` with what the character rode before it: the row just
    /// under its feet while standing, the column beside it while holding on to a wall.
    /// The world then carries it along by as much, stopping at walls and ceilings, and a
    /// jump off keeps that speed ([`super::Tuning::lift_boost`]). Moving solids don't push
    /// the character otherwise: keep them from moving into it from the side or above.
    fn carry(&self, _r: Rect) -> (f32, f32) {
        (0.0, 0.0)
    }
}

/// Any `Fn(Rect) -> bool` is a terrain of solid ground.
impl<F: Fn(Rect) -> bool> Terrain for F {
    fn solid(&self, r: Rect) -> bool {
        self(r)
    }
}
