//! The heroine of Cheesed: a mouse with a long blonde mane, a red headband, a torn red cape
//! and a sword at her hip, after the character sheet, about 34 pixels tall at `K` = 2.
//!
//! She is a cut-out puppet: boots, legs, tunic, belt, sheathed sword, arm, head, muzzle,
//! ears, headband and eye are placed in the body's [`Frame`] for the current sprite, so
//! every pose's squash, stretch and lean carry over; standing poses plant her feet wide
//! with a hand on the hilt. The cape, tail, three mane strands and headband ends come
//! from the [`Rig`]'s simulated chains, the mane with a slow wave on top.
//!
//! Back to front: cape, back mane, tail, far ear, far leg, sheath, near leg, tunic, arm,
//! head, near ear, bangs, headband, eye.

use super::KF;
use crate::gfx::{DrawList, Raster, Rgb, bayer, palette as pal};
use crate::hero::pose::pose;
#[cfg(test)]
use crate::hero::rig::Air;
use crate::hero::rig::{Frame, Pt, Rig, feet, hand};
use crate::hero::sprite::GLIDE;

/// How to paint her, beyond pose and position.
#[derive(Clone, Copy, Debug)]
pub struct Paint {
    /// Pixels per unit.
    pub k: f32,
    /// Seconds, for the mane's wave.
    pub t: f32,
    /// How hard the mane flutters (1 calm, more in a gust).
    pub flutter: f32,
    /// Her dash is spent: the mane turns pale blue.
    pub spent: bool,
    /// Paint all of her in one color (afterimages, a flash).
    pub tint: Option<Rgb>,
    /// Mix this color into her by this much, dithered (a charge's glow).
    pub glow: Option<(Rgb, f32)>,
    /// Below 1, dither her away.
    pub alpha: f32,
}

impl Default for Paint {
    fn default() -> Self {
        Self {
            k: KF,
            t: 0.0,
            flutter: 1.0,
            spent: false,
            tint: None,
            glow: None,
            alpha: 1.0,
        }
    }
}

/// Draw her for `sprite`, hitbox top-left `x, y` in units, facing `facing`. `rig` holds her
/// cape, tail, mane and headband ends and how far she's drawn from where they were
/// simulated (units); without one (afterimages), she's drawn without them.
pub fn draw(
    r: &mut dyn Raster,
    x: f32,
    y: f32,
    sprite: u16,
    facing: i32,
    rig: Option<(&Rig, Pt)>,
    paint: &Paint,
) {
    let k = paint.k;
    let fr = Frame::new(x, y, sprite, facing).zoom(k);
    let z = fr.scale();
    let (hair, hair_dk) = if paint.spent {
        (pal::MOUSE_HAIR_SPENT, pal::MOUSE_HAIR_SPENT_DK)
    } else {
        (pal::MOUSE_HAIR, pal::MOUSE_HAIR_DK)
    };
    let mut d = DrawList::default();
    // Chain points in pixels, moved to where she's drawn.
    let chain = |pts: &[Pt], shift: Pt| -> Vec<Pt> {
        pts.iter()
            .map(|p| ((p.0 + shift.0) * k, (p.1 + shift.1) * k))
            .collect()
    };

    // Blown forward (a wind at her back), the mane crosses in front of her head, so it's
    // drawn over it rather than hidden behind it.
    let forward = rig.is_some_and(|(rig, _)| {
        let h = &rig.hair[0];
        let (root, tip) = (h.points[0], h.points[h.points.len() - 1]);
        (tip.0 - root.0) * fr.f > 1.0
    });
    if let Some((rig, shift)) = rig {
        let cape = chain(&rig.cape.points, shift);
        strip(
            &mut d,
            &cape,
            3.6 * k,
            9.5 * k,
            pal::MOUSE_RED,
            pal::MOUSE_RED_DK,
        );
        hem(&mut d, &cape, 9.5 * k, pal::MOUSE_RED_DK);
        if !forward {
            mane(&mut d, rig, shift, paint, (hair, hair_dk));
        }
        let tail = chain(&rig.tail.points, shift);
        strip(
            &mut d,
            &tail,
            0.9 * k,
            0.45 * k,
            pal::MOUSE_PINK_DK,
            pal::MOUSE_PINK_DK,
        );
    }
    if let Some(spread) = pose(sprite).wings.filter(|_| !GLIDE.contains(&sprite)) {
        wings(&mut d, &fr, spread, k);
    }

    // The far ear and the mane's mass behind the head.
    disc(&mut d, fr.at(-3.2, -8.6), 2.3 * z, pal::MOUSE_FUR_DK);
    disc(&mut d, fr.at(-3.1, -8.5), 1.5 * z, pal::MOUSE_PINK_DK);
    disc(&mut d, fr.at(-1.6, -5.4), 2.6 * z, hair_dk);

    // Legs: the far one first, the sword's sheath between, the near one in front.
    let (back, front) = feet(sprite);
    leg(&mut d, &fr, back, pal::MOUSE_FUR_DK, pal::MOUSE_LEATHER_DK);
    let drawn = matches!(sprite, 49 | 50);
    sheath(&mut d, &fr, !drawn);
    leg(&mut d, &fr, front, pal::MOUSE_FUR, pal::MOUSE_LEATHER);

    // Tunic: shoulders, waist, a skirt with a ragged hem; belt, chest strap, star brooch.
    let poly = |pts: &[Pt]| -> Vec<Pt> { pts.iter().map(|p| fr.at(p.0, p.1)).collect() };
    d.fill_polygon(
        &poly(&[(-2.0, -2.0), (2.3, -2.0), (1.9, 1.2), (-1.7, 1.2)]),
        pal::MOUSE_TUNIC,
    );
    d.fill_polygon(
        &poly(&[
            (-1.9, 1.0),
            (2.1, 1.0),
            (2.7, 3.3),
            (1.6, 3.0),
            (0.6, 3.5),
            (-0.6, 3.0),
            (-2.5, 3.4),
        ]),
        pal::MOUSE_TUNIC,
    );
    d.fill_polygon(
        &poly(&[(-2.0, -2.0), (-1.2, -2.0), (-1.0, 1.2), (-1.7, 1.2)]),
        pal::MOUSE_TUNIC_DK,
    );
    thick(
        &mut d,
        fr.at(-1.8, 1.1),
        fr.at(2.0, 1.1),
        0.8 * z,
        pal::MOUSE_LEATHER,
    );
    thick(
        &mut d,
        fr.at(-1.6, -1.8),
        fr.at(1.7, 0.9),
        0.5 * z,
        pal::MOUSE_LEATHER_DK,
    );
    disc(&mut d, fr.at(0.9, 1.1), 0.55 * z, pal::MOUSE_METAL);
    disc(&mut d, fr.at(1.0, -1.7), 0.6 * z, pal::MOUSE_METAL);

    // The near arm: shoulder, elbow, a leather bracer, the gloved hand.
    let (hu, hv) = hand(sprite);
    let (su, sv) = (0.6, -1.4);
    let elbow = ((su + hu) / 2.0 - 0.3, (sv + hv) / 2.0 + 0.4);
    thick(
        &mut d,
        fr.at(su, sv),
        fr.at(elbow.0, elbow.1),
        0.9 * z,
        pal::MOUSE_FUR,
    );
    thick(
        &mut d,
        fr.at(elbow.0, elbow.1),
        fr.at(hu, hv),
        1.0 * z,
        pal::MOUSE_LEATHER,
    );
    disc(&mut d, fr.at(hu, hv), 0.65 * z, pal::MOUSE_LEATHER_DK);

    // Head: skull, cheek, a long muzzle with a pink nose, whiskers.
    disc(&mut d, fr.at(0.6, -4.6), 2.7 * z, pal::MOUSE_FUR);
    disc(&mut d, fr.at(1.4, -3.9), 1.5 * z, pal::MOUSE_FUR_LT);
    disc(&mut d, fr.at(2.7, -3.9), 1.3 * z, pal::MOUSE_FUR);
    disc(&mut d, fr.at(3.6, -3.8), 0.8 * z, pal::MOUSE_FUR_LT);
    disc(&mut d, fr.at(4.3, -4.0), 0.45 * z, pal::MOUSE_PINK_DK);
    // A whisker, only when there are pixels enough for it not to read as a stick.
    if k >= 2.0 {
        line(
            &mut d,
            fr.at(3.8, -3.6),
            fr.at(5.0, -4.0),
            pal::MOUSE_FUR_LT,
        );
    }

    // The near ear: big, round, pink inside.
    disc(&mut d, fr.at(0.7, -8.6), 2.7 * z, pal::MOUSE_FUR);
    disc(&mut d, fr.at(0.9, -8.4), 1.9 * z, pal::MOUSE_PINK);

    if forward && let Some((rig, shift)) = rig {
        mane(&mut d, rig, shift, paint, (hair, hair_dk));
    }

    // Bangs over the brow, the headband slanting down toward the eyes (determined), its
    // knot and loose ends behind.
    thick(&mut d, fr.at(-1.4, -6.9), fr.at(1.6, -6.3), 1.0 * z, hair);
    thick(&mut d, fr.at(0.6, -6.6), fr.at(2.1, -5.4), 0.6 * z, hair);
    thick(
        &mut d,
        fr.at(-2.7, -6.1),
        fr.at(2.6, -5.3),
        0.75 * z,
        pal::MOUSE_RED,
    );
    disc(&mut d, fr.at(-2.8, -5.8), 0.6 * z, pal::MOUSE_RED_DK);
    if let Some((rig, shift)) = rig {
        let band = chain(&rig.band.points, shift);
        strip(
            &mut d,
            &band,
            0.9 * k,
            0.5 * k,
            pal::MOUSE_RED,
            pal::MOUSE_RED_DK,
        );
    }

    // The eye, fierce: yellow and half-lidded under a heavy brow that slants down toward
    // the muzzle, the pupil fixed ahead.
    // At one pixel per unit, a single dark pixel; the slanted headband carries the look.
    if k >= 2.0 {
        disc(&mut d, fr.at(2.0, -4.6), 0.6 * z, pal::MOUSE_IRIS);
        pixel(&mut d, fr.at(2.4, -4.5), pal::MOUSE_EYE);
        thick(
            &mut d,
            fr.at(1.2, -5.3),
            fr.at(2.9, -4.7),
            0.6 * z,
            pal::MOUSE_FUR_DK,
        );
    } else {
        pixel(&mut d, fr.at(2.2, -4.4), pal::MOUSE_EYE);
    }

    // Drawn and raised to the sky (the fidget, after the Farewell painting): the blade
    // from her fist, a guard, and a glint at the tip on the second frame.
    if drawn {
        let (hu, hv) = hand(sprite);
        let tip = fr.at(hu + 0.4, hv - 8.5);
        thick(&mut d, fr.at(hu, hv), tip, 0.8 * z, pal::BLADE);
        line(
            &mut d,
            fr.at(hu + 0.1, hv - 0.5),
            (tip.0, tip.1 + 1.0),
            pal::BLADE_DK,
        );
        thick(
            &mut d,
            fr.at(hu - 0.9, hv - 0.6),
            fr.at(hu + 0.9, hv - 0.4),
            0.5 * z,
            pal::MOUSE_METAL,
        );
        if sprite == 50 {
            for (dx, dy) in [(0.0, -3.0), (0.0, 3.0), (-3.0, 0.0), (3.0, 0.0)] {
                line(&mut d, tip, (tip.0 + dx * k, tip.1 + dy * k), pal::WHITE);
            }
        }
    }

    for s in &d.spans {
        let c = paint.tint.unwrap_or(s.c);
        if let Some((g, amount)) = paint.glow.filter(|_| paint.tint.is_none()) {
            for px in s.x0..s.x1 {
                let c = if amount > bayer(px, s.y) { g } else { c };
                r.put(px, s.y, c);
            }
        } else if paint.alpha >= 1.0 {
            r.span(s.x0, s.x1, s.y, c, s.a);
        } else {
            for px in s.x0..s.x1 {
                if paint.alpha > bayer(px, s.y) {
                    r.put(px, s.y, c);
                }
            }
        }
    }
}

/// The mane's three strands (crown strand last, on top), waving, with loose wisps off the
/// crown strand, in `colors` (light, dark).
fn mane(d: &mut DrawList, rig: &Rig, shift: Pt, paint: &Paint, colors: (Rgb, Rgb)) {
    let k = paint.k;
    let (hair, hair_dk) = colors;
    let chain = |pts: &[Pt]| -> Vec<Pt> {
        pts.iter()
            .map(|p| ((p.0 + shift.0) * k, (p.1 + shift.1) * k))
            .collect()
    };
    for (i, h) in rig.hair.iter().enumerate().rev() {
        let amp = 0.6 * k * paint.flutter;
        let pts = wave(&chain(&h.points), paint.t * paint.flutter, i as f32, amp);
        let c = if i == 0 { hair } else { hair_dk };
        strip(d, &pts, 2.6 * k, 0.6 * k, c, hair_dk);
        // Loose wisps flicking off each strand: wild, windblown.
        // Only from the crown strand, every third point.
        let wisps = if i == 0 { &pts[..] } else { &pts[..0] };
        // They flick the way the strand flows, back or (blown) forward.
        let dir = (pts[pts.len() - 1].0 - pts[0].0).signum();
        for (j, p) in wisps.iter().enumerate().skip(2).step_by(3) {
            let w = (paint.t * 7.0 + j as f32 + i as f32 * 1.7).sin() * 1.2 * k;
            line(d, *p, (p.0 + dir * 1.6 * k, p.1 + w), hair);
        }
    }
}

/// One leg, from the hip to a cuffed boot at local forward `u`.
fn leg(d: &mut DrawList, fr: &Frame, u: f32, fur: Rgb, boot: Rgb) {
    let z = fr.scale();
    thick(d, fr.at(u * 0.4, 2.6), fr.at(u, 4.2), 1.2 * z, fur);
    let pts = [
        fr.at(u - 0.8, 3.8),
        fr.at(u + 0.8, 3.8),
        fr.at(u + 1.4, 6.0),
        fr.at(u - 0.9, 6.0),
    ];
    d.fill_polygon(&pts, boot);
    thick(
        d,
        fr.at(u - 0.9, 3.9),
        fr.at(u + 0.9, 3.9),
        0.5 * z,
        pal::MOUSE_LEATHER_LT,
    );
}

/// The sheath at the left hip, slanting back, with the sword's hilt in front when
/// `sword` is in it.
fn sheath(d: &mut DrawList, fr: &Frame, sword: bool) {
    let z = fr.scale();
    thick(
        d,
        fr.at(0.2, 1.6),
        fr.at(-3.6, 5.2),
        0.9 * z,
        pal::MOUSE_LEATHER_DK,
    );
    disc(d, fr.at(-3.6, 5.2), 0.5 * z, pal::MOUSE_METAL);
    if !sword {
        return;
    }
    // The cross-guard and the grip, pommel up front.
    thick(
        d,
        fr.at(0.0, 0.9),
        fr.at(0.9, 2.0),
        0.4 * z,
        pal::MOUSE_METAL,
    );
    thick(
        d,
        fr.at(0.5, 1.4),
        fr.at(1.3, 0.6),
        0.5 * z,
        pal::MOUSE_LEATHER,
    );
    disc(d, fr.at(1.4, 0.5), 0.4 * z, pal::MOUSE_METAL);
}

/// Offset the points of a mane strand sideways in a slow wave that runs down the strand.
fn wave(pts: &[Pt], t: f32, phase: f32, amp: f32) -> Vec<Pt> {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let (a, b) = (pts[i.saturating_sub(1)], pts[(i + 1).min(n - 1)]);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let l = (dx * dx + dy * dy).sqrt().max(1e-4);
            let w = (t * 4.0 + i as f32 * 1.1 + phase * 2.1).sin() * amp * i as f32 / n as f32;
            (pts[i].0 - dy / l * w, pts[i].1 + dx / l * w)
        })
        .collect()
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

/// A line `w` pixels thick between fractional points, as a quad.
fn thick(d: &mut DrawList, a: Pt, b: Pt, w: f32, c: Rgb) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l = (dx * dx + dy * dy).sqrt();
    if l < 0.5 || w < 1.0 {
        line(d, a, b, c);
        return;
    }
    let (nx, ny) = (-dy / l * w / 2.0, dx / l * w / 2.0);
    d.fill_polygon(
        &[
            (a.0 + nx, a.1 + ny),
            (b.0 + nx, b.1 + ny),
            (b.0 - nx, b.1 - ny),
            (a.0 - nx, a.1 - ny),
        ],
        c,
    );
}

/// A ribbon through `pts`, `w0` pixels wide at the start and `w1` at the end, in `c`, its
/// last stretch in `edge`.
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
    let w = |k: usize| (w0 + (w1 - w0) * k as f32 / (n - 1) as f32) / 2.0;
    for i in 0..n - 1 {
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

/// A torn hem at the end of the cape: three ragged teeth across its width `w`.
fn hem(d: &mut DrawList, pts: &[Pt], w: f32, c: Rgb) {
    let n = pts.len();
    if n < 2 {
        return;
    }
    let (a, b) = (pts[n - 2], pts[n - 1]);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l = (dx * dx + dy * dy).sqrt().max(1e-4);
    let (ux, uy) = (dx / l, dy / l);
    let (nx, ny) = (-uy, ux);
    for (k, len) in [(-0.4, 2.5), (0.05, 4.0), (0.4, 2.0)] {
        let base = (b.0 + nx * w * k, b.1 + ny * w * k);
        let half = w * 0.12;
        let tri = [
            (base.0 - nx * half, base.1 - ny * half),
            (base.0 + nx * half, base.1 + ny * half),
            (base.0 + ux * len, base.1 + uy * len),
        ];
        d.fill_polygon(&tri, c);
    }
}

/// Monarch-style wings from the shoulders, spread `spread` degrees back from straight up.
fn wings(d: &mut DrawList, fr: &Frame, spread: f32, k: f32) {
    let base = fr.at(-1.0, -1.8);
    for kk in [0.0f32, 0.35] {
        let a = (-90.0 - fr.f * spread * (1.0 - kk)).to_radians();
        let tip = (base.0 + a.cos() * 9.0 * k, base.1 + a.sin() * 9.0 * k);
        let a2 = a + fr.f * 0.6;
        let mid = (base.0 + a2.cos() * 6.0 * k, base.1 + a2.sin() * 6.0 * k);
        d.fill_polygon(&[base, tip, mid], pal::WHITE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::{Canvas, H, W};
    use crate::hero::sprite::COUNT;

    #[test]
    fn every_sprite_draws_her_near_the_hitbox() {
        let mut rig = Rig::default();
        rig.step(100.0, 100.0, 0, 1, (0.0, 0.0), Air::default(), 1.0 / 120.0);
        let center = ((100.0 + 3.0) * KF, (100.0 + 6.0) * KF);
        for sprite in 0..COUNT {
            let mut c = Canvas::new(W, H);
            draw(
                &mut c,
                100.0,
                100.0,
                sprite,
                1,
                Some((&rig, (0.0, 0.0))),
                &Paint::default(),
            );
            let lit: Vec<(i32, i32)> = (0..H)
                .flat_map(|y| (0..W).map(move |x| (x, y)))
                .filter(|&(x, y)| c.get(x, y) != Rgb(0, 0, 0))
                .collect();
            assert!(lit.len() > 60, "sprite {sprite}: {} pixels", lit.len());
            assert!(
                lit.iter().all(|&(x, y)| (x as f32 - center.0).abs() < 48.0
                    && (y as f32 - center.1).abs() < 48.0),
                "sprite {sprite} stays near her"
            );
        }
    }
}
