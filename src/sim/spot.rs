//! [`Spot`]: where to put the character, by its hitbox's top-left or by its feet.
//!
//! Every placement takes one ([`World::new`](super::World::new), the `Platformer`'s spawn,
//! respawn, teleport and checkpoint), so feet and top-left can't be mixed up. Tiles are not
//! a spot: a [`Level`](super::Level) turns its spawn tile into one with
//! [`Level::start`](super::Level::start).
//!
//! A spot is in the terrain's space: units, or your pixels with a `Builder::scale`.
//!
//! ```
//! use game_toolkit_platformer::sim::Spot;
//!
//! // A 6x12 body standing on the ground at y = 100, centered on x = 50.
//! assert_eq!(Spot::Feet(50, 100).top_left((6, 12)), (47, 88));
//! assert_eq!(Spot::TopLeft(47, 88).top_left((6, 12)), (47, 88));
//! ```

/// A place for the character's body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spot {
    /// The hitbox's top-left corner at `x, y`.
    TopLeft(i32, i32),
    /// Standing on the ground at height `floor_y` (the ground's top edge), centered on `x`:
    /// `Feet(x, floor_y)`. The way to put it on a terrain of your own.
    Feet(i32, i32),
}

impl Spot {
    /// The hitbox's top-left for a body of `(w, h)` at this spot.
    pub fn top_left(self, (w, h): (i32, i32)) -> (i32, i32) {
        match self {
            Spot::TopLeft(x, y) => (x, y),
            Spot::Feet(x, floor_y) => (x - (w + 1) / 2, floor_y - h),
        }
    }

    /// This spot given in `k` pixels per unit, in units (rounding down).
    pub fn in_units(self, k: i32) -> Spot {
        let d = |v: i32| v.div_euclid(k);
        match self {
            Spot::TopLeft(x, y) => Spot::TopLeft(d(x), d(y)),
            Spot::Feet(x, y) => Spot::Feet(d(x), d(y)),
        }
    }

    /// This spot given in units, in `k` pixels per unit.
    pub fn in_pixels(self, k: i32) -> Spot {
        match self {
            Spot::TopLeft(x, y) => Spot::TopLeft(x * k, y * k),
            Spot::Feet(x, y) => Spot::Feet(x * k, y * k),
        }
    }
}
