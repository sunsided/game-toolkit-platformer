//! Jumps, in order of preference: out of a roll, from the ground, within coyote time, off
//! a wall (or straight up it while climbing), and in the air. Wall jumps follow the stick:
//! toward the wall is a short kick back onto the same wall (chained, it scales the wall),
//! away a full leap; up and down tilt it. Down + jump on a one-way
//! platform drops through it instead.

use super::{Mode, Player};
use crate::sim::{Event, Intent, JumpKind, Terrain, Tuning};

impl Player {
    /// Turn a waiting jump press into a jump when one is possible. Falling just above the
    /// ground, the press keeps waiting for the landing instead of spending the air jump.
    /// Returns whether the hero left the ground by choice (jump or drop-through).
    pub(super) fn jump(
        &mut self,
        level: &dyn Terrain,
        i: &Intent,
        move_x: f32,
        t: &Tuning,
        ev: &mut Vec<Event>,
    ) -> bool {
        let able = match self.mode {
            // Jumping cancels a super dash charge.
            Mode::Normal | Mode::Climb | Mode::Dash { .. } | Mode::SuperCharge { .. } => true,
            Mode::Roll { t: rt } => rt >= t.roll_jump_after,
            Mode::HardLand { .. }
            | Mode::Dive { .. }
            | Mode::DiveLand { .. }
            | Mode::SuperDash { .. } => false,
        };
        if self.buffer <= 0.0 || !able {
            return false;
        }
        if self.grounded && i.down() && self.on_platform_only(level) {
            self.drop_through = t.drop_through_time;
            self.grounded = false;
            self.coyote = 0.0;
            self.buffer = 0.0;
            ev.push(Event::DroppedThrough);
            return true;
        }
        let wall = self.wall_near(level, t.wall_jump_reach);
        let kind = if matches!(self.mode, Mode::Roll { .. }) {
            JumpKind::Roll
        } else if self.grounded {
            JumpKind::Ground
        } else if self.coyote > 0.0 {
            JumpKind::Coyote
        } else if self.mode == Mode::Climb {
            if move_x * (self.facing as f32) < 0.0 {
                JumpKind::Wall
            } else {
                JumpKind::Climb
            }
        } else if wall.is_some() {
            JumpKind::Wall
        } else if self.air_jumps > 0 && !self.ground_below(level, t.land_jump_lookahead) {
            JumpKind::Double
        } else {
            return false;
        };
        self.var_jump = t.var_jump_time;
        self.var_jump_speed = t.jump_speed;
        self.vy = -t.jump_speed;
        match kind {
            JumpKind::Double => {
                self.air_jumps -= 1;
                self.vy = -t.double_jump_speed;
                self.var_jump = t.double_var_jump_time;
                self.var_jump_speed = t.double_jump_speed;
                if move_x != 0.0 {
                    self.vx = move_x * self.vx.abs().max(t.run_speed);
                    self.facing = move_x.signum() as i32;
                }
            }
            JumpKind::Wall => {
                let side = match self.mode {
                    Mode::Climb => self.facing,
                    Mode::SuperCharge { wall: w, .. } if w != 0 => w,
                    _ => wall.unwrap_or(self.facing),
                };
                // The stick aims the jump. Sideways: toward the wall is a small kick
                // that arcs back onto the same wall higher up (chained, it scales the
                // wall), away is the full leap, neutral in between. Up and down tilt the
                // jump higher or flatter.
                let away = (-move_x * side as f32 + 1.0) / 2.0;
                let speed = t.wall_climb_speed + (t.wall_jump_speed - t.wall_climb_speed) * away;
                let lock = t.wall_climb_lock + (t.wall_jump_lock - t.wall_climb_lock) * away;
                self.vy = -t.jump_speed * (1.0 - t.wall_jump_aim * i.move_y.clamp(-1.0, 1.0));
                self.vx = -side as f32 * speed;
                self.facing = -side;
                self.force_move = lock;
                self.force_dir = -side;
            }
            JumpKind::Climb => {
                self.stamina = (self.stamina - t.climb_jump_cost).max(0.0);
            }
            JumpKind::Ground | JumpKind::Coyote | JumpKind::Roll => {
                self.vx += move_x * t.jump_boost;
                self.grounded = false;
                self.coyote = 0.0;
                self.peak_y = self.y;
            }
        }
        // Off moving ground (or a moving wall): keep its speed, upward only.
        let (lx, ly) = self.lift;
        self.vx += lx * t.lift_boost;
        self.vy += ly.min(0.0) * t.lift_boost;
        self.var_jump_speed -= ly.min(0.0) * t.lift_boost;
        self.lift = (0.0, 0.0);
        self.lift_time = 0.0;
        self.mode = Mode::Normal;
        self.buffer = 0.0;
        self.can_cut = true;
        ev.push(Event::Jumped {
            kind,
            buffered: !i.jump_pressed,
        });
        true
    }

    /// Standing on a one-way platform with no solid ground under the feet.
    fn on_platform_only(&self, level: &dyn Terrain) -> bool {
        let r = self.rect();
        level.platform(r) && !level.solid(r.shifted(0, 1))
    }

    /// The side of a wall within `reach` pixels (facing side first), for wall jumps.
    fn wall_near(&self, level: &dyn Terrain, reach: i32) -> Option<i32> {
        [self.facing, -self.facing]
            .into_iter()
            .find(|&side| (1..=reach).any(|d| level.solid(self.rect().shifted(side * d, 0))))
    }
}
