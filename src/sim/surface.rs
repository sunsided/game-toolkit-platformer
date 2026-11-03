//! [`Surface`]: what kind of ground or wall the character touches: ice, a conveyor belt, a
//! wall too smooth to climb, a bouncy pad. A [`Terrain`](super::Terrain) answers it in
//! [`Terrain::surface`](super::Terrain::surface); by default everything is
//! [`Surface::DEFAULT`] and the [`Tuning`](super::Tuning) alone sets the feel.
//!
//! `docs/surfaces.md` has the recipe.

/// The kind of a surface. Each field's default leaves the tuning's feel as it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    /// Scales running's acceleration, braking and turning on this ground: `1.0` as tuned,
    /// below for ice (`0.1` slides a long way), above for sticky ground.
    pub friction: f32,
    /// Whether the character may grab and climb this wall. Wall slides and wall jumps
    /// still work on a wall that isn't climbable.
    pub climbable: bool,
    /// A conveyor belt: carries what stands on it sideways at this speed, units/s
    /// (positive right). A jump off it keeps the speed, as off moving ground.
    pub belt: f32,
    /// Bounciness: a landing faster than [`Surface::BOUNCE_MIN`] bounces back up at this
    /// share of its downward speed (`0.0`: none, `1.0`: as fast as it came down).
    pub bounce: f32,
}

impl Surface {
    /// Plain ground: the tuning's feel, climbable, still, not bouncy.
    pub const DEFAULT: Surface = Surface {
        friction: 1.0,
        climbable: true,
        belt: 0.0,
        bounce: 0.0,
    };

    /// The slowest landing (units/s down) a bouncy surface bounces back.
    pub const BOUNCE_MIN: f32 = 40.0;
}

impl Default for Surface {
    fn default() -> Self {
        Self::DEFAULT
    }
}
