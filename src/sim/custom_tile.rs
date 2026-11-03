//! [`CustomTile`]: what a [`Tile::Custom`](super::Tile::Custom) is to the movement, set
//! per id with [`Level::define`](super::Level::define).

use super::{Hold, Surface};

/// How the movement treats one kind of custom tile. The default is air.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CustomTile {
    /// Whether and how it holds the character.
    pub hold: Hold,
    /// The surface it is, when it holds the character (solid or a platform).
    pub surface: Surface,
}

impl CustomTile {
    /// A solid tile with `surface`.
    pub const fn solid(surface: Surface) -> Self {
        Self {
            hold: Hold::Solid,
            surface,
        }
    }

    /// A one-way platform with `surface`: a bouncy, icy or moving-belt ledge.
    pub const fn platform(surface: Surface) -> Self {
        Self {
            hold: Hold::Platform,
            surface,
        }
    }
}
