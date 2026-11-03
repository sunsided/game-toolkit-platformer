//! [`DrawList`]: records grid spans that the app draws as GPU rects.

use super::{H, Raster, Rgb, W};

/// One recorded run of pixels: `x0..x1` on row `y`, color `c`, opacity `a`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    /// First pixel column, inclusive.
    pub x0: i32,
    /// End column, exclusive.
    pub x1: i32,
    /// Pixel row.
    pub y: i32,
    /// Color.
    pub c: Rgb,
    /// Opacity, 0..=1.
    pub a: f32,
}

/// Records spans for the GPU. Consecutive spans with the same row, color and opacity that
/// touch are merged, so a filled shape costs one rect per row.
#[derive(Default)]
pub struct DrawList {
    /// The recorded spans, in draw order.
    pub spans: Vec<Span>,
}

impl DrawList {
    /// Forget all spans, for the next frame.
    pub fn clear(&mut self) {
        self.spans.clear();
    }
}

impl Raster for DrawList {
    fn size(&self) -> (i32, i32) {
        (W, H)
    }

    fn span(&mut self, x0: i32, x1: i32, y: i32, c: Rgb, a: f32) {
        if !(0..H).contains(&y) || a <= 0.0 {
            return;
        }
        let (x0, x1, a) = (x0.max(0), x1.min(W), a.min(1.0));
        if x0 >= x1 {
            return;
        }
        if let Some(last) = self.spans.last_mut()
            && last.y == y
            && last.c == c
            && last.a == a
            && last.x1 == x0
        {
            last.x1 = x1;
            return;
        }
        self.spans.push(Span { x0, x1, y, c, a });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::palette;

    #[test]
    fn merges_rows() {
        let mut d = DrawList::default();
        d.fill_rect(10, 10, 5, 3, palette::WHITE);
        assert_eq!(d.spans.len(), 3);
        d.put(15, 12, palette::WHITE);
        assert_eq!(d.spans.len(), 3);
        assert_eq!(d.spans[2].x1, 16);
    }
}
