//! [`Moves`]: which built-in moves may run in one step. An [`Ability`](super::Ability)
//! clears what its own move rules out in [`Ability::before`](super::Ability::before): no
//! dash while attacking, no wall grab while carrying, slow walking while pushing.

/// Which built-in moves may run this step, and how fast running may go. Every step starts
/// from [`Moves::ALL`]; abilities narrow it before the built-in moves run.
///
/// A cleared move doesn't start, and one in progress lets go where that makes sense (a
/// climb without `climb` lets go of the wall, a glide without `glide` folds). Presses
/// stay buffered as usual, so a jump pressed while `jump` was cleared can still fire once
/// it is back within the buffer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Moves {
    /// Every jump: from the ground, coyote, roll, wall, climb, in the air; and dropping
    /// through one-way platforms.
    pub jump: bool,
    /// Starting a dash.
    pub dash: bool,
    /// Starting a dive.
    pub dive: bool,
    /// Grabbing and holding on to walls.
    pub climb: bool,
    /// Charging a super dash.
    pub super_dash: bool,
    /// Gliding.
    pub glide: bool,
    /// The most running (and walking) may reach on its own, units/s: `0.0` keeps the body
    /// from accelerating at all. Speed from elsewhere (a launch, a dash's end) still
    /// brakes down to it at the tuning's rate.
    pub max_run: f32,
}

impl Moves {
    /// Everything allowed, running at the tuning's full speed.
    pub const ALL: Moves = Moves {
        jump: true,
        dash: true,
        dive: true,
        climb: true,
        super_dash: true,
        glide: true,
        max_run: f32::INFINITY,
    };
}

impl Default for Moves {
    fn default() -> Self {
        Self::ALL
    }
}
