//! [`Grip`]: how the hero holds a box, kept in the [`super::Stage`] so the renderer and
//! the animation can see it, and a cloned stage plays out the same.

use super::Shove;

/// The hero's hold on a box this step, and the strain that breaks it loose.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Grip {
    /// What she does with which box (by index), if anything.
    pub shove: Option<(Shove, usize)>,
    /// How far the box has given, 0 (stuck) to 1 (sliding freely); scales push and pull
    /// speed.
    pub give: f32,
    /// Seconds she has been pushing or pulling the same box the same way: boxes stick
    /// until [`super::PushPull::breakaway`], then speed up over [`super::PushPull::ramp`].
    pub effort: f32,
    /// Her `x` before the built-in moves ran this step, for pulling the box after her.
    pub from_x: i32,
}

impl Grip {
    /// The box she strains against but which hasn't budged yet, if any: it may rumble.
    pub fn straining(&self, breakaway: f32) -> Option<usize> {
        let (_, i) = self.shove?;
        let moving = matches!(
            self.shove,
            Some((Shove::Push { .. } | Shove::Pull { .. }, _))
        );
        (moving && self.effort > 0.0 && self.effort < breakaway).then_some(i)
    }
}
