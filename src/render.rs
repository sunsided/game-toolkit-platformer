//! Draws a [`Session`]: the level into a cached backdrop [`Canvas`], everything that moves
//! (hero, crystals, effects) and the debug overlay into any [`Raster`].
//!
//! The hero has no art yet: [`hero`] draws each sprite number as a placeholder pose (a
//! squashed, stretched, rotated block with an eye), enough to read every animation state
//! and frame. Real art replaces that module, not the controller.

pub mod debug;
pub mod hero;
pub mod mouse;

use crate::gfx::{Canvas, H, Raster, W, bayer, palette as pal};
use crate::session::{Fx, FxColor, Session, Skin};
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
            cheese(r, cx, cy + bob);
        } else {
            // Eaten: crumbs where it was, until it's back.
            r.put(cx - 2, cy + 2, pal::CHEESE_DK);
            r.put(cx + 1, cy + 3, pal::CHEESE_DK);
        }
    }
    let rumble = s.world.straining();
    for (i, b) in s.world.blocks.iter().enumerate() {
        let mut q = b.rect();
        if rumble == Some(i) {
            // Straining against it: it shudders a pixel each way and sheds a little dust.
            q.x += [0, 1, 0, -1][(s.world.tick / 3 % 4) as usize];
            if s.world.tick % 6 < 3 {
                r.put(q.x - 1, q.bottom() - 1, pal::PALE);
                r.put(q.right(), q.bottom() - 2, pal::PALE);
            }
        }
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
        effect(r, f, s.skin);
    }
    let (x, y) = s.hero_pos();
    let p = &s.world.player;
    let spent = p.dashes == 0 && s.world.tuning.dashes > 0;
    // The rig was simulated at the step's position; she's drawn between steps.
    let (sx, sy) = p.exact_pos();
    let shift = (x - sx, y - sy);
    let sprite = s.animator.sprite();
    match s.skin {
        Skin::Mouse => mouse::draw(
            r,
            x,
            y,
            sprite,
            p.facing,
            Some(&s.rig),
            shift,
            spent,
            None,
            1.0,
        ),
        Skin::Blocks => {
            let tint = spent.then_some(pal::STEEL_LT);
            hero::draw(r, x, y, sprite, p.facing, tint, 1.0);
        }
    }
    debug::draw(r, s);
}

/// A wedge of cheese centered near `cx, cy`, holes and all.
fn cheese(r: &mut dyn Raster, cx: i32, cy: i32) {
    for row in 0..5 {
        let w = 2 + row;
        r.span(cx - 3, cx - 3 + w + 1, cy - 2 + row, pal::CHEESE, 1.0);
    }
    r.span(cx - 3, cx + 4, cy + 3, pal::CHEESE_DK, 1.0);
    r.put(cx - 1, cy, pal::CHEESE_DK);
    r.put(cx + 1, cy + 2, pal::CHEESE_DK);
}

/// A 5x4 heart with its top-left at `x - 2, y`.
fn heart(r: &mut dyn Raster, x: i32, y: i32) {
    r.put(x - 1, y, pal::RED);
    r.put(x + 1, y, pal::RED);
    r.span(x - 2, x + 3, y + 1, pal::RED, 1.0);
    r.span(x - 1, x + 2, y + 2, pal::RED, 1.0);
    r.put(x, y + 3, pal::RED);
}

/// Draw one effect.
fn effect(r: &mut dyn Raster, f: &Fx, skin: Skin) {
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
            let a = 0.5 * (1.0 - k);
            match skin {
                Skin::Mouse => {
                    let c = Some(pal::CYAN);
                    mouse::draw(r, x, y, sprite, facing, None, (0.0, 0.0), false, c, a);
                }
                Skin::Blocks => hero::draw(r, x, y, sprite, facing, Some(pal::CYAN), a),
            }
        }
        Fx::Heart { x, y, .. } => {
            let (hx, hy) = (x as i32, (y - k * 8.0) as i32);
            if 1.0 - k > bayer(hx, hy) {
                heart(r, hx, hy);
            }
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
