//! [`Rect`]: an integer axis-aligned box in grid pixels.

/// An axis-aligned box: top-left `x, y`, size `w` x `h`, grid pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub w: i32,
    /// Height.
    pub h: i32,
}

impl Rect {
    /// The box moved by `dx, dy`.
    pub fn shifted(self, dx: i32, dy: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            ..self
        }
    }

    /// One past the right edge.
    pub fn right(self) -> i32 {
        self.x + self.w
    }

    /// One past the bottom edge.
    pub fn bottom(self) -> i32 {
        self.y + self.h
    }

    /// Whether the two boxes share any pixel.
    pub fn intersects(self, o: Rect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }
}
