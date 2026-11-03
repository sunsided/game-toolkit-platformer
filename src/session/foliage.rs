//! [`Foliage`]: moss hanging under the platforms and grass on the rock tops, each sprig a
//! little spring-damped pendulum. Drawing only; the simulation never sees it.
//!
//! Sprigs hang or stand still in calm air and lean and flutter in a gust (the ones the
//! streaks in the sky show; stronger higher up, in the gust's real direction), bend aside where the heroine brushes through, and take a push from
//! what she does: every footstep the animation controller marks rustles the grass under
//! that foot, and landings, rolls, dashes, dives and the super dash shove sprigs nearby
//! away from her.

use crate::session::Wind;
use crate::sim::{Level, Rect, TILE, Tile};

/// One sprig of moss or blade of grass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sprig {
    /// Where it grows from, units.
    pub x: f32,
    /// Where it grows from, units.
    pub y: f32,
    /// Its length, units.
    pub len: f32,
    /// Hanging down (moss) or standing up (grass).
    pub hangs: bool,
    /// Its lean from rest, radians, positive toward +x.
    pub angle: f32,
    /// How fast the lean changes, radians/s.
    vel: f32,
    /// How hard it springs back.
    stiff: f32,
    /// Its own flutter: radians/s and a starting phase.
    flutter: (f32, f32),
}

impl Sprig {
    /// The tip, units.
    pub fn tip(&self) -> (f32, f32) {
        let (s, c) = self.angle.sin_cos();
        let up = if self.hangs { 1.0 } else { -1.0 };
        (self.x + s * self.len, self.y + up * c * self.len)
    }
}

/// How much sideways wind (units/s²) leans a sprig (radians/s²).
const WIND_LEAN: f32 = 0.025;
/// How much the wind's strength makes a sprig flutter (radians/s² per unit/s²).
const FLUTTER: f32 = 0.012;
/// Damping of the sway.
const DAMPING: f32 = 5.0;
/// How far a sprig may bend, radians.
const MAX_LEAN: f32 = 1.3;
/// How hard brushing through swings a sprig: radians/s² per unit/s of her speed while
/// she touches it.
const BRUSH: f32 = 0.6;

/// All the plants of a level.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Foliage {
    /// Every sprig.
    pub sprigs: Vec<Sprig>,
}

/// A hash for placing plants: the same level grows the same plants.
fn hash(a: i32, b: i32) -> u32 {
    let mut h = (a as u32).wrapping_mul(0x9e37_79b1) ^ (b as u32).wrapping_mul(0x85eb_ca77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^ (h >> 13)
}

impl Foliage {
    /// Grow plants on `level`: moss under some tile undersides (rock and planks) above open
    /// air, grass on some exposed rock tops.
    pub fn grow(level: &Level) -> Self {
        let mut sprigs = Vec::new();
        for ty in 0..level.h {
            for tx in 0..level.w {
                let tile = level.tile(tx, ty);
                if tile == Tile::Empty {
                    continue;
                }
                let (x0, y0) = ((tx * TILE) as f32, (ty * TILE) as f32);
                let open_below = level.tile(tx, ty + 1) == Tile::Empty;
                let open_above = level.tile(tx, ty - 1) == Tile::Empty;
                // Moss under undersides, not too low to the floor.
                if open_below && ty + 3 < level.h && hash(tx, ty) % 5 < 3 {
                    let bottom = if tile == Tile::Platform {
                        y0 + 3.0
                    } else {
                        y0 + TILE as f32
                    };
                    for k in 0..1 + hash(tx, ty + 7) % 3 {
                        let h = hash(tx * 7 + k as i32, ty);
                        sprigs.push(Sprig {
                            x: x0 + 1.0 + (h % 6) as f32,
                            y: bottom,
                            len: 3.0 + (h / 7 % 7) as f32,
                            hangs: true,
                            angle: 0.0,
                            vel: 0.0,
                            // Moss is light and hangs loose.
                            stiff: 8.0 + (h / 50 % 8) as f32,
                            flutter: (2.0 + (h % 17) as f32 * 0.15, (h % 628) as f32 / 100.0),
                        });
                    }
                }
                // Grass on rock tops.
                if tile == Tile::Solid && open_above && hash(tx, ty + 31) % 3 < 2 {
                    for k in 0..2 + hash(tx, ty + 11) % 3 {
                        let h = hash(tx * 13 + k as i32, ty + 3);
                        sprigs.push(Sprig {
                            x: x0 + (h % 8) as f32 + 0.5,
                            y: y0,
                            len: 1.5 + (h / 8 % 3) as f32,
                            hangs: false,
                            angle: 0.0,
                            vel: 0.0,
                            stiff: 110.0 + (h / 40 % 60) as f32,
                            flutter: (4.0 + (h % 13) as f32 * 0.3, (h % 628) as f32 / 100.0),
                        });
                    }
                }
            }
        }
        Self { sprigs }
    }

    /// One step of `dt`: wind at simulation step `tick`, and the heroine's `body` (units)
    /// moving at `vel` (units/s) brushing through.
    pub fn step(&mut self, tick: u64, body: Rect, vel: (f32, f32), dt: f32) {
        // Only gusts move the plants (the breeze is for her mane and cape); a gust pushes
        // harder higher up.
        let gust = Wind::at(tick, 0.0).push.0;
        for s in &mut self.sprigs {
            let height = 1.0 - (s.y / 216.0).clamp(0.0, 1.0);
            let wind = gust * (0.6 + 0.6 * height);
            // Leaning with the wind, and fluttering in it: the stronger, the livelier.
            let t = tick as f32 / 120.0;
            let flutter = (t * s.flutter.0 + s.flutter.1).sin() * wind.abs() * FLUTTER;
            let mut acc = -s.stiff * s.angle - DAMPING * s.vel + wind * WIND_LEAN + flutter;
            let (lo, hi) = if s.hangs {
                (s.y, s.y + s.len)
            } else {
                (s.y - s.len, s.y)
            };
            let touched = body.x as f32 <= s.x + 1.0
                && s.x - 1.0 <= body.right() as f32
                && body.y as f32 <= hi
                && lo <= body.bottom() as f32;
            if touched {
                acc += vel.0 * BRUSH;
            }
            s.vel += acc * dt;
            s.angle = (s.angle + s.vel * dt).clamp(-MAX_LEAN, MAX_LEAN);
        }
    }

    /// Shove sprigs within `radius` of `x, y` (units) away from it, by up to `strength`
    /// radians/s, less with distance.
    pub fn shove(&mut self, x: f32, y: f32, radius: f32, strength: f32) {
        for s in &mut self.sprigs {
            let (mx, my) = (
                s.x,
                if s.hangs {
                    s.y + s.len / 2.0
                } else {
                    s.y - s.len / 2.0
                },
            );
            let d = ((mx - x).powi(2) + (my - y).powi(2)).sqrt();
            if d < radius {
                let side = if mx >= x { 1.0 } else { -1.0 };
                s.vel += side * strength * (1.0 - d / radius);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_demo_level_grows_moss_and_grass() {
        let f = Foliage::grow(&Level::demo());
        let moss = f.sprigs.iter().filter(|s| s.hangs).count();
        let grass = f.sprigs.len() - moss;
        assert!(moss > 20 && grass > 40, "{moss} moss, {grass} grass");
    }

    #[test]
    fn brushing_through_bends_and_it_springs_back() {
        let mut f = Foliage {
            sprigs: vec![Sprig {
                x: 50.0,
                y: 100.0,
                len: 3.0,
                hangs: false,
                angle: 0.0,
                vel: 0.0,
                stiff: 120.0,
                flutter: (5.0, 0.0),
            }],
        };
        let calm = (0..120 * 60)
            .find(|&k| Wind::at(k, 100.0).gust == 0.0)
            .unwrap();
        let body = Rect {
            x: 48,
            y: 88,
            w: 6,
            h: 12,
        };
        for k in 0..12 {
            f.step(calm + k, body, (90.0, 0.0), 1.0 / 120.0);
        }
        let bent = f.sprigs[0].angle;
        assert!(bent > 0.1, "bent along her run: {bent}");
        let away = Rect { x: 0, ..body };
        for k in 0..240 {
            f.step(calm + 12 + k, away, (0.0, 0.0), 1.0 / 120.0);
        }
        assert!(
            f.sprigs[0].angle.abs() < bent / 2.0,
            "springs back toward the breeze lean"
        );
    }

    #[test]
    fn a_landing_shoves_grass_away_from_her() {
        let mut f = Foliage::grow(&Level::demo());
        let (x, y) = (100.0, 200.0);
        f.shove(x, y, 12.0, 3.0);
        for s in f
            .sprigs
            .iter()
            .filter(|s| (s.x - x).abs() < 8.0 && (s.y - y).abs() < 4.0)
        {
            assert!((s.vel > 0.0) == (s.x >= x), "{s:?}");
        }
    }

    #[test]
    fn moss_hangs_still_in_calm_air_and_sways_in_a_gust() {
        let mut f = Foliage::grow(&Level::demo());
        let away = Rect {
            x: -100,
            y: -100,
            w: 1,
            h: 1,
        };
        let moss = |f: &Foliage| -> Vec<f32> {
            f.sprigs
                .iter()
                .filter(|s| s.hangs)
                .map(|s| s.angle)
                .collect()
        };
        let calm = (600..120 * 60)
            .find(|&k| Wind::at(k, 0.0).gust == 0.0 && Wind::at(k - 240, 0.0).gust == 0.0)
            .unwrap();
        for k in calm - 240..=calm {
            f.step(k, away, (0.0, 0.0), 1.0 / 120.0);
        }
        let still = moss(&f).iter().map(|a| a.abs()).fold(0.0f32, f32::max);
        assert!(still < 0.02, "calm: hangs straight ({still})");
        let peak = (0..120 * 60)
            .max_by(|a, b| Wind::at(*a, 0.0).gust.total_cmp(&Wind::at(*b, 0.0).gust))
            .unwrap();
        let mut f = Foliage::grow(&Level::demo());
        let mut leans = Vec::new();
        for k in peak - 120..peak + 120 {
            f.step(k, away, (0.0, 0.0), 1.0 / 120.0);
            leans.push(moss(&f).iter().sum::<f32>() / moss(&f).len() as f32);
        }
        let deepest = leans.iter().copied().fold(0.0f32, f32::min);
        assert!(
            deepest < -0.15,
            "gust: leans with it (right to left): {deepest}"
        );
    }
}
