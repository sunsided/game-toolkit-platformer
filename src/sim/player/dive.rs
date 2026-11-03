//! Dives: in the air, a short hang, then straight down at high speed until the ground,
//! where the impact costs a moment of control (see `land`).

use super::{Mode, Player};
use crate::sim::{Event, Intent, Tuning};

impl Player {
    /// Start a dive on the press, in the air, from free movement or a dash.
    pub(super) fn try_dive(&mut self, i: &Intent, t: &Tuning, ev: &mut Vec<Event>) {
        if !t.dive
            || !i.dive_pressed
            || self.grounded
            || !matches!(self.mode, Mode::Normal | Mode::Dash { .. })
        {
            return;
        }
        self.mode = Mode::Dive { t: 0.0 };
        self.vx = 0.0;
        self.vy = 0.0;
        self.stop_jump();
        ev.push(Event::DiveStarted);
    }

    /// While diving: hang during the windup, then plunge.
    pub(super) fn dive_motion(&mut self, t: &Tuning) {
        if let Mode::Dive { t: dt } = self.mode {
            self.vx = 0.0;
            self.vy = if dt < t.dive_windup {
                0.0
            } else {
                t.dive_speed
            };
        }
    }
}
