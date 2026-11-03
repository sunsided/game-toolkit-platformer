//! The debug look (F3): every hero sprite as a rotated block with an eye, colored by
//! kind of state.

use super::KF;
use crate::gfx::{Raster, Rgb, palette as pal};
use crate::hero::pose::pose;
use crate::sim::player;

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
    // In units, then scaled to pixels.
    let rot = |lx: f32, ly: f32| {
        (
            (ox + lx * cos - ly * sin) * KF,
            (oy + lx * sin + ly * cos) * KF,
        )
    };
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
            let tip = (bx + a.cos() * 9.0 * KF, by + a.sin() * 9.0 * KF);
            let a2 = a + f * 0.6;
            let mid = (bx + a2.cos() * 6.0 * KF, by + a2.sin() * 6.0 * KF);
            r.fill_polygon(&[(bx, by), tip, mid], pal::WHITE);
        }
    }
    let (ex, ey) = rot(f * (hw - 1.5), -hh + 2.5);
    r.put(ex.floor() as i32, ey.floor() as i32, pal::WHITE);
    let (sx, sy) = rot(-f * (hw - 0.5), -hh + 4.5);
    r.put(sx.floor() as i32, sy.floor() as i32, body.scale(0.6));
}
