//! [`Hold`]: how a [`CustomTile`](super::CustomTile) holds the character.

/// How a custom tile holds the character: not at all, from every side, or from above.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Hold {
    /// Air: the character passes through (spikes, water, a checkpoint).
    #[default]
    Air,
    /// Blocks from every side like [`Tile::Solid`](super::Tile::Solid).
    Solid,
    /// A one-way platform like [`Tile::Platform`](super::Tile::Platform): holds from
    /// above, lets the character jump up through it, drops it on down + jump.
    Platform,
}
