//! [`Canvas`]: a CPU RGB image, for the backdrop texture and headless snapshots.

use super::{DrawList, Raster, Rgb};

/// A CPU RGB image.
#[derive(Clone)]
pub struct Canvas {
    /// Width in pixels.
    pub w: i32,
    /// Height in pixels.
    pub h: i32,
    /// Row-major pixels, top row first.
    pub px: Vec<Rgb>,
}

impl Canvas {
    /// A black canvas.
    pub fn new(w: i32, h: i32) -> Self {
        Self {
            w,
            h,
            px: vec![Rgb(0, 0, 0); (w * h) as usize],
        }
    }

    /// The pixel at `x, y`, or black outside.
    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Rgb {
        if (0..self.w).contains(&x) && (0..self.h).contains(&y) {
            self.px[(y * self.w + x) as usize]
        } else {
            Rgb(0, 0, 0)
        }
    }

    /// Copy the rectangle `x, y, w, h` from `src` (same size) into this canvas.
    pub fn copy_rect(&mut self, src: &Canvas, x: i32, y: i32, w: i32, h: i32) {
        for yy in y.max(0)..(y + h).min(self.h) {
            let (a, b) = (x.max(0), (x + w).min(self.w));
            if a < b {
                let i = (yy * self.w) as usize;
                self.px[i + a as usize..i + b as usize]
                    .copy_from_slice(&src.px[i + a as usize..i + b as usize]);
            }
        }
    }

    /// Draw every span of `list` into this canvas, in order.
    pub fn apply(&mut self, list: &DrawList) {
        for s in &list.spans {
            self.span(s.x0, s.x1, s.y, s.c, s.a);
        }
    }

    /// Write the pixels as RGBA bytes (opaque) into `out`, which must hold `w * h * 4`.
    pub fn to_rgba(&self, out: &mut [u8]) {
        for (o, p) in out.as_chunks_mut::<4>().0.iter_mut().zip(&self.px) {
            o.copy_from_slice(&[p.0, p.1, p.2, 255]);
        }
    }
}

impl Raster for Canvas {
    fn size(&self) -> (i32, i32) {
        (self.w, self.h)
    }

    fn span(&mut self, x0: i32, x1: i32, y: i32, c: Rgb, a: f32) {
        if !(0..self.h).contains(&y) || a <= 0.0 {
            return;
        }
        let (x0, x1) = (x0.max(0), x1.min(self.w));
        if x0 >= x1 {
            return;
        }
        let row = &mut self.px[(y * self.w) as usize..][x0 as usize..x1 as usize];
        if a >= 1.0 {
            row.fill(c);
        } else {
            for p in row {
                *p = p.lerp(c, a);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::{H, W, palette};

    #[test]
    fn canvas_and_drawlist_agree() {
        let mut a = Canvas::new(W, H);
        let mut b = Canvas::new(W, H);
        let mut d = DrawList::default();
        let tri = [(100.3, 50.0), (140.0, 90.7), (90.0, 120.2)];
        a.fill_polygon(&tri, palette::RED);
        a.fill_circle(300, 200, 17, palette::GREEN);
        d.fill_polygon(&tri, palette::RED);
        d.fill_circle(300, 200, 17, palette::GREEN);
        b.apply(&d);
        assert!(a.px == b.px);
    }
}
