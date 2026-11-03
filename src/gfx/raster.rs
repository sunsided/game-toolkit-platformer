//! The [`Raster`] trait: the one drawing abstraction every primitive is written against.

use super::Rgb;

/// The 4x4 Bayer matrix, for ordered dithering.
const BAYER4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// The ordered-dither threshold at `x, y`, in 0..1. A pixel shows the "on" color of a
/// dither when its coverage is above this threshold.
#[inline]
pub fn bayer(x: i32, y: i32) -> f32 {
    (BAYER4[(y & 3) as usize][(x & 3) as usize] as f32 + 0.5) / 16.0
}

/// Something grid pixels can be drawn into. Implementors provide [`Raster::size`] and
/// [`Raster::span`]; every other method is built on those.
pub trait Raster {
    /// Width and height in pixels.
    fn size(&self) -> (i32, i32);

    /// Fill pixels `x0..x1` (end exclusive) of row `y` with `c` at opacity `a` (0..=1).
    /// Implementors clip.
    fn span(&mut self, x0: i32, x1: i32, y: i32, c: Rgb, a: f32);

    /// Set one pixel.
    fn put(&mut self, x: i32, y: i32, c: Rgb) {
        self.span(x, x + 1, y, c, 1.0);
    }

    /// Mix `c` over one pixel at opacity `a`.
    fn blend(&mut self, x: i32, y: i32, c: Rgb, a: f32) {
        self.span(x, x + 1, y, c, a);
    }

    /// Fill a `w` x `h` rectangle with its top-left at `x, y`.
    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb) {
        self.blend_rect(x, y, w, h, c, 1.0);
    }

    /// Mix `c` over a rectangle at opacity `a`.
    fn blend_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb, a: f32) {
        for yy in y..y + h {
            self.span(x, x + w, yy, c, a);
        }
    }

    /// A 1px rectangle outline.
    fn rect_outline(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb) {
        self.span(x, x + w, y, c, 1.0);
        self.span(x, x + w, y + h - 1, c, 1.0);
        for yy in y + 1..y + h - 1 {
            self.put(x, yy, c);
            self.put(x + w - 1, yy, c);
        }
    }

    /// A box with 1px outline and corners cut off, for speech bubbles and panels.
    fn round_box(&mut self, x: i32, y: i32, w: i32, h: i32, fill: Rgb, outline: Rgb) {
        self.span(x + 1, x + w - 1, y, outline, 1.0);
        self.span(x + 1, x + w - 1, y + h - 1, outline, 1.0);
        for yy in y + 1..y + h - 1 {
            self.put(x, yy, outline);
            self.span(x + 1, x + w - 1, yy, fill, 1.0);
            self.put(x + w - 1, yy, outline);
        }
    }

    /// A 1px line from `x0, y0` to `x1, y1` (Bresenham), both ends included.
    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Rgb) {
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let (mut x, mut y, mut err) = (x0, y0, dx + dy);
        loop {
            self.put(x, y, c);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// A filled disc of radius `r` around `cx, cy`, mixed in at opacity `a`.
    fn blend_circle(&mut self, cx: i32, cy: i32, r: i32, c: Rgb, a: f32) {
        if r < 0 {
            return;
        }
        let rr = (r * r + r) as f32;
        for dy in -r..=r {
            let half = (rr - (dy * dy) as f32).max(0.0).sqrt() as i32;
            self.span(cx - half, cx + half + 1, cy + dy, c, a);
        }
    }

    /// A filled disc of radius `r` around `cx, cy`.
    fn fill_circle(&mut self, cx: i32, cy: i32, r: i32, c: Rgb) {
        self.blend_circle(cx, cy, r, c, 1.0);
    }

    /// A disc whose coverage falls from `inner` at the center to `outer` at the rim, drawn
    /// with ordered dithering instead of translucency: pixels are fully `c` or untouched.
    fn dither_circle(&mut self, cx: i32, cy: i32, r: i32, c: Rgb, inner: f32, outer: f32) {
        if r <= 0 {
            return;
        }
        for dy in -r..=r {
            for dx in -r..=r {
                let d = ((dx * dx + dy * dy) as f32).sqrt() / r as f32;
                if d > 1.0 {
                    continue;
                }
                let cov = inner + (outer - inner) * d;
                if cov > bayer(cx + dx, cy + dy) {
                    self.put(cx + dx, cy + dy, c);
                }
            }
        }
    }

    /// A filled polygon with float vertices (scanline, even-odd), sampled at pixel
    /// centers. Rotated shapes come out with honest grid jaggies.
    fn fill_polygon(&mut self, pts: &[(f32, f32)], c: Rgb) {
        if pts.len() < 3 {
            return;
        }
        let y0 = pts.iter().map(|p| p.1).fold(f32::MAX, f32::min).floor() as i32;
        let y1 = pts.iter().map(|p| p.1).fold(f32::MIN, f32::max).ceil() as i32;
        let mut xs = Vec::with_capacity(8);
        for y in y0..=y1 {
            let sy = y as f32 + 0.5;
            xs.clear();
            for i in 0..pts.len() {
                let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
                if (a.1 <= sy) != (b.1 <= sy) {
                    xs.push(a.0 + (sy - a.1) / (b.1 - a.1) * (b.0 - a.0));
                }
            }
            xs.sort_by(f32::total_cmp);
            for pair in xs.as_chunks::<2>().0 {
                let xa = (pair[0] - 0.5).ceil() as i32;
                let xb = (pair[1] - 0.5).ceil() as i32;
                self.span(xa, xb, y, c, 1.0);
            }
        }
    }
}
