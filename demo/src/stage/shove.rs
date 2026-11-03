//! [`Shove`]: the hero's hold on a box in one step.

/// The hero's hold on a box, decided by [`super::PushPull`] each step before she moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shove {
    /// Grabbing a box on `side` without moving.
    Hold {
        /// Side of the box, -1 or 1.
        side: i32,
    },
    /// Walking into a box on `side`, pushing it.
    Push {
        /// Side of the box.
        side: i32,
    },
    /// Grabbing a box on `side` and walking away, pulling it.
    Pull {
        /// Side of the box.
        side: i32,
    },
}
