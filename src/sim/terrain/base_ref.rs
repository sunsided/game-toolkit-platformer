//! [`BaseRef`]: what [`terrain_layer!`](crate::terrain_layer) calls on the field it wraps.

use super::Terrain;

/// A field a [`terrain_layer!`](crate::terrain_layer) names as its base, seen as a
/// [`Terrain`]: the terrain itself, or one behind a `&mut` or a `Box` (method calls see
/// through those). Not for your own use; the macro writes the calls.
#[doc(hidden)]
pub trait BaseRef {
    /// This base, to read.
    fn __terrain_base(&self) -> &dyn Terrain;
    /// This base, to change.
    fn __terrain_base_mut(&mut self) -> &mut dyn Terrain;
}

impl<T: Terrain> BaseRef for T {
    fn __terrain_base(&self) -> &dyn Terrain {
        self
    }
    fn __terrain_base_mut(&mut self) -> &mut dyn Terrain {
        self
    }
}

impl BaseRef for dyn Terrain + '_ {
    fn __terrain_base(&self) -> &dyn Terrain {
        self
    }
    fn __terrain_base_mut(&mut self) -> &mut dyn Terrain {
        self
    }
}
