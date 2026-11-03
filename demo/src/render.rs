//! Draws a [`Session`]: the night and the level into a cached backdrop [`Canvas`]
//! ([`backdrop`]), everything that moves (the heroine, cheese, crates, effects) and the
//! debug overlay into any [`Raster`].
//!
//! The simulation works in units (8 per tile); the screen has [`K`] pixels per unit, and
//! everything here scales positions by `K`. At 1 the art has exactly the physics' pixels
//! (and, as it turned out, the most charm); 2 doubles the detail if ever wanted (the
//! grid in `gfx` has to double with it). The overlay's text stays at one pixel per pixel.

pub mod backdrop;
pub mod debug;
pub mod hero;
pub mod mouse;

pub use backdrop::{Twinkle, backdrop, plain, twinkles};
pub use mouse::Paint;

use crate::gfx::{H, Raster, W, bayer, palette as pal};
use crate::session::{Fx, FxColor, Session, Skin, Wind};
use crate::sim::{Mode, Rect, TILE, player};

/// Screen pixels per simulation unit.
pub const K: i32 = 1;
/// [`K`] as a float.
pub const KF: f32 = K as f32;

/// A simulation rectangle in screen pixels.
fn px(r: Rect) -> Rect {
    Rect {
        x: r.x * K,
        y: r.y * K,
        w: r.w * K,
        h: r.h * K,
    }
}

/// Draw everything that moves: twinkling `stars` (from [`twinkles`]), cheese, crates,
/// effects, the heroine, the overlay.
pub fn scene(r: &mut dyn Raster, s: &Session, stars: &[Twinkle], clock: f32) {
    if s.skin == Skin::Mouse {
        twinkle(r, stars, clock);
    }
    for c in &s.level.crystals {
        let a = px(c.area);
        let (cx, cy) = (a.x + TILE * K / 2, a.y + TILE * K / 2);
        let bob = ((clock * 3.0).sin() * 2.0).round() as i32;
        if c.ready() {
            cheese(r, cx, cy + bob, clock);
        } else {
            // Eaten: crumbs where it was, until it's back.
            for (dx, dy) in [(-4, 5), (2, 6), (-1, 7)] {
                r.put(cx + dx, cy + dy, pal::CHEESE_DK);
            }
        }
    }
    let rumble = s.world.straining();
    for (i, b) in s.level.blocks.iter().enumerate() {
        let mut q = px(b.rect());
        if rumble == Some(i) {
            // Straining against it: it shudders a pixel each way and sheds a little dust.
            q.x += [0, 1, 0, -1][(s.world.tick / 3 % 4) as usize];
            if s.world.tick % 6 < 3 {
                r.put(q.x - 2, q.bottom() - 1, pal::PALE);
                r.put(q.right() + 1, q.bottom() - 3, pal::PALE);
            }
        }
        crate_box(r, q);
    }
    if s.skin == Skin::Mouse {
        gusts(r, s);
        plants(r, s, true);
    }
    for f in &s.fx {
        effect(r, f, s);
    }
    let (x, y) = s.hero_pos();
    let p = &s.world.player;
    let spent = p.dashes == 0 && s.world.tuning.dashes > 0;
    // The rig was simulated at the step's position; she's drawn between steps.
    let (sx, sy) = p.exact_pos();
    let shift = (x - sx, y - sy);
    let sprite = s.animator.sprite();
    match s.skin {
        Skin::Mouse => {
            let charge = charge(r, s, (x, y));
            // Fully charged: she flashes white, three frames in eight, until released.
            let flash = charge >= 1.0 && s.world.tick % 8 < 3;
            // Charging, crystal light seeps into her as the charge builds.
            let glow = (charge > 0.0).then_some((pal::CRYSTAL, 0.15 + charge * 0.45));
            let wind = Wind::at(s.world.tick, sy);
            let paint = Paint {
                t: s.world.tick as f32 / 120.0,
                flutter: 1.0 + wind.gust * 0.5,
                spent,
                tint: flash.then_some(pal::WHITE),
                glow,
                ..Paint::default()
            };
            mouse::draw(r, x, y, sprite, p.facing, Some((&s.rig, shift)), &paint);
        }
        Skin::Blocks => {
            let tint = spent.then_some(pal::STEEL_LT);
            hero::draw(r, x, y, sprite, p.facing, tint, 1.0);
        }
    }
    if s.skin == Skin::Mouse {
        // Grass in front of her feet.
        plants(r, s, false);
    }
    debug::draw(r, s);
}

/// Moss (`hanging`) or grass: a stalk from the root to the tip, curled a bit more toward
/// the end, with a lighter leaf or tip pixel.
fn plants(r: &mut dyn Raster, s: &Session, hanging: bool) {
    let (stalk, leaf) = if hanging {
        (pal::MOSS, pal::MOSS_LT)
    } else {
        (pal::GRASS, pal::GRASS_LT)
    };
    for sp in s.foliage.sprigs.iter().filter(|sp| sp.hangs == hanging) {
        let up = if hanging { 1.0 } else { -1.0 };
        // Two segments: the outer one bends further, so a swaying strand curls.
        let half = sp.len / 2.0;
        let (s1, c1) = (sp.angle * 0.6).sin_cos();
        let mid = (sp.x + s1 * half, sp.y + up * c1 * half);
        let (s2, c2) = (sp.angle * 1.4).sin_cos();
        let tip = (mid.0 + s2 * half, mid.1 + up * c2 * half);
        let p = |q: (f32, f32)| ((q.0 * KF) as i32, (q.1 * KF) as i32);
        let (a, b, c) = (p((sp.x, sp.y)), p(mid), p(tip));
        r.line(a.0, a.1, b.0, b.1, stalk);
        r.line(b.0, b.1, c.0, c.1, stalk);
        r.put(c.0, c.1, leaf);
        if hanging && sp.len > 6.0 {
            r.put(b.0 + 1, b.1, leaf);
        }
    }
}

/// The super dash's charge, drawn around the hero at `pos` (units): crystal shards
/// spiral in as it builds, and once full a ring pulses around her. Returns how full it is
/// (0 when not charging).
fn charge(r: &mut dyn Raster, s: &Session, pos: (f32, f32)) -> f32 {
    let Mode::SuperCharge { t, .. } = s.world.player.mode else {
        return 0.0;
    };
    let full = (t / s.world.tuning.super_charge).min(1.0);
    let (cx, cy) = (
        (pos.0 + player::W as f32 / 2.0) * KF,
        (pos.1 + player::H as f32 / 2.0) * KF,
    );
    let spin = s.world.tick as f32 / 120.0 * 5.0;
    let reach = (16.0 - 11.0 * full) * KF;
    for i in 0..10 {
        let a = i as f32 / 10.0 * std::f32::consts::TAU + spin;
        let (px, py) = (
            (cx + a.cos() * reach) as i32,
            (cy + a.sin() * reach * 0.8) as i32,
        );
        if 0.3 + full > bayer(px, py) {
            r.put(px, py, pal::CRYSTAL);
            r.put(px + 1, py, pal::CRYSTAL);
            r.put(px, py - 1, pal::CRYSTAL_LT);
            r.put(px + 1, py - 1, pal::CRYSTAL_LT);
        }
    }
    if full >= 1.0 {
        let pulse = (7.0 + (s.world.tick as f32 * 0.25).sin() * 1.5) * KF;
        let n = (pulse * 10.0) as i32;
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            let (px, py) = (
                (cx + a.cos() * pulse * 1.2) as i32,
                (cy + a.sin() * pulse * 1.4) as i32,
            );
            r.put(
                px,
                py,
                if i % 3 == 0 {
                    pal::WHITE
                } else {
                    pal::CRYSTAL_LT
                },
            );
        }
    }
    full
}

/// Streaks of air blowing across the sky in a gust, thicker the stronger it is; from the
/// moon's side, right to left, like the breeze.
fn gusts(r: &mut dyn Raster, s: &Session) {
    let wind = Wind::at(s.world.tick, s.world.player.y as f32);
    if wind.gust <= 0.05 {
        return;
    }
    let t = s.world.tick as f32 / 120.0;
    for i in 0..8 {
        let row = (backdrop::hash(i, 90) % (H as u32 * 2 / 3)) as i32;
        let speed = 160.0 + (backdrop::hash(i, 91) % 120) as f32;
        let len = 4 + (backdrop::hash(i, 92) % 8) as i32;
        let span = (W + 40) as f32;
        let x = (W as f32 - (t * speed + (backdrop::hash(i, 93) % 400) as f32) % span) as i32;
        let wobble = ((t * 3.0 + i as f32).sin() * 1.5) as i32;
        // Translucent, so they read as moving air rather than dots or lines.
        r.span(x, x + len * K, row + wobble, pal::STAR, 0.18 * wind.gust);
    }
}

/// Stars brighten and dim, each on its own slow rhythm; near the top of its swell a bright
/// one shows a faint cross of glints. Subtle: a fraction of a pixel's worth either way.
fn twinkle(r: &mut dyn Raster, stars: &[Twinkle], clock: f32) {
    for st in stars {
        let v = (clock * st.speed + st.phase).sin();
        if v > 0.55 {
            let a = (v - 0.55) / 0.45 * (0.25 + 0.35 * st.bright);
            r.blend(st.x, st.y, pal::WHITE, a);
            if v > 0.93 && st.bright > 0.85 {
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    r.blend(st.x + dx, st.y + dy, pal::STAR, a * 0.6);
                }
            }
        } else if v < -0.6 {
            r.blend(st.x, st.y, pal::SKY_1, (-v - 0.6) / 0.4 * 0.45);
        }
    }
}

/// A wooden crate in `q` (screen pixels): a frame, planks, an X brace, a lit top edge.
fn crate_box(r: &mut dyn Raster, q: Rect) {
    r.fill_rect(q.x, q.y, q.w, q.h, pal::WOOD);
    for k in 1..4 {
        let y = q.y + k * q.h / 4;
        r.span(q.x + 2, q.right() - 2, y, pal::WOOD_DK, 0.6);
    }
    for (a, b) in [
        ((q.x + 2, q.y + 2), (q.right() - 3, q.bottom() - 3)),
        ((q.right() - 3, q.y + 2), (q.x + 2, q.bottom() - 3)),
    ] {
        r.line(a.0, a.1, b.0, b.1, pal::WOOD_DK);
        r.line(a.0 + 1, a.1, b.0 + 1, b.1, pal::WOOD_DK);
    }
    r.rect_outline(q.x, q.y, q.w, q.h, pal::WOOD_DK);
    r.rect_outline(q.x + 1, q.y + 1, q.w - 2, q.h - 2, pal::WOOD_DK);
    r.span(q.x + 2, q.right() - 2, q.y + 2, pal::WOOD_LT, 1.0);
    for (x, y) in [
        (q.x + 2, q.y + 2),
        (q.right() - 3, q.y + 2),
        (q.x + 2, q.bottom() - 3),
        (q.right() - 3, q.bottom() - 3),
    ] {
        r.put(x, y, pal::NAIL);
    }
}

/// A wedge of cheese centered near `cx, cy`, holes and all, with a glint that comes and
/// goes; drawn for `K`.
fn cheese(r: &mut dyn Raster, cx: i32, cy: i32, clock: f32) {
    if K == 1 {
        for row in 0..4 {
            r.span(
                cx - 3,
                cx - 1 + row * 3 / 2 + 1,
                cy - 2 + row,
                pal::CHEESE,
                1.0,
            );
        }
        r.span(cx - 3, cx + 3, cy + 2, pal::CHEESE_DK, 1.0);
        r.put(cx - 1, cy, pal::CHEESE_DK);
        if (clock * 2.0).fract() < 0.15 {
            r.put(cx - 3, cy - 2, pal::WHITE);
        }
        return;
    }
    for row in 0..9 {
        let w = 3 + row * 3 / 2;
        r.span(cx - 6, cx - 6 + w + 1, cy - 4 + row, pal::CHEESE, 1.0);
    }
    r.span(cx - 6, cx + 8, cy + 5, pal::CHEESE_DK, 1.0);
    r.span(cx - 6, cx + 8, cy + 6, pal::CHEESE_DK, 1.0);
    r.fill_circle(cx - 2, cy + 1, 1, pal::CHEESE_DK);
    r.put(cx + 3, cy + 3, pal::CHEESE_DK);
    r.put(cx - 4, cy + 4, pal::CHEESE_DK);
    if (clock * 2.0).fract() < 0.15 {
        r.put(cx - 5, cy - 3, pal::WHITE);
        r.put(cx - 4, cy - 4, pal::WHITE);
    }
}

/// A 7x6 heart with its top centered at `x, y`.
fn heart(r: &mut dyn Raster, x: i32, y: i32) {
    r.span(x - 3, x - 1, y, pal::RED, 1.0);
    r.span(x + 1, x + 3, y, pal::RED, 1.0);
    r.span(x - 3, x + 4, y + 1, pal::RED, 1.0);
    r.span(x - 3, x + 4, y + 2, pal::RED, 1.0);
    r.span(x - 2, x + 3, y + 3, pal::RED, 1.0);
    r.span(x - 1, x + 2, y + 4, pal::RED, 1.0);
    r.put(x, y + 5, pal::RED);
    r.put(x - 2, y + 1, pal::WHITE);
}

/// Draw one effect. Positions and sizes are in units; drawn at `K`.
fn effect(r: &mut dyn Raster, f: &Fx, s: &Session) {
    let k = f.progress();
    match *f {
        Fx::Ring {
            x, y, r: rad, kind, ..
        } => {
            let c = match kind {
                FxColor::Recharge => pal::CHEESE,
                FxColor::Air => pal::PALE,
                FxColor::Impact => pal::YELLOW,
            };
            let rr = rad * k * KF;
            let n = (rr * 6.0).max(8.0) as i32;
            for i in 0..n {
                let a = i as f32 / n as f32 * std::f32::consts::TAU;
                let (px, py) = (
                    (x * KF + a.cos() * rr) as i32,
                    (y * KF + a.sin() * rr) as i32,
                );
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
            match s.skin {
                Skin::Mouse => {
                    let paint = Paint {
                        tint: Some(pal::CYAN),
                        alpha: a,
                        ..Paint::default()
                    };
                    mouse::draw(r, x, y, sprite, facing, None, &paint);
                }
                Skin::Blocks => hero::draw(r, x, y, sprite, facing, Some(pal::CYAN), a),
            }
        }
        Fx::Heart { x, y, .. } => {
            let (hx, hy) = ((x * KF) as i32, ((y - k * 8.0) * KF) as i32);
            if 1.0 - k > bayer(hx, hy) {
                heart(r, hx, hy);
            }
        }
        Fx::Dust { x, y, spread, .. } => {
            let d = spread * k * KF;
            for side in [-1.0, 1.0] {
                for j in 0..4 {
                    let px = (x * KF + side * (d - j as f32 * 3.0)) as i32;
                    let py = (y * KF - 1.0 - j as f32 * k * 4.0) as i32;
                    if 1.0 - k > bayer(px, py) {
                        r.put(px, py, pal::PALE);
                        r.put(px + 1, py, pal::PALE);
                    }
                }
            }
        }
    }
}
