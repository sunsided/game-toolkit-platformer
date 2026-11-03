//! [`Scaled`]: a terrain in your pixels, seen in the controller's units.

use crate::sim::{Rect, Terrain};

/// A terrain that answers in pixels, `k` of them per unit, asked in units: what a
/// [`crate::Platformer`] with a [`crate::Builder::scale`] steps through.
pub(crate) struct Scaled<'a> {
    /// The terrain, in pixels.
    pub inner: &'a mut dyn Terrain,
    /// Pixels per unit.
    pub k: i32,
}

impl Terrain for Scaled<'_> {
    fn solid(&self, r: Rect) -> bool {
        self.inner.solid(r.scaled(self.k))
    }

    fn platform(&self, r: Rect) -> bool {
        self.inner.platform(r.scaled(self.k))
    }

    fn refill(&mut self, r: Rect) -> bool {
        self.inner.refill(r.scaled(self.k))
    }

    fn fell_out(&self, r: Rect) -> bool {
        self.inner.fell_out(r.scaled(self.k))
    }

    fn update(&mut self, dt: f32, bodies: &[Rect]) {
        let bodies: Vec<Rect> = bodies.iter().map(|b| b.scaled(self.k)).collect();
        self.inner.update(dt, &bodies);
    }

    fn carry(&self, r: Rect) -> (f32, f32) {
        let (dx, dy) = self.inner.carry(r.scaled(self.k));
        (dx / self.k as f32, dy / self.k as f32)
    }

    // Every query above is converted; the base is only there so abilities find your
    // terrain behind this one (`find_mut`).
    fn base(&self) -> Option<&dyn Terrain> {
        Some(&*self.inner)
    }

    fn base_mut(&mut self) -> Option<&mut dyn Terrain> {
        Some(&mut *self.inner)
    }
}
