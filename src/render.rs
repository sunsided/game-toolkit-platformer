//! Draws a [`Session`]: the level into a cached backdrop [`Canvas`], everything that moves
//! (hero, crystals, effects) and the debug overlay into any [`Raster`].
//!
//! The hero has no art yet: [`hero`] draws each sprite number as a placeholder pose (a
//! squashed, stretched, rotated block with an eye), enough to read every animation state
//! and frame. Real art replaces that module, not the controller.

pub mod debug;
pub mod hero;

use crate::gfx::{Canvas, H, Raster, W, bayer, palette as pal};
use crate::session::{Fx, FxColor, Session};
use crate::sim::{Level, TILE, Tile};

/// Draw the level's static part: the sky and the tiles.
pub fn backdrop(level: &Level) -> Canvas {
    let mut c = Canvas::new(W, H);
    for y in 0..H {
        let t = y as f32 / H as f32;
        for x in 0..W {
            let band = (t * 6.0).floor() / 6.0;
            let frac = t * 6.0 - (t * 6.0).floor();
            let up = if frac > bayer(x, y) { 1.0 / 6.0 } else { 0.0 };
            c.put(x, y, pal::SKY_TOP.lerp(pal::SKY_BOTTOM, band + up));
        }
    }
    for ty in 0..level.h {
        for tx in 0..level.w {
            let (x, y) = (tx * TILE, ty * TILE);
            match level.tile(tx, ty) {
                Tile::Empty => {}
                Tile::Solid => {
                    c.fill_rect(x, y, TILE, TILE, pal::STONE);
                    c.fill_rect(x, y + TILE - 2, TILE, 2, pal::STONE_DK);
                    if !level.solid(tx, ty - 1) {
                        c.fill_rect(x, y, TILE, 1, pal::STONE_LT);
                    }
                    if !level.solid(tx - 1, ty) {
                        c.fill_rect(x, y, 1, TILE, pal::STONE_DK);
                    }
                }
                Tile::Platform => {
                    c.fill_rect(x, y, TILE, 1, pal::STONE_LT);
                    c.fill_rect(x, y + 1, TILE, 2, pal::STONE);
                    c.put(x + 1, y + 3, pal::STONE_DK);
                    c.put(x + TILE - 2, y + 3, pal::STONE_DK);
                }
            }
        }
    }
    c
}

/// Draw everything that moves: crystals, effects, the hero, and the overlay if on.
pub fn scene(r: &mut dyn Raster, s: &Session, clock: f32) {
    for c in &s.world.crystals {
        let (cx, cy) = (c.area.x + TILE / 2, c.area.y + TILE / 2);
        let bob = ((clock * 3.0).sin() * 1.5).round() as i32;
        if c.ready() {
            diamond(r, cx, cy + bob, 3, pal::CYAN);
            r.put(cx - 1, cy + bob - 1, pal::WHITE);
        } else {
            r.put(cx, cy - 3, pal::STEEL);
            r.put(cx, cy + 3, pal::STEEL);
            r.put(cx - 3, cy, pal::STEEL);
            r.put(cx + 3, cy, pal::STEEL);
        }
    }
    for b in &s.world.blocks {
        let q = b.rect();
        r.fill_rect(q.x, q.y, q.w, q.h, pal::WOOD);
        r.rect_outline(q.x, q.y, q.w, q.h, pal::WOOD_DK);
        r.line(
            q.x + 1,
            q.y + 1,
            q.right() - 2,
            q.bottom() - 2,
            pal::WOOD_DK,
        );
        r.line(
            q.right() - 2,
            q.y + 1,
            q.x + 1,
            q.bottom() - 2,
            pal::WOOD_DK,
        );
        r.fill_rect(q.x + 1, q.y + 1, q.w - 2, 1, pal::WOOD_LT);
    }
    for f in &s.fx {
        effect(r, f);
    }
    let (x, y) = s.hero_pos();
    let p = &s.world.player;
    let tint = if p.dashes == 0 {
        Some(pal::STEEL_LT)
    } else {
        None
    };
    hero::draw(r, x, y, s.animator.sprite(), p.facing, tint, 1.0);
    debug::draw(r, s);
}

/// A filled diamond of radius `rad`.
fn diamond(r: &mut dyn Raster, cx: i32, cy: i32, rad: i32, c: crate::gfx::Rgb) {
    for dy in -rad..=rad {
        let half = rad - dy.abs();
        r.span(cx - half, cx + half + 1, cy + dy, c, 1.0);
    }
}

/// Draw one effect.
fn effect(r: &mut dyn Raster, f: &Fx) {
    let k = f.progress();
    match *f {
        Fx::Ring {
            x, y, r: rad, kind, ..
        } => {
            let c = match kind {
                FxColor::Recharge => pal::CYAN,
                FxColor::Air => pal::PALE,
                FxColor::Impact => pal::YELLOW,
            };
            let rr = rad * k;
            let n = (rr * 6.0).max(8.0) as i32;
            for i in 0..n {
                let a = i as f32 / n as f32 * std::f32::consts::TAU;
                let (px, py) = ((x + a.cos() * rr) as i32, (y + a.sin() * rr) as i32);
                if 1.0 - k > bayer(px, py) {
                    r.put(px, py, c);
                }
            }
        }
        Fx::Ghost {
            x,
            y,
            sprite,
            facing,
            ..
        } => {
            hero::draw(r, x, y, sprite, facing, Some(pal::CYAN), 0.5 * (1.0 - k));
        }
        Fx::Dust { x, y, spread, .. } => {
            let d = spread * k;
            for side in [-1.0, 1.0] {
                for j in 0..3 {
                    let px = (x + side * (d - j as f32 * 2.0)) as i32;
                    let py = (y - 1.0 - j as f32 * k * 2.0) as i32;
                    if 1.0 - k > bayer(px, py) {
                        r.put(px, py, pal::PALE);
                    }
                }
            }
        }
    }
}
