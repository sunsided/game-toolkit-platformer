//! [`Wind`]: the air the heroine's mane, headband and cape move in. Drawing only.
//!
//! A steady breeze, stronger the higher she is, streams her mane and cape out behind her
//! whichever way she faces; gusts from the moon's side (right to left) build and die down
//! on top of it and blow in their real direction, so a gust from behind carries them
//! forward. Gusts come from three slow waves of
//! different periods added up, so they arrive irregularly but the same way every run, and
//! fade out over a second or so rather than stopping.

/// Breeze at the bottom of the level, units/s² of push on a loose strand.
const BREEZE_LOW: f32 = 120.0;
/// Breeze at the top of the level.
const BREEZE_HIGH: f32 = 400.0;
/// Extra push at the peak of a gust.
const GUST: f32 = 350.0;
/// The level's height in units, for "how high up".
const LEVEL_H: f32 = 216.0;

/// The wind at one moment and height.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Wind {
    /// The steady breeze's strength, units/s², faded out while a gust blows. It only
    /// streams loose parts out behind her, whichever way she faces.
    pub breeze: f32,
    /// A gust's push in its real direction, units/s² (x positive to the right, y down).
    pub push: (f32, f32),
    /// How strong the current gust is, 0 (none) to 1 (peak).
    pub gust: f32,
}

/// How long a gust takes to die down, seconds (the time constant of its fall-off).
const FADE: f32 = 0.6;
/// How far back to look for a gust still dying down, in samples of [`FADE_STEP`].
const FADE_SAMPLES: i32 = 20;
/// Seconds between the samples.
const FADE_STEP: f32 = 0.1;

/// The raw gust signal at `t` seconds: three slow waves of different periods added up,
/// above a threshold.
fn raw(t: f32) -> f32 {
    let waves = (t * 0.7).sin() + 0.6 * (t * 1.9 + 1.0).sin() + 0.8 * (t * 0.31 + 2.0).sin();
    ((waves - 0.9) / 1.5).clamp(0.0, 1.0)
}

/// Smoothstep: eases in and out of 0 and 1.
fn ease(x: f32) -> f32 {
    x * x * (3.0 - 2.0 * x)
}

impl Wind {
    /// The wind at simulation step `tick` for a body whose top is at `y` (units).
    pub fn at(tick: u64, y: f32) -> Self {
        let t = tick as f32 / 120.0;
        let height = (1.0 - y / LEVEL_H).clamp(0.0, 1.0);
        let breeze = BREEZE_LOW + (BREEZE_HIGH - BREEZE_LOW) * height;
        // A gust dies down instead of stopping: the strongest recent value, fading
        // exponentially with its age, then eased.
        let held = (0..FADE_SAMPLES)
            .map(|j| {
                let age = j as f32 * FADE_STEP;
                raw(t - age) * (-age / FADE).exp()
            })
            .fold(0.0f32, f32::max);
        let gust = ease(held.clamp(0.0, 1.0));
        // Gusts lift a little, too.
        Self {
            breeze: breeze * (1.0 - gust),
            push: (-GUST * gust, -GUST * 0.1 * gust),
            gust,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn higher_is_windier_and_gusts_come_and_go() {
        let calm = (0..120 * 60)
            .find(|&k| Wind::at(k, 100.0).gust == 0.0)
            .unwrap();
        assert!(
            Wind::at(calm, 10.0).breeze > Wind::at(calm, 200.0).breeze,
            "stronger up high"
        );
        let gusts: Vec<f32> = (0..120 * 60).map(|k| Wind::at(k, 100.0).gust).collect();
        assert!(gusts.iter().any(|&g| g > 0.5), "a strong gust in a minute");
        assert!(
            gusts.iter().filter(|&&g| g == 0.0).count() > gusts.len() / 3,
            "and calm"
        );
    }
}
