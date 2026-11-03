//! Ground contact after moving: landings (plain, rolled, hard, dive impact, the end of a
//! downward dash), walking off ledges, and the airtime's peak.

use super::{Mode, Player};
use crate::sim::{Event, Level, Tuning};

impl Player {
    /// Check the ground under the hero after moving. `landing_vy` is the vertical speed
    /// before the floor stopped it; `jumped` whether the hero left the ground by choice.
    pub(super) fn land(
        &mut self,
        level: &Level,
        move_x: f32,
        landing_vy: f32,
        jumped: bool,
        t: &Tuning,
        ev: &mut Vec<Event>,
    ) {
        let was = self.grounded;
        self.grounded = self.vy >= 0.0 && self.stands(level);
        if self.grounded && !was {
            self.touch_down(move_x, landing_vy, t, ev);
        } else if was && !self.grounded && !jumped {
            ev.push(Event::LeftGround);
            if matches!(self.mode, Mode::Roll { .. }) {
                self.mode = Mode::Normal;
                ev.push(Event::RollEnded);
            }
        }
        if self.grounded {
            self.peak_y = self.y;
        } else {
            self.peak_y = self.peak_y.min(self.y);
        }
    }

    /// The moment of landing. A dive becomes its impact; a downward dash ends. Then, if
    /// moving freely: a queued jump wins (it fires next step, no roll); a high drop while
    /// moving rolls; a very high drop standing still lands hard.
    fn touch_down(&mut self, move_x: f32, landing_vy: f32, t: &Tuning, ev: &mut Vec<Event>) {
        let drop = self.y - self.peak_y;
        ev.push(Event::Landed {
            impact: landing_vy.max(0.0),
            drop,
        });
        self.air_jumps = t.air_jumps;
        self.stop_jump();
        match self.mode {
            Mode::Dive { .. } => {
                self.mode = Mode::DiveLand { t: 0.0 };
                self.vx = 0.0;
                ev.push(Event::DiveImpact { drop });
                return;
            }
            Mode::Dash { dy, .. } if dy > 0.0 => {
                self.mode = Mode::Normal;
                ev.push(Event::DashEnded);
            }
            _ => {}
        }
        if self.mode != Mode::Normal || self.buffer > 0.0 {
            return;
        }
        if drop >= t.roll_drop && move_x != 0.0 {
            self.facing = move_x.signum() as i32;
            self.vx = self.facing as f32 * self.vx.abs().max(t.roll_speed);
            self.mode = Mode::Roll { t: 0.0 };
            ev.push(Event::RollStarted);
        } else if drop >= t.hard_land_drop {
            self.mode = Mode::HardLand { t: 0.0 };
            ev.push(Event::HardLanded);
        }
    }
}
