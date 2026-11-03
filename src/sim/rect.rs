//! [`Rect`]: an integer axis-aligned box, in units or in a terrain's pixels.

/// An axis-aligned box: top-left `x, y`, size `w` x `h`. Units inside the movement, a
/// terrain's pixels at the facade with a `Builder::scale` (see `docs/scale.md`).
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

    /// The box `k` times as large, from the origin: units to pixels at `k` pixels per
    /// unit.
    pub fn scaled(self, k: i32) -> Self {
        Self {
            x: self.x * k,
            y: self.y * k,
            w: self.w * k,
            h: self.h * k,
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

    /// Whether the two boxes overlap (share any unit square).
    pub fn intersects(self, o: Rect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }
}
