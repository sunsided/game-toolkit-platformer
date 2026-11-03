//! A debug look with game-toolkit's painter: a [`Level`] as blocks, the character as a
//! box colored by its animation state with the state's name above it. Enough to play and
//! tune before there is any art.

use game_toolkit::prelude::*;

use crate::Platformer;
use crate::sim::{Level, Tile};

impl Level {
    /// Draw the tiles and the dash refills that are there with `p`, the level's
    /// top-left at `origin` (window pixels), `scale` window pixels per unit.
    pub fn draw_debug(&self, p: &mut Painter<'_>, origin: [f32; 2], scale: f32) {
        let at = |x: f32, y: f32| [origin[0] + x * scale, origin[1] + y * scale];
        let t = self.tile_size() as f32;
        for ty in 0..self.height() {
            for tx in 0..self.width() {
                let (x, y) = (tx as f32 * t, ty as f32 * t);
                match self.tile(tx, ty) {
                    Tile::Empty => {}
                    Tile::Solid => {
                        p.rect(at(x, y), [t * scale, t * scale], Color::rgb8(74, 64, 100));
                    }
                    Tile::Platform => {
                        p.rect(
                            at(x, y),
                            [t * scale, 2.0 * scale],
                            Color::rgb8(156, 112, 66),
                        );
                    }
                    Tile::Custom(k) => {
                        // A small square, its shade by number, so kinds tell apart.
                        let shade = 120u8.wrapping_add(k.wrapping_mul(53));
                        p.rect(
                            at(x + 2.0, y + 2.0),
                            [(t - 4.0) * scale, (t - 4.0) * scale],
                            Color::rgb8(200, shade, 120),
                        );
                    }
                }
            }
        }
        for c in self.refills().iter().filter(|c| c.ready()) {
            let a = c.area;
            let (q, side) = (a.w as f32 / 4.0, a.w as f32 / 2.0);
            p.rect(
                at(a.x as f32 + q, a.y as f32 + q),
                [side * scale, side * scale],
                Color::rgb8(253, 196, 67),
            );
        }
    }
}

impl Platformer {
    /// Draw the character as a box colored by its animation state, an eye on the side it
    /// faces and the state's name above, with `p`: the world's origin at `origin` (window
    /// pixels), `scale` window pixels per unit.
    pub fn draw_debug(&self, p: &mut Painter<'_>, origin: [f32; 2], scale: f32) {
        let at = |x: f32, y: f32| [origin[0] + x * scale, origin[1] + y * scale];
        let (x, y) = self.position();
        let r = self.hitbox();
        p.rect(
            at(x, y),
            [r.w as f32 * scale, r.h as f32 * scale],
            state_color(self.state()),
        );
        let eye_x = if self.facing() > 0 {
            x + r.w as f32 - 2.0
        } else {
            x + 1.0
        };
        p.rect(
            at(eye_x, y + 2.0),
            [scale, scale],
            Color::rgb8(255, 255, 255),
        );
        p.text(
            at(x - 4.0, y - 6.0),
            self.state(),
            3.5 * scale,
            Color::rgb8(194, 203, 232),
        );
    }
}

/// A steady, distinct color per state name, so state changes are visible at a glance.
fn state_color(name: &str) -> Color {
    let h = name
        .bytes()
        .fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619));
    let (r, g, b) = (
        (h & 0xff) as u8,
        ((h >> 8) & 0xff) as u8,
        ((h >> 16) & 0xff) as u8,
    );
    // Keep it bright enough on a dark background.
    Color::rgb8(r / 2 + 110, g / 2 + 110, b / 2 + 110)
}
