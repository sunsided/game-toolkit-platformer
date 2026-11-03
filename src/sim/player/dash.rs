//! Dashes: a burst in one of eight directions with gravity off, handing over part of its
//! speed when it ends. Up and down are just directions; a downward dash that meets the
//! ground ends there (see `land`).

use std::f32::consts::FRAC_1_SQRT_2;

use super::{Mode, Player};
use crate::sim::{Event, Intent, Tuning};

/// The stick has to lean this far to count as a direction.
const DEADZONE: f32 = 0.3;

impl Player {
    /// Start a buffered dash if one is left, the cooldown is over and the body isn't busy
    /// recovering. The direction is the stick's, snapped to eight ways; with the stick
    /// centered, the facing direction.
    pub(super) fn try_dash(&mut self, i: &Intent, t: &Tuning, ev: &mut Vec<Event>) {
        let able = matches!(
            self.mode,
            Mode::Normal | Mode::Roll { .. } | Mode::Climb | Mode::Dive { .. }
        );
        if self.dash_buffer <= 0.0 || self.dashes == 0 || self.dash_cooldown > 0.0 || !able {
            return;
        }
        let snap = |v: f32| if v.abs() > DEADZONE { v.signum() } else { 0.0 };
        let (mut dx, mut dy) = (snap(i.move_x), snap(i.move_y));
        if !t.dash_eight_way {
            dy = 0.0;
        }
        if dx == 0.0 && dy == 0.0 {
            dx = self.facing as f32;
        }
        if dx != 0.0 && dy != 0.0 {
            dx *= FRAC_1_SQRT_2;
            dy *= FRAC_1_SQRT_2;
        }
        if dx != 0.0 {
            self.facing = dx.signum() as i32;
        }
        self.dashes -= 1;
        self.dash_cooldown = t.dash_cooldown;
        self.dash_buffer = 0.0;
        self.stop_jump();
        self.mode = Mode::Dash { t: 0.0, dx, dy };
        self.vx = dx * t.dash_speed;
        self.vy = dy * t.dash_speed;
        ev.push(Event::Dashed { dx, dy });
    }

    /// The dash is over: keep part of its speed (less of it going up) and move freely.
    pub(super) fn end_dash(&mut self, t: &Tuning) {
        if let Mode::Dash { dx, dy, .. } = self.mode {
            self.vx = dx * t.dash_end_speed;
            self.vy = dy * t.dash_end_speed * if dy < 0.0 { t.dash_end_up } else { 1.0 };
        }
        self.mode = Mode::Normal;
    }
}
