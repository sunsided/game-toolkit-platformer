//! [`PushPull`]: pushing and pulling the [`super::Stage`]'s boxes, a move of the demo's own
//! built on the library's [`Ability`] hooks.
//!
//! Standing beside a box at rest, walking into it pushes; holding grab facing it holds it,
//! and walking away pulls. A box sticks until she has strained against it for
//! [`PushPull::breakaway`] (its weight), then speeds up over [`PushPull::ramp`]. While she
//! has hold of one, grab holds the box rather than a wall, and walking is slow.

use super::{Shove, Stage};
use crate::sim::{Ability, Intent, Mode, Player, Step};

/// Pushing and pulling boxes, with their feel.
#[derive(Clone, Debug)]
pub struct PushPull {
    /// Walking speed pushing a box, units/s.
    pub push_speed: f32,
    /// Walking speed pulling a box, units/s.
    pub pull_speed: f32,
    /// Seconds of straining before a box budges (static friction: weight).
    pub breakaway: f32,
    /// Seconds from budging to full push or pull speed.
    pub ramp: f32,
}

impl Default for PushPull {
    fn default() -> Self {
        Self {
            push_speed: 40.0,
            pull_speed: 30.0,
            breakaway: 0.25,
            ramp: 0.3,
        }
    }
}

impl PushPull {
    /// Her hold on a box this step, and which box: within two units on either side, while
    /// standing on the ground and moving freely.
    fn shove(stage: &Stage, p: &Player, i: &Intent) -> Option<(Shove, usize)> {
        if !p.grounded || p.mode != Mode::Normal {
            return None;
        }
        let mx = if i.move_x.abs() > 0.3 {
            i.move_x.signum() as i32
        } else {
            0
        };
        for side in [p.facing, -p.facing] {
            // Within 2 units: a box that just moved a unit ahead still counts.
            let Some(b) = (1..=2).find_map(|d| stage.pushable(p.rect().shifted(side * d, 0)))
            else {
                continue;
            };
            if mx == side {
                return Some((Shove::Push { side }, b));
            }
            if i.grab && side == p.facing {
                let s = if mx == -side {
                    Shove::Pull { side }
                } else {
                    Shove::Hold { side }
                };
                return Some((s, b));
            }
        }
        None
    }
}

impl Ability for PushPull {
    fn before(&mut self, s: &mut Step<'_>) {
        let Some(stage) = s.terrain.find_mut::<Stage>() else {
            return;
        };
        let shove = Self::shove(stage, s.player, s.intent);
        let moving = matches!(shove, Some((Shove::Push { .. } | Shove::Pull { .. }, _)));
        let g = &mut stage.grip;
        g.effort = if moving && shove == g.shove {
            g.effort + s.dt
        } else if moving {
            s.dt
        } else {
            0.0
        };
        g.shove = shove;
        g.from_x = s.player.x;
        // Static friction: nothing moves until she has strained long enough.
        g.give = ((g.effort - self.breakaway) / self.ramp.max(s.dt)).clamp(0.0, 1.0);
        let give = g.give;
        let Some((shove, id)) = shove else {
            return;
        };
        // Grab beside a box holds the box, not the wall.
        s.moves.climb = false;
        s.moves.max_run = match shove {
            Shove::Push { side } => {
                // The box moves first; she follows into the room it made.
                let amount = side as f32 * self.push_speed * give * s.dt;
                stage.shove(id, amount, s.player.rect());
                // She keeps at least the box's pace: up against it, the movement holds her
                // own speed at zero, so it wouldn't build up again by itself.
                let pace = (s.player.vx * side as f32).max(self.push_speed * give);
                s.player.vx = side as f32 * pace;
                self.push_speed * give
            }
            Shove::Pull { .. } => self.pull_speed * give,
            Shove::Hold { .. } => 0.0,
        };
    }

    fn after(&mut self, s: &mut Step<'_>) {
        let Some(stage) = s.terrain.find_mut::<Stage>() else {
            return;
        };
        match stage.grip.shove {
            Some((Shove::Pull { side }, id)) => {
                // The box follows her by as many units as she moved.
                let dx = s.player.x - stage.grip.from_x;
                stage.shove(id, dx as f32, s.player.rect());
                s.player.facing = side;
            }
            // Facing stays on the box while holding it.
            Some((Shove::Hold { side }, _)) => s.player.facing = side,
            _ => {}
        }
    }
}
