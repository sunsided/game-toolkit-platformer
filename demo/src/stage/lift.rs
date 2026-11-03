//! [`Lift`]: a solid platform going back and forth between two points: the demo's moving
//! ground, carrying whoever stands on it or holds on to its side (`docs/moving-platforms.md`
//! is the same thing as a recipe).

use crate::sim::Rect;

/// A moving platform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lift {
    /// Where it is.
    pub rect: Rect,
    /// Where it was before the last update, for [`Lift::carry`].
    prev: Rect,
    /// One end of its way (top-left, units).
    pub a: (i32, i32),
    /// The other end.
    pub b: (i32, i32),
    /// Speed along the way, units/s.
    pub speed: f32,
    /// Seconds it waits at each end.
    pub pause: f32,
    /// Seconds left to wait.
    wait: f32,
    /// Heading for `b` (else for `a`).
    to_b: bool,
    /// Sub-unit motion not yet moved.
    rem: (f32, f32),
}

impl Lift {
    /// A `w` x `h` platform at `a`, going to `b` and back at `speed`, waiting `pause` at
    /// each end.
    pub fn new(a: (i32, i32), b: (i32, i32), (w, h): (i32, i32), speed: f32, pause: f32) -> Self {
        let rect = Rect {
            x: a.0,
            y: a.1,
            w,
            h,
        };
        Self {
            rect,
            prev: rect,
            a,
            b,
            speed,
            pause,
            wait: pause,
            to_b: true,
            rem: (0.0, 0.0),
        }
    }

    /// Whether `body` stands on top of it: it may move into that one (and carries it).
    fn rider(&self, body: Rect) -> bool {
        body.bottom() == self.rect.y && body.right() > self.rect.x && body.x < self.rect.right()
    }

    /// One step of `dt`: wait, or move toward the end it's heading for, a unit at a time.
    /// It stops short of `bodies` it would move into from below or the side (it doesn't
    /// shove the characters), but goes on into its riders, which the world carries along.
    pub fn update(&mut self, dt: f32, bodies: &[Rect]) {
        self.prev = self.rect;
        if self.wait > 0.0 {
            self.wait = (self.wait - dt).max(0.0);
            return;
        }
        let (tx, ty) = if self.to_b { self.b } else { self.a };
        let (dx, dy) = ((tx - self.rect.x) as f32, (ty - self.rect.y) as f32);
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.5 {
            self.to_b = !self.to_b;
            self.wait = self.pause;
            self.rem = (0.0, 0.0);
            return;
        }
        let step = (self.speed * dt).min(len);
        self.rem.0 += dx / len * step;
        self.rem.1 += dy / len * step;
        let (nx, ny) = (self.rem.0.round() as i32, self.rem.1.round() as i32);
        self.rem.0 -= nx as f32;
        self.rem.1 -= ny as f32;
        let others: Vec<Rect> = bodies.iter().copied().filter(|&b| !self.rider(b)).collect();
        for (n, (sx, sy)) in [(nx, (nx.signum(), 0)), (ny, (0, ny.signum()))] {
            for _ in 0..n.abs() {
                let next = self.rect.shifted(sx, sy);
                if others.iter().any(|&b| next.intersects(b)) {
                    self.rem = (0.0, 0.0);
                    return;
                }
                self.rect = next;
            }
        }
    }

    /// How far it moved in the last update, if `r` touched it before (the row under a
    /// rider's feet, the column beside a hand on its side).
    pub fn carry(&self, r: Rect) -> Option<(f32, f32)> {
        r.intersects(self.prev).then(|| {
            (
                (self.rect.x - self.prev.x) as f32,
                (self.rect.y - self.prev.y) as f32,
            )
        })
    }
}
