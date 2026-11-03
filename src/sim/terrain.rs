//! [`Terrain`]: what the movement asks about the world around the character. Your game
//! implements it for its own level (a tilemap, a collision grid, anything); only
//! [`Terrain::solid`] is required.
//!
//! Positions and sizes are in the controller's units: the body is 6x12 unless
//! [`super::Tuning::size`] says otherwise, the tuning is made for 8-unit tiles. If your
//! tiles are 16 pixels, give the [`crate::Builder`] a scale of 2
//! ([`crate::Builder::scale`]) and answer in your pixels, or use [`super::Level`], which
//! already speaks units.
//!
//! Hazards, pickups and checkpoints aren't the terrain's business: test the character's
//! hitbox against them after each update (`docs/hazards.md` has the recipe).
//!
//! To start the character standing on your ground, give it the feet's position:
//! `Platformer::standing(x, floor_y, preset)` (or `Builder::spawn_standing`, or
//! `Player::standing_at` for the hitbox's top-left).
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
//!
//! # Building on another terrain
//!
//! To add state of your own to a terrain (doors, switches, moving platforms, boxes),
//! wrap it and name it as the [`Terrain::base`]: every method you don't write forwards to
//! it, those added later included. Write only what changes:
//!
//! ```
//! use game_toolkit_platformer::sim::{Level, Rect, Terrain};
//!
//! /// The demo level with a door that can be shut.
//! struct Stage {
//!     level: Level,
//!     door: Rect,
//!     shut: bool,
//! }
//!
//! impl Terrain for Stage {
//!     fn solid(&self, r: Rect) -> bool {
//!         self.level.solid(r) || (self.shut && self.door.intersects(r))
//!     }
//!     // Platforms, refills, falling out, the update: the level's.
//!     fn base(&self) -> Option<&dyn Terrain> {
//!         Some(&self.level)
//!     }
//!     fn base_mut(&mut self) -> Option<&mut dyn Terrain> {
//!         Some(&mut self.level)
//!     }
//! }
//! # let s = Stage { level: Level::demo(), door: Rect { x: 0, y: 0, w: 8, h: 8 }, shut: true };
//! # assert!(s.solid(Rect { x: 1, y: 1, w: 2, h: 2 }) && !s.fell_out(Rect { x: 9, y: 9, w: 2, h: 2 }));
//! ```

use std::any::Any;

use super::Rect;

/// The world around the character, as the movement sees it.
///
/// Every method but [`Terrain::solid`] has a default: forward to [`Terrain::base`] if
/// there is one, otherwise there is nothing of the kind (no platforms, no refills, ...).
pub trait Terrain {
    /// Whether `r` overlaps anything solid (walls, floors, ceilings, moving solids).
    fn solid(&self, r: Rect) -> bool;

    /// Whether `r` stands exactly on top of a one-way platform (its bottom edge on the
    /// platform's top). One-way platforms hold from above, let the character jump up
    /// through them, and drop it on down + jump.
    fn platform(&self, r: Rect) -> bool {
        self.base().is_some_and(|b| b.platform(r))
    }

    /// The character's body `r` touches a dash refill: use it up and return `true` (it's
    /// only asked when a dash is spent).
    fn refill(&mut self, r: Rect) -> bool {
        self.base_mut().is_some_and(|b| b.refill(r))
    }

    /// Whether `r` is out of the world (fell into a pit).
    fn fell_out(&self, r: Rect) -> bool {
        self.base().is_some_and(|b| b.fell_out(r))
    }

    /// Once per step after the characters moved, with their `bodies` (one, unless several
    /// share the terrain through a [`crate::Party`]): let things fall onto them, refills
    /// come back, platforms move, and so on. Writing it yourself on a terrain with a
    /// [`Terrain::base`], update the base too, or its refills never come back.
    fn update(&mut self, dt: f32, bodies: &[Rect]) {
        if let Some(b) = self.base_mut() {
            b.update(dt, bodies);
        }
    }

    /// How far the solid (or one-way platform) overlapping `r` moved in the last
    /// [`Terrain::update`], in units: moving platforms, elevators.
    ///
    /// Asked right after `update` with what the character rode before it: the row just
    /// under its feet while standing, the column beside it while holding on to a wall.
    /// The world then carries it along by as much, stopping at walls and ceilings, and a
    /// jump off keeps that speed ([`super::Tuning::lift_boost`]). Moving solids don't push
    /// the character otherwise: keep them from moving into it from the side or above
    /// (`docs/moving-platforms.md` has a whole one).
    fn carry(&self, r: Rect) -> (f32, f32) {
        self.base().map_or((0.0, 0.0), |b| b.carry(r))
    }

    /// The terrain this one builds on, which the defaults forward to. Default: none.
    fn base(&self) -> Option<&dyn Terrain> {
        None
    }

    /// [`Terrain::base`], to change. Return the same terrain from both.
    fn base_mut(&mut self) -> Option<&mut dyn Terrain> {
        None
    }

    /// This terrain as itself, so an [`Ability`](super::Ability) can reach your type
    /// through [`find_mut`](trait.Terrain.html#method.find_mut) (break a block, flip a
    /// switch): write `Some(self)`. Default: `None`, it isn't found (its
    /// [`Terrain::base`] still is).
    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        None
    }
}

impl dyn Terrain + '_ {
    /// The terrain of type `T` in this one: itself, or the first of its bases (see
    /// [`Terrain::base`]) that is a `T` and says so in [`Terrain::as_any_mut`].
    ///
    /// ```
    /// use game_toolkit_platformer::sim::{Level, Terrain, Tile};
    ///
    /// let mut level = Level::parse(&["....", "####"]);
    /// let terrain: &mut dyn Terrain = &mut level;
    /// terrain.find_mut::<Level>().unwrap().set(1, 1, Tile::Empty);
    /// assert_eq!(level.tile(1, 1), Tile::Empty);
    /// ```
    pub fn find_mut<T: 'static>(&mut self) -> Option<&mut T> {
        if self.as_any_mut().is_some_and(|a| a.is::<T>()) {
            return self.as_any_mut()?.downcast_mut();
        }
        self.base_mut()?.find_mut()
    }
}

/// Any `Fn(Rect) -> bool` is a terrain of solid ground.
impl<F: Fn(Rect) -> bool> Terrain for F {
    fn solid(&self, r: Rect) -> bool {
        self(r)
    }
}
