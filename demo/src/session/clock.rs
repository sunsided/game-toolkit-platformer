//! [`Clock`]: turns frame time into fixed simulation steps, with slow motion, pause and
//! single steps.

use crate::sim::DT;

/// Playback speeds, slowest first.
pub const SPEEDS: [f32; 6] = [1.0 / 16.0, 1.0 / 8.0, 1.0 / 4.0, 1.0 / 2.0, 1.0, 2.0];
/// The index of normal speed in [`SPEEDS`].
const NORMAL: usize = 4;
/// At most this many steps per frame, so a hitch doesn't spiral.
const MAX_STEPS: u32 = 8;

/// The fixed-step clock.
#[derive(Clone, Debug)]
pub struct Clock {
    /// Index into [`SPEEDS`].
    speed: usize,
    /// Paused: only single steps advance.
    pub paused: bool,
    /// Scaled time not yet stepped.
    acc: f32,
    /// A single step was asked for while paused.
    step_once: bool,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            speed: NORMAL,
            paused: false,
            acc: 0.0,
            step_once: false,
        }
    }
}

impl Clock {
    /// The playback speed multiplier.
    pub fn speed(&self) -> f32 {
        SPEEDS[self.speed]
    }

    /// Slower, faster, or back to normal.
    pub fn change(&mut self, slower: bool, faster: bool, normal: bool) {
        if slower {
            self.speed = self.speed.saturating_sub(1);
        }
        if faster {
            self.speed = (self.speed + 1).min(SPEEDS.len() - 1);
        }
        if normal {
            self.speed = NORMAL;
        }
    }

    /// Pause or go on.
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        self.acc = 0.0;
    }

    /// While paused: take exactly one step next frame.
    pub fn step(&mut self) {
        self.paused = true;
        self.step_once = true;
    }

    /// How many fixed steps `dt` seconds of frame time make.
    pub fn advance(&mut self, dt: f32) -> u32 {
        if self.paused {
            return std::mem::take(&mut self.step_once) as u32;
        }
        self.acc += dt * self.speed();
        let n = ((self.acc / DT) as u32).min(MAX_STEPS);
        self.acc = (self.acc - n as f32 * DT).clamp(0.0, DT);
        n
    }

    /// How far into the next step the clock is, 0..1, for drawing between steps.
    pub fn alpha(&self) -> f32 {
        if self.paused {
            1.0
        } else {
            (self.acc / DT).clamp(0.0, 1.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slow_motion_steps_less_often() {
        let mut c = Clock::default();
        let normal: u32 = (0..60).map(|_| c.advance(1.0 / 60.0)).sum();
        c.change(false, false, true);
        c.change(true, false, false);
        let half: u32 = (0..60).map(|_| c.advance(1.0 / 60.0)).sum();
        assert!((119..=121).contains(&normal), "{normal}");
        assert!((59..=61).contains(&half), "{half}");
    }

    #[test]
    fn paused_steps_once_on_request() {
        let mut c = Clock::default();
        c.toggle_pause();
        assert_eq!(c.advance(1.0), 0);
        c.step();
        assert_eq!(c.advance(0.0), 1);
        assert_eq!(c.advance(0.0), 0);
    }
}
