//! Placeholder art: every hero sprite number as a pose, a rotated block with an eye.

use crate::gfx::{Raster, Rgb, palette as pal};
use crate::hero::sprite as s;
use crate::sim::player;

/// One placeholder pose.
#[derive(Clone, Copy, Debug)]
struct Pose {
    /// Block width.
    w: f32,
    /// Block height.
    h: f32,
    /// Rotation in degrees, positive leaning forward.
    rot: f32,
    /// Vertical offset of the feet, pixels (negative up).
    dy: f32,
    /// Body color.
    c: Rgb,
    /// Wings spread at this angle from straight back, degrees (Monarch Wings frames).
    wings: Option<f32>,
}

/// A pose of `w` x `h`, rotated `rot` degrees, colored `c`.
const fn p(w: f32, h: f32, rot: f32, c: Rgb) -> Pose {
    Pose {
        w,
        h,
        rot,
        dy: 0.0,
        c,
        wings: None,
    }
}

/// The pose for `sprite`.
fn pose(sprite: u16) -> Pose {
    let (g, a, r, d, m, y) = (
        pal::ORANGE,
        pal::CYAN,
        pal::RED,
        pal::YELLOW,
        pal::MAGENTA,
        pal::GREEN,
    );
    match sprite {
        s::IDLE_A => p(6.0, 12.0, 0.0, g),
        s::IDLE_B => p(6.0, 11.0, 0.0, g),
        s::LOOK_UP_A => p(6.0, 12.0, -10.0, g),
        s::LOOK_UP_B => p(6.0, 13.0, -18.0, g),
        s::LOOK_DOWN_A => p(7.0, 11.0, 12.0, g),
        s::LOOK_DOWN_B => p(8.0, 9.0, 22.0, g),
        48 => p(6.0, 12.0, -14.0, g),
        49 => p(6.0, 12.0, 0.0, g),
        50 => p(5.0, 14.0, 0.0, pal::PALE),
        51 => p(7.0, 11.0, 0.0, g),
        52..=55 => {
            let spread = [60.0, 150.0, 100.0, 30.0][(sprite - 52) as usize];
            Pose {
                wings: Some(spread),
                dy: -1.0,
                ..p(6.0, 12.0, 0.0, a)
            }
        }
        56..=59 => Pose {
            dy: -(((sprite - 56) % 2) as f32),
            ..p(7.0, 11.0, 24.0, g)
        },
        60..=63 => Pose {
            dy: -(((sprite - 60) % 2) as f32),
            ..p(7.0, 11.0, -20.0, g)
        },
        s::BOX_HOLD => p(7.0, 11.0, 6.0, g),
        s::RUN_A | s::RUN_C => p(7.0, 11.0, 10.0, g),
        s::RUN_B | s::RUN_D => Pose {
            dy: -1.0,
            ..p(6.0, 12.0, 6.0, g)
        },
        s::SKID => p(8.0, 10.0, -18.0, g),
        s::TAKEOFF => p(8.0, 9.0, 0.0, a),
        s::RISE => p(5.0, 14.0, 0.0, a),
        s::APEX => p(7.0, 11.0, 0.0, a),
        s::FALL_A => p(6.0, 13.0, 0.0, a),
        s::FALL_B => p(5.0, 13.0, 4.0, a),
        12..=15 => Pose {
            dy: -2.0,
            ..p(7.0, 10.0, 90.0 * (sprite - 12) as f32, pal::PALE)
        },
        s::LAND_A => p(9.0, 8.0, 0.0, g),
        s::LAND_B => p(7.0, 10.0, 0.0, g),
        s::HARD_A => p(10.0, 6.0, 0.0, r),
        s::HARD_B => p(8.0, 9.0, 0.0, r),
        20..=23 => Pose {
            dy: -1.0,
            ..p(8.0, 8.0, 90.0 * (sprite - 20) as f32, m)
        },
        24 | 25 => p(11.0 + (sprite - 24) as f32, 8.0, 0.0, d),
        26 | 27 => p(5.0, 14.0 + (sprite - 26) as f32, 0.0, d),
        28 | 29 => p(5.0, 14.0 + (sprite - 28) as f32, 180.0, d),
        30..=33 => Pose {
            dy: -(((sprite - 30) % 2) as f32),
            ..p(6.0, 12.0, -6.0, y)
        },
        34 | 35 => p(6.0, 12.0 - (sprite - 34) as f32, -10.0, y),
        s::WALL_JUMP => p(8.0, 10.0, 20.0, a),
        s::HOP => p(7.0, 10.0, 15.0, y),
        s::DROP => p(5.0, 13.0, 0.0, a),
        s::DIVE_WINDUP => Pose {
            dy: -2.0,
            ..p(8.0, 8.0, 0.0, r)
        },
        40 | 41 => p(5.0, 15.0 + (sprite - 40) as f32, 180.0, r),
        s::DIVE_LAND_A => p(12.0, 6.0, 0.0, r),
        s::DIVE_LAND_B => p(8.0, 9.0, 0.0, r),
        _ => p(6.0, 12.0, 0.0, pal::WHITE),
    }
}

/// Draw `sprite` for a hitbox with top-left `x, y`, facing `facing`. `tint` replaces the
/// body color (afterimages, no dash left); `alpha` < 1 dithers the whole pose away.
pub fn draw(
    r: &mut dyn Raster,
    x: f32,
    y: f32,
    sprite: u16,
    facing: i32,
    tint: Option<Rgb>,
    alpha: f32,
) {
    let pz = pose(sprite);
    let f = facing as f32;
    let cx = x + player::W as f32 / 2.0;
    let feet = y + player::H as f32 + pz.dy;
    let (ox, oy) = (cx, feet - pz.h / 2.0);
    let a = (pz.rot * f).to_radians();
    let (sin, cos) = a.sin_cos();
    let rot = |lx: f32, ly: f32| (ox + lx * cos - ly * sin, oy + lx * sin + ly * cos);
    let (hw, hh) = (pz.w / 2.0, pz.h / 2.0);
    let quad = [rot(-hw, -hh), rot(hw, -hh), rot(hw, hh), rot(-hw, hh)];
    let body = tint.unwrap_or(pz.c);
    if alpha >= 1.0 {
        r.fill_polygon(&quad, body);
    } else {
        let mut mask = crate::gfx::DrawList::default();
        mask.fill_polygon(&quad, body);
        for sp in &mask.spans {
            for px in sp.x0..sp.x1 {
                if alpha > crate::gfx::bayer(px, sp.y) {
                    r.put(px, sp.y, body);
                }
            }
        }
        return;
    }
    if let Some(spread) = pz.wings {
        // Two pale wings from the shoulders, spread back and up.
        for k in [0.0f32, 0.35] {
            let a = (-90.0 - f * (spread * (1.0 - k))).to_radians();
            let (bx, by) = rot(-f, -hh + 3.0);
            let tip = (bx + a.cos() * 9.0, by + a.sin() * 9.0);
            let a2 = a + f * 0.6;
            let mid = (bx + a2.cos() * 6.0, by + a2.sin() * 6.0);
            r.fill_polygon(&[(bx, by), tip, mid], pal::WHITE);
        }
    }
    let (ex, ey) = rot(f * (hw - 1.5), -hh + 2.5);
    r.put(ex.floor() as i32, ey.floor() as i32, pal::WHITE);
    let (sx, sy) = rot(-f * (hw - 0.5), -hh + 4.5);
    r.put(sx.floor() as i32, sy.floor() as i32, body.scale(0.6));
}
