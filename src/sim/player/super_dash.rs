//! The super dash (Hollow Knight's Crystal Heart): hold the button standing or on a wall
//! to charge; let go once charged to fly straight and level, gravity off, until a wall
//! stops it (a bonk) or jump or the button cancels it. Let go early and nothing happens;
//! jumping during the charge cancels it too (see `jump`).

use super::{Mode, Player};
use crate::sim::{Event, Intent, Level, Tuning};

/// Speed knocked back off the wall a super dash hits, sideways and up.
const BONK: (f32, f32) = (60.0, 90.0);

impl Player {
    /// Start, keep, launch or cancel a super dash.
    pub(super) fn super_dash(
        &mut self,
        level: &Level,
        i: &Intent,
        t: &Tuning,
        ev: &mut Vec<Event>,
    ) {
        let fresh = i.super_dash && !self.was_super;
        self.was_super = i.super_dash;
        if !t.super_dash {
            return;
        }
        match self.mode {
            Mode::Normal | Mode::Climb if fresh => {
                let wall = if self.mode == Mode::Climb {
                    self.facing
                } else {
                    self.wall_slide
                };
                if !self.grounded && wall == 0 {
                    return;
                }
                let wall = if self.grounded { 0 } else { wall };
                self.mode = Mode::SuperCharge { t: 0.0, wall };
                self.vx = 0.0;
                self.vy = 0.0;
                self.stop_jump();
            }
            Mode::SuperCharge { t: charged, wall } => {
                self.vx = 0.0;
                if wall != 0 {
                    // Clinging: the wall holds the hero while it charges.
                    self.vy = 0.0;
                }
                let lost = if wall == 0 {
                    !self.grounded
                } else {
                    !self.wall(level, wall)
                };
                if lost {
                    self.mode = Mode::Normal;
                } else if !i.super_dash && charged >= t.super_charge {
                    let dir = if wall != 0 { -wall } else { self.facing };
                    self.facing = dir;
                    self.mode = Mode::SuperDash { dir };
                    self.vx = dir as f32 * t.super_speed;
                    ev.push(Event::SuperDashStarted { dir });
                } else if !i.super_dash {
                    // Let go too early: nothing.
                    self.mode = Mode::Normal;
                }
            }
            Mode::SuperDash { dir } => {
                if fresh || i.jump_pressed {
                    self.mode = Mode::Normal;
                    self.vx = dir as f32 * t.super_cancel_speed;
                    // The press cancels; it doesn't also jump.
                    self.buffer = 0.0;
                    ev.push(Event::SuperDashEnded { bonk: false });
                } else {
                    self.vx = dir as f32 * t.super_speed;
                    self.vy = 0.0;
                }
            }
            _ => {}
        }
    }

    /// The super dash hit a wall: knocked back a little, falling.
    pub(super) fn super_bonk(&mut self, dir: i32, ev: &mut Vec<Event>) {
        self.mode = Mode::Normal;
        self.vx = -dir as f32 * BONK.0;
        self.vy = -BONK.1;
        ev.push(Event::SuperDashEnded { bonk: true });
    }
}
