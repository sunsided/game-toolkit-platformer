//! The hero's body frame and loose parts.
//!
//! [`Frame`] maps the body's local coordinates (forward, down; the 6x12 hitbox spans
//! -3..3 by -6..6) to the screen for a sprite: squashed, stretched, leaning and mirrored as
//! its [`Pose`](super::pose::Pose) says. [`Rig`] carries the parts that aren't posed but
//! simulated: the cape, the tail, the mane and the headband's ends, each a short chain
//! of points that trails behind the body, swings when it turns and settles when it rests.
//!
//! Drawing only: the simulation never sees any of it.

use super::pose::pose;
use super::sprite as s;
use crate::sim::player;

/// A point on the screen, grid pixels.
pub type Pt = (f32, f32);

/// The body's local coordinate frame for one sprite at one position.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    /// Body center x.
    ox: f32,
    /// Body center y.
    oy: f32,
    /// Cosine of the lean.
    cos: f32,
    /// Sine of the lean.
    sin: f32,
    /// Horizontal squash or stretch.
    sx: f32,
    /// Vertical squash or stretch.
    sy: f32,
    /// Facing, 1 or -1.
    pub f: f32,
}

impl Frame {
    /// The frame of `sprite` for a hitbox with top-left `x, y`, facing `facing`.
    pub fn new(x: f32, y: f32, sprite: u16, facing: i32) -> Self {
        let p = pose(sprite);
        let f = facing as f32;
        let feet = y + player::H as f32 + p.dy;
        let a = (p.rot * f).to_radians();
        Self {
            ox: x + player::W as f32 / 2.0,
            oy: feet - p.h / 2.0,
            cos: a.cos(),
            sin: a.sin(),
            sx: p.w / player::W as f32,
            sy: p.h / player::H as f32,
            f,
        }
    }

    /// The screen point of local `u` (forward) and `v` (down).
    pub fn at(&self, u: f32, v: f32) -> Pt {
        let (lx, ly) = (u * self.f * self.sx, v * self.sy);
        (
            self.ox + lx * self.cos - ly * self.sin,
            self.oy + lx * self.sin + ly * self.cos,
        )
    }

    /// The average stretch, for sizing round parts.
    pub fn scale(&self) -> f32 {
        (self.sx + self.sy) / 2.0
    }
}

/// Where the arm's hand is for `sprite`, local (forward, down).
pub fn hand(sprite: u16) -> Pt {
    match sprite {
        56..=64 => (3.6, -0.2),
        30..=33 => (2.6, -3.8 + ((sprite - 30) % 2) as f32),
        34 | 35 => (2.6, -2.8),
        24..=29 | 75 | 76 => (3.0, -0.8),
        s::TAKEOFF | s::RISE | 12..=15 => (1.6, -2.6),
        _ => (1.0, 2.2),
    }
}

/// How far the feet are apart along the walk cycle for `sprite`, -1.5..1.5.
pub fn stride(sprite: u16) -> f32 {
    match sprite {
        s::RUN_A | 56 | 60 => 1.0,
        s::RUN_C | 58 | 62 => -1.0,
        67 => 1.5,
        69 => -1.5,
        s::RISE | s::FALL_A | s::FALL_B => 0.6,
        _ => 0.0,
    }
}

/// A chain of points hanging from an anchor: Verlet integration with fixed segment
/// lengths, gravity, air drag from moving, and a pull toward a rest shape.
#[derive(Clone, Debug)]
pub struct Chain {
    /// The points, anchor first.
    pub points: Vec<Pt>,
    /// The points one step ago.
    prev: Vec<Pt>,
    /// Segment length, pixels.
    seg: f32,
    /// Downward pull, px/s².
    gravity: f32,
    /// Pull toward the rest direction (backward, down), px/s², before facing.
    rest: Pt,
    /// How much air from moving pushes it, per px/s of speed.
    wind: f32,
    /// Fraction of velocity lost per step.
    drag: f32,
    /// How much the rest direction turns per segment (radians, toward up when
    /// positive): a tail that curls rather than sticks out straight.
    curl: f32,
}

impl Chain {
    /// A chain of `n` points `seg` apart.
    fn new(n: usize, seg: f32, gravity: f32, rest: Pt, wind: f32, drag: f32) -> Self {
        Self {
            curl: 0.0,
            points: vec![(0.0, 0.0); n],
            prev: vec![(0.0, 0.0); n],
            seg,
            gravity,
            rest,
            wind,
            drag,
        }
    }

    /// The same chain, curling `curl` radians per segment.
    fn curled(mut self, curl: f32) -> Self {
        self.curl = curl;
        self
    }

    /// The rest pull on point `i`, facing `f`: the rest direction turned by the curl.
    fn rest_at(&self, i: usize, f: f32) -> Pt {
        let (bx, by) = (-f * self.rest.0, self.rest.1);
        let a = f * self.curl * i as f32;
        let (s, c) = a.sin_cos();
        (bx * c - by * s, bx * s + by * c)
    }

    /// Lay the chain out straight from `anchor` along its rest direction.
    fn reset(&mut self, anchor: Pt, f: f32) {
        let (rx, ry) = (-f * self.rest.0, self.rest.1 + self.gravity);
        let len = (rx * rx + ry * ry).sqrt().max(1e-3);
        for (i, p) in self.points.iter_mut().enumerate() {
            let d = i as f32 * self.seg;
            *p = (anchor.0 + rx / len * d, anchor.1 + ry / len * d);
        }
        self.prev.clone_from(&self.points);
    }

    /// One step of `dt`: pin the anchor, move the rest by inertia, gravity, the rest pull
    /// and the air from the body's velocity `vel`, then restore the segment lengths.
    fn step(&mut self, anchor: Pt, vel: Pt, f: f32, extra: Pt, dt: f32) {
        self.points[0] = anchor;
        self.prev[0] = anchor;
        for i in 1..self.points.len() {
            let rest = self.rest_at(i, f);
            let acc = (
                rest.0 - vel.0 * self.wind + extra.0,
                self.gravity + rest.1 - vel.1 * self.wind + extra.1,
            );
            let (p, q) = (self.points[i], self.prev[i]);
            let v = (
                (p.0 - q.0) * (1.0 - self.drag),
                (p.1 - q.1) * (1.0 - self.drag),
            );
            self.prev[i] = p;
            self.points[i] = (p.0 + v.0 + acc.0 * dt * dt, p.1 + v.1 + acc.1 * dt * dt);
        }
        for _ in 0..3 {
            for i in 1..self.points.len() {
                let a = self.points[i - 1];
                let b = self.points[i];
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let d = (dx * dx + dy * dy).sqrt().max(1e-4);
                self.points[i] = (a.0 + dx / d * self.seg, a.1 + dy / d * self.seg);
            }
        }
    }
}

/// Where the loose parts hang from, local (forward, down).
pub mod anchor {
    use super::Pt;

    /// The cape, between the shoulders, a little behind.
    pub const CAPE: Pt = (-1.0, -1.4);
    /// The tail, low on the back.
    pub const TAIL: Pt = (-2.3, 3.6);
    /// The mane, back of the head.
    pub const HAIR: Pt = (-2.0, -4.4);
    /// The headband's knot, back of the head.
    pub const BAND: Pt = (-2.4, -5.0);
}

/// The hero's simulated parts.
#[derive(Clone, Debug)]
pub struct Rig {
    /// The cape, from the shoulders down.
    pub cape: Chain,
    /// The tail.
    pub tail: Chain,
    /// The blonde mane streaming back from the head.
    pub hair: Chain,
    /// The headband's loose ends.
    pub band: Chain,
    /// The cape's anchor last step, to notice teleports.
    last: Option<Pt>,
}

impl Default for Rig {
    fn default() -> Self {
        Self {
            cape: Chain::new(6, 1.8, 300.0, (260.0, 0.0), 2.2, 0.12),
            tail: Chain::new(10, 1.3, 40.0, (420.0, 160.0), 1.0, 0.18).curled(0.35),
            hair: Chain::new(5, 1.4, 120.0, (380.0, 80.0), 1.6, 0.15),
            band: Chain::new(4, 1.3, 200.0, (260.0, 40.0), 2.6, 0.1),
            last: None,
        }
    }
}

impl Rig {
    /// Follow the body one step of `dt`: hitbox top-left `x, y`, `sprite`, `facing`,
    /// velocity `vel` (px/s). Big jumps (a respawn, a placement) lay the parts out anew.
    pub fn step(&mut self, x: f32, y: f32, sprite: u16, facing: i32, vel: Pt, dt: f32) {
        let fr = Frame::new(x, y, sprite, facing);
        let at = |p: Pt| fr.at(p.0, p.1);
        let cape = at(anchor::CAPE);
        let jumped = self
            .last
            .is_none_or(|l| (l.0 - cape.0).abs() > 16.0 || (l.1 - cape.1).abs() > 16.0);
        self.last = Some(cape);
        if jumped {
            self.cape.reset(cape, fr.f);
            self.tail.reset(at(anchor::TAIL), fr.f);
            self.hair.reset(at(anchor::HAIR), fr.f);
            self.band.reset(at(anchor::BAND), fr.f);
            return;
        }
        // Gliding, the cape opens up and back like a sail.
        let sail = if s::GLIDE.contains(&sprite) {
            (-fr.f * 500.0, -900.0)
        } else {
            (0.0, 0.0)
        };
        self.cape.step(cape, vel, fr.f, sail, dt);
        self.tail.step(at(anchor::TAIL), vel, fr.f, (0.0, 0.0), dt);
        self.hair.step(at(anchor::HAIR), vel, fr.f, (0.0, 0.0), dt);
        self.band.step(at(anchor::BAND), vel, fr.f, (0.0, 0.0), dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn running_streams_the_cape_behind() {
        let mut r = Rig::default();
        let (mut x, y) = (100.0, 100.0);
        for _ in 0..120 {
            x += 90.0 / 120.0;
            r.step(x, y, s::RUN_A, 1, (90.0, 0.0), 1.0 / 120.0);
        }
        let tip = *r.cape.points.last().unwrap();
        assert!(tip.0 < r.cape.points[0].0 - 3.0, "trails behind: {tip:?}");
        // Turning around swings it to the other side.
        for _ in 0..120 {
            x -= 90.0 / 120.0;
            r.step(x, y, s::RUN_A, -1, (-90.0, 0.0), 1.0 / 120.0);
        }
        let tip = *r.cape.points.last().unwrap();
        assert!(tip.0 > r.cape.points[0].0 + 3.0, "swung over: {tip:?}");
    }

    #[test]
    fn segments_keep_their_length() {
        let mut r = Rig::default();
        for k in 0..200 {
            let t = k as f32 / 120.0;
            r.step(
                100.0 + t.sin() * 20.0,
                100.0,
                s::IDLE_A,
                1,
                (20.0 * t.cos(), 0.0),
                1.0 / 120.0,
            );
        }
        for w in r.tail.points.windows(2) {
            let d = ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt();
            assert!((d - r.tail.seg).abs() < 0.05, "{d}");
        }
    }
}
