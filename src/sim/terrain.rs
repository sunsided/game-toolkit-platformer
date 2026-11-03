//! [`Terrain`]: what the movement asks about the world around the character. Your game
//! implements it for its own level (a tilemap, a collision grid, anything): write
//! [`Terrain::solid`] and say what it builds on with [`terrain_layer!`](crate::terrain_layer)
//! (`terrain_layer!()` for nothing, `terrain_layer!(level)` for a field it wraps).
//!
//! Positions and sizes are in the controller's units: the body is 6x12 unless
//! [`super::Tuning::size`] says otherwise, the tuning is made for 8-unit tiles. If your
//! tiles are 16 pixels, give the [`crate::Builder`] a scale of 2
//! ([`crate::Builder::scale`]) and answer in your pixels, or use [`super::Level`], which
//! already speaks units.
//!
//! Hazards, pickups and checkpoints aren't the terrain's business: test the character's
//! hitboxes against them after each update (`docs/hazards.md` has the recipe). Dash
//! refills are the one exception, because the movement decides when one is used (only
//! when a dash has been spent): [`Terrain::refill`].
//!
//! To start the character standing on your ground, give it the feet's position:
//! `Platformer::at(Spot::Feet(x, floor_y), preset)` (see [`super::Spot`]).
//!
//! The smallest terrain is a closure (it builds on nothing):
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
//! wrap it and name it as the [`Terrain::base`] with [`terrain_layer!`](crate::terrain_layer):
//! every method you don't write forwards to it, those added later included, and
//! [`find_mut`](trait.Terrain.html#method.find_mut) finds your type. Write only what
//! changes; your own state moves in [`Terrain::tick`], which runs on every layer by
//! itself, so the base's refills come back whatever you write:
//!
//! ```
//! use game_toolkit_platformer::sim::{Level, Rect, Terrain};
//!
//! /// The demo level with a door that opens and shuts every second.
//! struct Stage {
//!     level: Level,
//!     door: Rect,
//!     clock: f32,
//! }
//!
//! impl Terrain for Stage {
//!     fn solid(&self, r: Rect) -> bool {
//!         let shut = self.clock % 2.0 < 1.0;
//!         self.level.solid(r) || (shut && self.door.intersects(r))
//!     }
//!     fn tick(&mut self, dt: f32, _bodies: &[Rect]) {
//!         self.clock += dt; // the level's refills come back on their own
//!     }
//!     // Platforms, refills, surfaces, falling out: the level's.
//!     game_toolkit_platformer::terrain_layer!(level);
//! }
//! # let mut s = Stage { level: Level::demo(), door: Rect { x: 0, y: 0, w: 8, h: 8 }, clock: 0.0 };
//! # assert!(s.solid(Rect { x: 1, y: 1, w: 2, h: 2 }) && !s.fell_out(Rect { x: 9, y: 9, w: 2, h: 2 }));
//! # let t: &mut dyn Terrain = &mut s;
//! # assert!(t.find_mut::<Stage>().is_some() && t.find_mut::<Level>().is_some());
//! ```
//!
//! [`Terrain::base`] has no default, so a wrapper can't forget it: without it, the level's
//! platforms, refills, surfaces and pits would silently be gone. This doesn't compile:
//!
//! ```compile_fail,E0046
//! use game_toolkit_platformer::sim::{Level, Rect, Terrain};
//!
//! struct Doors { level: Level }
//! impl Terrain for Doors {
//!     fn solid(&self, r: Rect) -> bool { self.level.solid(r) }
//!     // missing: game_toolkit_platformer::terrain_layer!(level);
//! }
//! ```

use std::any::Any;

use super::{Rect, Surface};

/// The world around the character, as the movement sees it.
///
/// Write [`Terrain::solid`] and [`Terrain::base`] (with [`terrain_layer!`](crate::terrain_layer));
/// every other method has a default: forward to the base if there is one, otherwise there
/// is nothing of the kind (no platforms, no refills, ...).
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

    /// What kind of ground or wall `r` touches: the row under the feet while standing, the
    /// column beside the body at a wall. Ice, conveyor belts, walls that can't be climbed,
    /// bouncy pads (see [`Surface`]). Default: the base's, or [`Surface::DEFAULT`].
    fn surface(&self, r: Rect) -> Surface {
        self.base().map_or(Surface::DEFAULT, |b| b.surface(r))
    }

    /// This layer's own update, once per step after the characters moved, with their
    /// `bodies` (one, unless several share the terrain through a [`crate::Party`]): let
    /// things fall onto them, refills come back, platforms move. Only this layer's state:
    /// [`Terrain::update`] runs the bases' ticks by itself. Default: nothing.
    fn tick(&mut self, _dt: f32, _bodies: &[Rect]) {}

    /// The whole terrain's update: [`Terrain::tick`] on this layer, then on every base
    /// down the chain. Don't write it; write [`Terrain::tick`]. Only an adapter that hands
    /// its base something else than it got (converted bodies, say) writes it, and then
    /// calls the base's `update`.
    fn update(&mut self, dt: f32, bodies: &[Rect]) {
        debug_assert!(
            same_base(self),
            "Terrain::base and Terrain::base_mut must return the same terrain"
        );
        self.tick(dt, bodies);
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

    /// The terrain this one builds on, which the defaults forward to, or `None`.
    /// [`terrain_layer!`](crate::terrain_layer) writes it with [`Terrain::base_mut`] and
    /// [`Terrain::as_any_mut`]. Required, so a wrapper can't lose its base's platforms,
    /// refills and pits by forgetting it.
    fn base(&self) -> Option<&dyn Terrain>;

    /// [`Terrain::base`], to change. Return the same terrain from both (checked in debug
    /// builds).
    fn base_mut(&mut self) -> Option<&mut dyn Terrain> {
        None
    }

    /// This terrain as itself, so an [`Ability`](super::Ability) can reach your type
    /// through [`find_mut`](trait.Terrain.html#method.find_mut) (break a block, flip a
    /// switch): `Some(self)`, which [`terrain_layer!`](crate::terrain_layer) writes.
    /// Default: `None`, it isn't found (its [`Terrain::base`] still is).
    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        None
    }
}

/// Whether `t`'s [`Terrain::base`] and [`Terrain::base_mut`] are the same terrain.
fn same_base<T: Terrain + ?Sized>(t: &mut T) -> bool {
    let shared = t.base().map(|b| b as *const dyn Terrain as *const ());
    let unique = t.base_mut().map(|b| b as *const dyn Terrain as *const ());
    shared == unique
}

/// Writes the boilerplate of a [`Terrain`] layer inside its `impl Terrain` block:
/// [`Terrain::base`] and [`Terrain::base_mut`] returning the field named (a name or a
/// tuple index; left out: no base), and [`Terrain::as_any_mut`] returning the terrain
/// itself, so [`find_mut`](trait.Terrain.html#method.find_mut) finds it. A terrain that
/// borrows (has a lifetime) can't be found; it writes `base` by hand.
///
/// ```
/// use game_toolkit_platformer::sim::{Level, Rect, Terrain};
///
/// struct Doors { level: Level }
/// impl Terrain for Doors {
///     fn solid(&self, r: Rect) -> bool { self.level.solid(r) }
///     game_toolkit_platformer::terrain_layer!(level);
/// }
///
/// struct Room;
/// impl Terrain for Room {
///     fn solid(&self, r: Rect) -> bool { r.bottom() > 100 }
///     game_toolkit_platformer::terrain_layer!(); // no base; still found by find_mut
/// }
/// # let t: &mut dyn Terrain = &mut Room;
/// # assert!(t.find_mut::<Room>().is_some());
/// ```
#[macro_export]
macro_rules! terrain_layer {
    () => {
        fn base(&self) -> ::core::option::Option<&dyn $crate::sim::Terrain> {
            ::core::option::Option::None
        }
        $crate::terrain_layer!(@any);
    };
    (@any) => {
        fn as_any_mut(&mut self) -> ::core::option::Option<&mut dyn ::core::any::Any> {
            ::core::option::Option::Some(self)
        }
    };
    ($base:tt) => {
        fn base(&self) -> ::core::option::Option<&dyn $crate::sim::Terrain> {
            ::core::option::Option::Some(&self.$base)
        }
        fn base_mut(&mut self) -> ::core::option::Option<&mut dyn $crate::sim::Terrain> {
            ::core::option::Option::Some(&mut self.$base)
        }
        $crate::terrain_layer!(@any);
    };
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

/// Any `Fn(Rect) -> bool` is a terrain of solid ground, built on nothing.
impl<F: Fn(Rect) -> bool> Terrain for F {
    fn solid(&self, r: Rect) -> bool {
        self(r)
    }
    fn base(&self) -> Option<&dyn Terrain> {
        None
    }
}
