//! The backdrop: a moonlit night after the Cheesed paintings (a violet sky with stars and
//! clouds, a big pale moon, snowy mountains, a castle with lit windows on a hill, dark
//! firs and mist), with the level's tiles on top: grey rock with a warm rim light on
//! exposed tops, and wooden planks for one-way platforms.
//!
//! Drawn once into a [`Canvas`] at [`K`] pixels per simulation unit. Everything that looks
//! random comes from [`hash`], so the picture is the same every run.

use super::K;
use crate::gfx::{Canvas, H, Raster, Rgb, W, bayer, palette as pal};
use crate::sim::{Level, TILE, Tile};

/// A small integer hash: the backdrop's dice.
pub fn hash(a: i32, b: i32) -> u32 {
    let mut h = (a as u32).wrapping_mul(0x9e37_79b1) ^ (b as u32).wrapping_mul(0x85eb_ca77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^ (h >> 15)
}

/// A hash as a fraction, 0..1.
fn rand(a: i32, b: i32) -> f32 {
    (hash(a, b) % 10_000) as f32 / 10_000.0
}

/// A length from the 768-pixel-wide layout the scenery was composed at, at this grid's
/// width (never below 1).
fn s(v: i32) -> i32 {
    (v * W / 768).max(1)
}

/// Where the moon is, and how big, in the 768-wide layout.
const MOON: (i32, i32, i32) = (684, 84, 30);

/// Draw the night and the level into a new canvas.
pub fn backdrop(level: &Level) -> Canvas {
    let mut c = Canvas::new(W, H);
    sky(&mut c);
    stars(&mut c);
    moon(&mut c);
    clouds(&mut c);
    ridge(
        &mut c,
        7,
        s(250),
        s(70),
        pal::NIGHT_FAR,
        Some(pal::NIGHT_SNOW),
    );
    castle(&mut c);
    ridge(&mut c, 11, s(310), s(40), pal::NIGHT_NEAR, None);
    mist(&mut c, s(330), s(380));
    firs(&mut c);
    tiles(&mut c, level);
    c
}

/// The diagnostic backdrop (F3): a plain dithered sky and flat tiles, nothing to look at
/// but the level.
pub fn plain(level: &Level) -> Canvas {
    let mut c = Canvas::new(W, H);
    for y in 0..H {
        let t = y as f32 / H as f32;
        for x in 0..W {
            let band = (t * 6.0).floor() / 6.0;
            let up = if (t * 6.0).fract() > bayer(x, y) {
                1.0 / 6.0
            } else {
                0.0
            };
            c.put(x, y, pal::SKY_TOP.lerp(pal::SKY_BOTTOM, band + up));
        }
    }
    let t = TILE * K;
    for ty in 0..level.h {
        for tx in 0..level.w {
            let (x, y) = (tx * t, ty * t);
            match level.tile(tx, ty) {
                Tile::Empty => {}
                Tile::Solid => {
                    c.fill_rect(x, y, t, t, pal::STONE);
                    c.fill_rect(x, y + t - 2, t, 2, pal::STONE_DK);
                    if !level.solid(tx, ty - 1) {
                        c.fill_rect(x, y, t, 1, pal::STONE_LT);
                    }
                    if !level.solid(tx - 1, ty) {
                        c.fill_rect(x, y, 1, t, pal::STONE_DK);
                    }
                }
                Tile::Platform => {
                    c.fill_rect(x, y, t, 1, pal::STONE_LT);
                    c.fill_rect(x, y + 1, t, 2, pal::STONE);
                }
            }
        }
    }
    c
}

/// The sky: violet at the top, warmer toward the horizon, in dithered bands.
fn sky(c: &mut Canvas) {
    let stops = [pal::SKY_0, pal::SKY_1, pal::SKY_2, pal::SKY_3];
    for y in 0..H {
        let t = y as f32 / H as f32 * (stops.len() - 1) as f32;
        let (i, f) = ((t as usize).min(stops.len() - 2), t.fract());
        for x in 0..W {
            // Eight dithered steps between each pair of stops.
            let up = if (f * 8.0).fract() > bayer(x, y) {
                1.0
            } else {
                0.0
            };
            let step = ((f * 8.0).floor() + up) / 8.0;
            c.put(x, y, stops[i].lerp(stops[i + 1], step));
        }
    }
}

/// Stars, denser and brighter toward the top; the brightest with a cross of glints.
fn stars(c: &mut Canvas) {
    for i in 0..s(520) {
        let x = (hash(i, 1) % W as u32) as i32;
        let y = (rand(i, 2).powf(1.6) * H as f32 * 0.7) as i32;
        let b = rand(i, 3);
        let col = if b > 0.93 {
            pal::WHITE
        } else {
            pal::STAR.lerp(pal::SKY_1, 0.6 - b * 0.6)
        };
        c.put(x, y, col);
        if b > 0.98 {
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                c.blend(x + dx, y + dy, pal::STAR, 0.5);
            }
        }
    }
}

/// The moon: a dithered halo, the disc shaded toward its lower right, and craters.
fn moon(c: &mut Canvas) {
    let (mx, my, r) = (s(MOON.0), s(MOON.1), s(MOON.2));
    for dy in -r * 3..=r * 3 {
        for dx in -r * 3..=r * 3 {
            let d = ((dx * dx + dy * dy) as f32).sqrt() / r as f32;
            if (1.0..3.0).contains(&d) {
                let glow = (1.0 - (d - 1.0) / 2.0).powi(2) * 0.55;
                if glow > bayer(mx + dx, my + dy) {
                    let p = c.get(mx + dx, my + dy);
                    c.put(mx + dx, my + dy, p.lerp(pal::MOON_GLOW, 0.35));
                }
            }
        }
    }
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r * r {
                let shade = ((dx + dy) as f32 / (2.0 * r as f32) + 0.5).clamp(0.0, 1.0);
                c.put(mx + dx, my + dy, pal::MOON.lerp(pal::MOON_DK, shade * 0.55));
            }
        }
    }
    for i in 0..10 {
        let a = rand(i, 11) * std::f32::consts::TAU;
        let d = rand(i, 12).sqrt() * (r - 3).max(1) as f32;
        let (cx, cy) = (mx + (a.cos() * d) as i32, my + (a.sin() * d) as i32);
        let cr = (rand(i, 13) * s(4) as f32) as i32;
        c.blend_circle(cx, cy, cr, pal::MOON_CRATER, 0.8);
        c.put(cx - 1, cy - 1, pal::MOON_DK);
    }
}

/// Purple cloud banks, lit from the moon's side.
fn clouds(c: &mut Canvas) {
    let banks = [
        (80, 70, 180),
        (300, 40, 140),
        (520, 140, 230),
        (470, 100, 120),
        (200, 170, 200),
    ];
    for (k, &(cx, cy, w)) in banks.iter().enumerate() {
        let k = k as i32;
        let (cx, cy, w, step) = (s(cx), s(cy), s(w), s(12));
        for i in 0..w / step {
            let x = cx - w / 2 + i * step + (rand(k, i) * s(8) as f32) as i32;
            let y = cy + (rand(k, i + 50) * s(10) as f32) as i32 - s(5);
            let r = s(8) + (rand(k, i + 100) * s(10) as f32) as i32;
            c.blend_circle(x, y, r, pal::CLOUD, 0.55);
            c.blend_circle(x - 1, y - s(3), (r - s(4)).max(1), pal::CLOUD_LT, 0.35);
        }
    }
}

/// A mountain ridge across the screen: peaks from a hash, filled down to the bottom, with
/// snow on the highest parts if given.
fn ridge(c: &mut Canvas, seed: i32, base: i32, height: i32, col: Rgb, snow: Option<Rgb>) {
    let peaks: Vec<(i32, i32)> = (0..=12)
        .map(|i| {
            let x = i * W / 12 + (rand(seed, i) * s(30) as f32) as i32 - s(15);
            let y = base - (rand(seed, i + 30) * height as f32) as i32;
            (x, y)
        })
        .collect();
    for x in 0..W {
        let i = peaks
            .iter()
            .rposition(|p| p.0 <= x)
            .unwrap_or(0)
            .min(peaks.len() - 2);
        let (a, b) = (peaks[i], peaks[i + 1]);
        let t = (x - a.0) as f32 / (b.0 - a.0).max(1) as f32;
        let jag = (rand(seed, x) * s(3) as f32) as i32;
        let top = (a.1 as f32 + (b.1 - a.1) as f32 * t) as i32 + jag;
        for y in top..H {
            c.put(x, y, col);
        }
        if let Some(snow) = snow
            && top < base - height / 2
        {
            let depth = s(6);
            for y in top..top + depth {
                if rand(x, y) > 0.15 + (y - top) as f32 / depth as f32 * 0.7 {
                    c.put(x, y, snow);
                }
            }
        }
    }
}

/// A castle on the far ridge below the moon: towers with pointed roofs, warm windows, a
/// red flag, and a winding path of torches down the hill.
fn castle(c: &mut Canvas) {
    let (bx, by) = (s(MOON.0 - 10), s(214));
    let towers = [
        (-22, 18, 4),
        (-12, 30, 5),
        (0, 40, 6),
        (12, 26, 5),
        (22, 16, 4),
    ];
    for (dx, h, w) in towers {
        let (x, top, h, w) = (bx + s(dx), by - s(h), s(h), s(w).max(2));
        c.fill_rect(x - w / 2, top, w, h + s(30), pal::CASTLE);
        let roof = [
            ((x - w / 2 - 1) as f32, top as f32),
            ((x + w / 2 + 1) as f32, top as f32),
            (x as f32 + 0.5, (top - w - s(3)) as f32),
        ];
        c.fill_polygon(&roof, pal::CASTLE);
        for k in 0..h / s(8) {
            if !hash(dx, k).is_multiple_of(3) {
                c.put(x, top + s(4) + k * s(8), pal::WINDOW);
            }
        }
    }
    c.fill_rect(bx - s(24), by - s(10), s(48), s(40), pal::CASTLE);
    c.line(bx, by - s(46), bx, by - s(54), pal::CASTLE);
    c.fill_rect(bx + 1, by - s(54), s(4).max(2), 1, pal::MOUSE_RED);
    // Torches along a path winding down the hill.
    let mut x = bx as f32;
    for k in 0..12 {
        let y = by + s(22) + k * s(7);
        x += ((k as f32) * 0.9).sin() * s(6) as f32;
        c.put(x as i32, y, pal::WINDOW);
    }
}

/// A band of mist between `y0` and `y1`, thickest in the middle.
fn mist(c: &mut Canvas, y0: i32, y1: i32) {
    for y in y0..y1 {
        let t = (y - y0) as f32 / (y1 - y0) as f32;
        let a = (1.0 - (t * 2.0 - 1.0).abs()) * 0.45;
        for x in 0..W {
            if a > bayer(x, y) {
                let p = c.get(x, y);
                c.put(x, y, p.lerp(pal::MIST, 0.5));
            }
        }
    }
}

/// Dark firs along the bottom of the hills.
fn firs(c: &mut Canvas) {
    let mut x = -4;
    let mut i = 0;
    while x < W + 8 {
        let h = s(26) + (rand(i, 70) * s(40) as f32) as i32;
        let base = s(372) + (rand(i, 71) * s(14) as f32) as i32;
        for tier in 0..4 {
            let ty = base - h + tier * h / 4;
            let half = 1 + tier * s(3) + h / 14;
            let tri = [
                ((x - half) as f32, (ty + h / 3) as f32),
                ((x + half) as f32, (ty + h / 3) as f32),
                (x as f32, ty as f32),
            ];
            c.fill_polygon(&tri, pal::FIR);
        }
        c.fill_rect(x, base, 1, s(8), pal::FIR);
        x += s(7) + (rand(i, 72) * s(12) as f32) as i32;
        i += 1;
    }
    for y in s(380)..H {
        for xx in 0..W {
            c.put(xx, y, pal::FIR);
        }
    }
}

/// The level's tiles at `K` pixels per unit.
fn tiles(c: &mut Canvas, level: &Level) {
    let t = TILE * K;
    for ty in 0..level.h {
        for tx in 0..level.w {
            let (x, y) = (tx * t, ty * t);
            match level.tile(tx, ty) {
                Tile::Empty => {}
                Tile::Solid => rock(c, level, tx, ty, x, y, t),
                Tile::Platform => plank(c, tx, x, y, t),
            }
        }
    }
}

/// One rock tile: speckled grey-violet stone, darker toward open sides and the bottom, a
/// lit rim on an open top with warm, golden flecks (moss in torchlight), and a crack now
/// and then.
fn rock(c: &mut Canvas, level: &Level, tx: i32, ty: i32, x: i32, y: i32, t: i32) {
    // Pixels per 8 of tile, so the details keep their proportions at any `K`.
    let q = (t / 8).max(1);
    let open_up = !level.solid(tx, ty - 1);
    let open_down = !level.solid(tx, ty + 1);
    let open_left = !level.solid(tx - 1, ty);
    let open_right = !level.solid(tx + 1, ty);
    for py in 0..t {
        for px in 0..t {
            let (gx, gy) = (x + px, y + py);
            let n = rand(gx, gy);
            let mut col = pal::ROCK.lerp(pal::ROCK_DK, n * 0.5);
            let edge = (open_left && px < q)
                || (open_right && px >= t - q)
                || (open_down && py >= t - q - q / 2);
            if edge {
                col = col.lerp(pal::ROCK_DK, 0.6);
            }
            c.put(gx, gy, col);
        }
    }
    if hash(tx, ty).is_multiple_of(4) {
        // A crack.
        let cx = x + q + (hash(tx, ty + 9) % (t - 3 * q).max(1) as u32) as i32;
        let cy = y + 2 * q;
        c.line(cx, cy, cx + q, cy + 2 * q, pal::ROCK_DK);
        c.line(cx + q, cy + 2 * q, cx, cy + 4 * q, pal::ROCK_DK);
    }
    if open_up {
        c.fill_rect(x, y, t, q, pal::ROCK_LT);
        for px in 0..t {
            if rand(x + px, y) > 0.7 {
                c.put(x + px, y, pal::GOLD_MOSS);
            }
            if q > 1 && rand(x + px, y + 1) > 0.85 {
                c.put(x + px, y + 1, pal::GOLD_MOSS);
            }
        }
    }
}

/// One wooden plank tile: lit top edge, grain, a dark underside, a nail at each end.
fn plank(c: &mut Canvas, tx: i32, x: i32, y: i32, t: i32) {
    let q = (t / 8).max(1);
    c.fill_rect(x, y, t, 3 * q, pal::PLANK);
    c.fill_rect(x, y, t, 1, pal::PLANK_LT);
    c.fill_rect(x, y + 3 * q - 1, t, 1, pal::PLANK_DK);
    if q > 1 {
        c.line(x + 2, y + 3, x + t - 4, y + 3, pal::PLANK_DK);
    }
    if tx % 2 == 0 {
        c.put(x, y + 1, pal::PLANK_DK);
    }
    c.put(x + q, y + q, pal::NAIL);
    c.put(x + t - q - 1, y + q, pal::NAIL);
}
