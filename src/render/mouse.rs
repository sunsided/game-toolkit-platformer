//! The heroine of Cheesed: a mouse with a blonde mane, a red headband and a torn red cape,
//! after the character sheet, at 12 pixels tall.
//!
//! She is a cut-out puppet: boots, tunic, arm, head, snout, ears and headband are placed in
//! the body's [`Frame`] for the current sprite, so every pose's squash, stretch and lean
//! carry over; the cape, tail, mane and headband ends come from the [`Rig`]'s simulated
//! chains. Back to front: cape, tail, far ear, mane, legs, tunic, arm, head, near ear,
//! headband, eye.

use crate::gfx::{DrawList, Raster, Rgb, bayer, palette as pal};
use crate::hero::pose::pose;
use crate::hero::rig::{Frame, Pt, Rig, hand, stride};

/// Draw the mouse for `sprite`, hitbox top-left `x, y`, facing `facing`. `rig` holds her
/// cape and tail (shifted by `shift` from where they were simulated to where she is
/// drawn); without one (afterimages), she's drawn without them. With her dash `spent`, the
/// mane turns pale blue. `tint` paints all of her in one color; `alpha` < 1 dithers her
/// away.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    r: &mut dyn Raster,
    x: f32,
    y: f32,
    sprite: u16,
    facing: i32,
    rig: Option<&Rig>,
    shift: Pt,
    spent: bool,
    tint: Option<Rgb>,
    alpha: f32,
) {
    let (hair, hair_dk) = if spent {
        (pal::MOUSE_HAIR_SPENT, pal::MOUSE_HAIR_SPENT_DK)
    } else {
        (pal::MOUSE_HAIR, pal::MOUSE_HAIR_DK)
    };
    let fr = Frame::new(x, y, sprite, facing);
    let mut d = DrawList::default();
    let moved = |p: &Pt| (p.0 + shift.0, p.1 + shift.1);

    if let Some(rig) = rig {
        let cape: Vec<Pt> = rig.cape.points.iter().map(moved).collect();
        strip(&mut d, &cape, 3.0, 5.5, pal::MOUSE_RED, pal::MOUSE_RED_DK);
        // The tail comes out from under the cape.
        let tail: Vec<Pt> = rig.tail.points.iter().map(moved).collect();
        polyline(&mut d, &tail, pal::MOUSE_PINK);
    }
    if let Some(spread) = pose(sprite).wings.filter(|_| !is_glide(sprite)) {
        wings(&mut d, &fr, spread);
    }

    // Far ear, then the mane behind the head.
    let k = fr.scale();
    disc(&mut d, fr.at(-2.4, -7.4), 2.3 * k, pal::MOUSE_FUR_DK);
    disc(&mut d, fr.at(-2.3, -7.3), 1.5 * k, pal::MOUSE_PINK_DK);
    disc(&mut d, fr.at(-1.6, -4.4), 2.0 * k, hair_dk);
    if let Some(rig) = rig {
        let hair_pts: Vec<Pt> = rig.hair.points.iter().map(moved).collect();
        strip(&mut d, &hair_pts, 3.2, 1.2, hair, hair_dk);
    }

    // Legs and boots, a stride apart when walking.
    let st = stride(sprite);
    quad(&mut d, &fr, (-1.8, 2.6), (1.8, 4.6), pal::MOUSE_FUR_DK);
    quad(
        &mut d,
        &fr,
        (-0.6 - st - 1.2, 4.4),
        (-0.6 - st + 0.6, 6.0),
        pal::MOUSE_LEATHER,
    );
    quad(
        &mut d,
        &fr,
        (0.6 + st - 0.6, 4.4),
        (0.6 + st + 1.2, 6.0),
        pal::MOUSE_LEATHER,
    );

    // Tunic and belt.
    quad(&mut d, &fr, (-2.0, -1.4), (2.0, 3.0), pal::MOUSE_TUNIC);
    quad(&mut d, &fr, (-2.0, 1.0), (2.0, 1.8), pal::MOUSE_LEATHER);
    pixel(&mut d, fr.at(0.8, 1.4), pal::MOUSE_METAL);

    // The near arm, from the shoulder to wherever the pose puts the hand.
    let (hu, hv) = hand(sprite);
    line(&mut d, fr.at(0.4, -0.8), fr.at(hu, hv), pal::MOUSE_FUR);
    pixel(&mut d, fr.at(hu, hv), pal::MOUSE_LEATHER);

    // Head and snout, the near ear, the headband, the eye and nose.
    disc(&mut d, fr.at(0.4, -3.4), 2.5 * k, pal::MOUSE_FUR);
    disc(&mut d, fr.at(2.3, -2.9), 1.2 * k, pal::MOUSE_FUR);
    pixel(&mut d, fr.at(3.4, -3.0), pal::MOUSE_PINK_DK);
    disc(&mut d, fr.at(1.0, -7.8), 2.5 * k, pal::MOUSE_FUR);
    disc(&mut d, fr.at(1.1, -7.7), 1.7 * k, pal::MOUSE_PINK);
    line(&mut d, fr.at(-2.0, -4.9), fr.at(2.4, -4.6), pal::MOUSE_RED);
    pixel(&mut d, fr.at(1.4, -3.8), pal::MOUSE_EYE);
    if let Some(rig) = rig {
        let band: Vec<Pt> = rig.band.points.iter().map(moved).collect();
        polyline(&mut d, &band, pal::MOUSE_RED);
    }
    // A wisp of mane over the brow.
    pixel(&mut d, fr.at(-0.2, -5.6), hair);
    pixel(&mut d, fr.at(0.8, -5.5), hair);

    for s in &d.spans {
        let c = tint.unwrap_or(s.c);
        if alpha >= 1.0 {
            r.span(s.x0, s.x1, s.y, c, s.a);
        } else {
            for px in s.x0..s.x1 {
                if alpha > bayer(px, s.y) {
                    r.put(px, s.y, c);
                }
            }
        }
    }
}

/// Whether `sprite` is a glide frame (the cape does the gliding, not wings).
fn is_glide(sprite: u16) -> bool {
    crate::hero::sprite::GLIDE.contains(&sprite)
}

/// A filled disc with a fractional center and radius, sampled at pixel centers.
fn disc(d: &mut DrawList, (cx, cy): Pt, rad: f32, c: Rgb) {
    let rad = rad.max(0.5);
    let (y0, y1) = ((cy - rad).floor() as i32, (cy + rad).ceil() as i32);
    for y in y0..=y1 {
        let dy = y as f32 + 0.5 - cy;
        let h2 = rad * rad - dy * dy;
        if h2 < 0.0 {
            continue;
        }
        let h = h2.sqrt();
        let (a, b) = ((cx - h - 0.5).ceil() as i32, (cx + h - 0.5).floor() as i32);
        if a <= b {
            d.span(a, b + 1, y, c, 1.0);
        }
    }
}

/// The rectangle between local corners `a` and `b`, leaning with the frame.
fn quad(d: &mut DrawList, fr: &Frame, a: Pt, b: Pt, c: Rgb) {
    let pts = [
        fr.at(a.0, a.1),
        fr.at(b.0, a.1),
        fr.at(b.0, b.1),
        fr.at(a.0, b.1),
    ];
    d.fill_polygon(&pts, c);
}

/// One pixel at a fractional point.
fn pixel(d: &mut DrawList, (x, y): Pt, c: Rgb) {
    d.put(x.floor() as i32, y.floor() as i32, c);
}

/// A 1 px line between fractional points.
fn line(d: &mut DrawList, a: Pt, b: Pt, c: Rgb) {
    d.line(
        a.0.floor() as i32,
        a.1.floor() as i32,
        b.0.floor() as i32,
        b.1.floor() as i32,
        c,
    );
}

/// A 1 px line through `pts`.
fn polyline(d: &mut DrawList, pts: &[Pt], c: Rgb) {
    for w in pts.windows(2) {
        line(d, w[0], w[1], c);
    }
}

/// A ribbon through `pts`, `w0` wide at the start and `w1` at the end, in `c`, with its
/// last stretch in `edge` (a cape's torn, darker hem).
fn strip(d: &mut DrawList, pts: &[Pt], w0: f32, w1: f32, c: Rgb, edge: Rgb) {
    let n = pts.len();
    if n < 2 {
        return;
    }
    let normal = |i: usize| {
        let (a, b) = (pts[i.saturating_sub(1)], pts[(i + 1).min(n - 1)]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = (dx * dx + dy * dy).sqrt().max(1e-4);
        (-dy / l, dx / l)
    };
    for i in 0..n - 1 {
        let w = |k: usize| (w0 + (w1 - w0) * k as f32 / (n - 1) as f32) / 2.0;
        let (na, nb) = (normal(i), normal(i + 1));
        let (a, b) = (pts[i], pts[i + 1]);
        let (wa, wb) = (w(i), w(i + 1));
        let quad = [
            (a.0 + na.0 * wa, a.1 + na.1 * wa),
            (b.0 + nb.0 * wb, b.1 + nb.1 * wb),
            (b.0 - nb.0 * wb, b.1 - nb.1 * wb),
            (a.0 - na.0 * wa, a.1 - na.1 * wa),
        ];
        d.fill_polygon(&quad, if i + 2 >= n { edge } else { c });
    }
}

/// Monarch-style wings from the shoulders, spread `spread` degrees back from straight up.
fn wings(d: &mut DrawList, fr: &Frame, spread: f32) {
    let base = fr.at(-1.0, -1.6);
    for k in [0.0f32, 0.35] {
        let a = (-90.0 - fr.f * spread * (1.0 - k)).to_radians();
        let tip = (base.0 + a.cos() * 9.0, base.1 + a.sin() * 9.0);
        let a2 = a + fr.f * 0.6;
        let mid = (base.0 + a2.cos() * 6.0, base.1 + a2.sin() * 6.0);
        d.fill_polygon(&[base, tip, mid], pal::WHITE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::{Canvas, H, W};
    use crate::hero::sprite::COUNT;

    #[test]
    fn every_sprite_draws_something_near_the_hitbox() {
        let mut rig = Rig::default();
        rig.step(100.0, 100.0, 0, 1, (0.0, 0.0), 1.0 / 120.0);
        for sprite in 0..COUNT {
            let mut c = Canvas::new(W, H);
            draw(
                &mut c,
                100.0,
                100.0,
                sprite,
                1,
                Some(&rig),
                (0.0, 0.0),
                false,
                None,
                1.0,
            );
            let lit: Vec<(i32, i32)> = (0..H)
                .flat_map(|y| (0..W).map(move |x| (x, y)))
                .filter(|&(x, y)| c.get(x, y) != Rgb(0, 0, 0))
                .collect();
            assert!(lit.len() > 40, "sprite {sprite}: {} pixels", lit.len());
            assert!(
                lit.iter()
                    .all(|&(x, y)| (80..126).contains(&x) && (80..126).contains(&y)),
                "sprite {sprite} stays near the hero"
            );
        }
    }
}
