//! [`Event`]: what happened in a step, for animation, sound and effects.

use super::CustomId;

/// How a jump left the ground (or the wall, or the air).
/// New kinds may come with new moves, so matches need a `_` arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum JumpKind {
    /// From standing or running on the ground.
    Ground,
    /// Just after walking off a ledge, within coyote time.
    Coyote,
    /// Out of a landing roll, keeping the roll's speed.
    Roll,
    /// Off a wall, away from it.
    Wall,
    /// Straight up a wall while holding on, at a stamina cost.
    Climb,
    /// In the air, spending an air jump.
    Double,
}

/// Something that happened in one step. New moves bring new events, so matches need a
/// `_` arm.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// The hero jumped. `buffered`: the press came before the jump was possible and waited
    /// in the jump buffer.
    Jumped {
        /// Which kind of jump.
        kind: JumpKind,
        /// Whether the press was queued.
        buffered: bool,
    },
    /// The hero let go of jump while rising and the jump was cut short.
    JumpCut,
    /// Vertical speed crossed zero going up to down in the air.
    Apex,
    /// The hero walked or rolled off the ground (not by jumping).
    LeftGround,
    /// Down + jump on a one-way platform: dropping through it.
    DroppedThrough,
    /// The hero touched down.
    Landed {
        /// Downward speed at contact, px/s.
        impact: f32,
        /// Pixels fallen from the highest point of the airtime.
        drop: i32,
    },
    /// A landing turned into a roll.
    RollStarted,
    /// The roll ran out (not by jumping).
    RollEnded,
    /// A landing from high up without moving: a heavy landing that costs control.
    HardLanded,
    /// A heavy landing or a dive impact is over.
    Recovered,
    /// A dash started, in direction `dx, dy` (8-way, normalized, y down).
    Dashed {
        /// Horizontal direction.
        dx: f32,
        /// Vertical direction.
        dy: f32,
    },
    /// The dash ran out and handed over its leftover speed.
    DashEnded,
    /// Dashes were refilled, on the ground or by a crystal.
    Recharged {
        /// By a crystal (in the air), not by the ground.
        crystal: bool,
    },
    /// Grabbed a wall.
    Grabbed,
    /// Let go of a wall (released grab, wall ended, or out of stamina).
    LetGo,
    /// Climbed over the top of a wall with a small hop.
    ClimbHop,
    /// Stamina ran out on a wall.
    StaminaOut,
    /// A dive started: a short hang, then straight down.
    DiveStarted,
    /// The super dash finished charging: release to launch.
    SuperCharged,
    /// The super dash launched.
    SuperDashStarted {
        /// Direction, -1 or 1.
        dir: i32,
    },
    /// The super dash is over.
    SuperDashEnded {
        /// It hit a wall (otherwise it was cancelled).
        bonk: bool,
    },
    /// The cloak opened: gliding.
    GlideStarted,
    /// The glide is over (jump released, landed, a wall, a dash).
    GlideEnded,
    /// A dive hit the ground.
    DiveImpact {
        /// Pixels fallen from the dive's start.
        drop: i32,
    },
    /// The head hit a ceiling.
    Bonked,
    /// The head clipped a ceiling corner and was nudged past it.
    CornerCorrected {
        /// Pixels nudged, signed.
        dx: i32,
    },
    /// The character fell out of the world (the terrain said so).
    FellOut,
    /// Something of your own happened (an [`Ability`](super::Ability) reported it). Make
    /// one with [`Event::custom`] and add numbers with [`Event::with`].
    Custom {
        /// Which of your events (see [`CustomId`]).
        id: CustomId,
        /// Up to four numbers of your own: a knockback's speed, a hit target's index, a
        /// charge level. Zero when unused.
        data: [f32; 4],
    },
    /// Something solid moved into the character (moving ground carried it into a
    /// ceiling, a moving solid hit it): it's stuck inside. Reported every step it stays
    /// so; usually a respawn.
    Squished,
    /// It was put back at its spawn point (after falling out, unless respawning is off).
    Respawned,
}

impl Event {
    /// A custom event `id` without data.
    pub const fn custom(id: CustomId) -> Self {
        Event::Custom { id, data: [0.0; 4] }
    }

    /// This event with `data` (only for [`Event::Custom`]; other events stay as they are).
    pub const fn with(self, data: [f32; 4]) -> Self {
        match self {
            Event::Custom { id, .. } => Event::Custom { id, data },
            other => other,
        }
    }

    /// The data of custom event `id`, if this is one.
    pub fn custom_data(&self, id: CustomId) -> Option<[f32; 4]> {
        match *self {
            Event::Custom { id: i, data } if i == id => Some(data),
            _ => None,
        }
    }
}
