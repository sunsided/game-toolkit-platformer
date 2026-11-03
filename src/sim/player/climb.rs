//! Climbing: hold grab against a wall to hang on, climb up or down at a stamina cost, hop
//! over the top. Releasing grab, losing the wall or running out of stamina lets go.
//!
//! The grip either holds firm or slips ([`Tuning::slip_speed`]); a tired grip (stamina
//! below [`Tuning::tired_stamina`]) slips regardless.
//! Wall slides and wall jumps don't need a grab (see `fall` and `jump`).

use super::{Mode, Player};
use crate::sim::{Event, Intent, Rect, Terrain, Tuning};

/// Seconds after a hop before grabbing again, so the hop carries over the top.
const REGRAB: f32 = 0.2;

impl Player {
    /// Grab, climb, hop or let go.
    pub(super) fn climb(
        &mut self,
        level: &dyn Terrain,
        i: &Intent,
        t: &Tuning,
        dt: f32,
        ev: &mut Vec<Event>,
    ) {
        self.slipping = false;
        if !t.grab {
            return;
        }
        match self.mode {
            Mode::Normal => {
                if self.shove != super::Shove::None {
                    // Grab beside a block on the ground holds the block, not the wall.
                    return;
                }
                let rising = self.vy < 0.0 && self.var_jump > 0.0;
                if i.grab
                    && self.regrab <= 0.0
                    && self.stamina > 0.0
                    && self.wall(level, self.facing)
                    && !rising
                {
                    self.mode = Mode::Climb;
                    self.vx = 0.0;
                    self.vy = 0.0;
                    self.stop_jump();
                    ev.push(Event::Grabbed);
                }
            }
            Mode::Climb => self.hold_on(level, i, t, dt, ev),
            _ => {}
        }
    }

    /// One step on the wall.
    fn hold_on(
        &mut self,
        level: &dyn Terrain,
        i: &Intent,
        t: &Tuning,
        dt: f32,
        ev: &mut Vec<Event>,
    ) {
        let up = i.move_y < -0.5;
        let down = i.move_y > 0.5;
        // The top half of the hitbox must still touch the wall to keep climbing up; above
        // that the hero hops over.
        let head = Rect {
            h: self.h / 2,
            ..self.rect().shifted(self.facing, 0)
        };
        if !i.grab || !self.wall(level, self.facing) {
            self.mode = Mode::Normal;
            ev.push(Event::LetGo);
            return;
        }
        if up && !level.solid(head) {
            self.mode = Mode::Normal;
            self.vy = -t.climb_hop_speed;
            self.vx = self.facing as f32 * t.run_speed * 0.6;
            self.regrab = REGRAB;
            // Keep pushing over the top while the wall is still beside the feet.
            self.force_move = REGRAB;
            self.force_dir = self.facing;
            ev.push(Event::ClimbHop);
            return;
        }
        self.vx = 0.0;
        let tired = self.stamina < t.tired_stamina;
        let slip = if tired {
            t.tired_slip_speed.max(t.slip_speed)
        } else {
            t.slip_speed
        };
        self.vy = if up && !tired {
            -t.climb_up_speed
        } else if down {
            t.climb_down_speed
        } else {
            slip
        };
        self.slipping = !down && self.vy > 0.0;
        let cost = if up {
            t.climb_up_cost
        } else if down {
            0.0
        } else {
            t.climb_still_cost
        };
        self.stamina -= cost * dt;
        if self.stamina <= 0.0 {
            self.stamina = 0.0;
            self.mode = Mode::Normal;
            ev.push(Event::StaminaOut);
            ev.push(Event::LetGo);
        }
    }
}
