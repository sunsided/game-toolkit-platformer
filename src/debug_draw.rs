//! A debug look with game-toolkit's painter: the level as blocks, the character as a box
//! colored by its animation state, the state's name above it. Enough to play and tune
//! before there is any art.

use game_toolkit::prelude::*;

use crate::Platformer;
use crate::sim::{TILE, Tile};

impl Platformer {
    /// Draw the level and the character with `p`, the level's top-left at `origin`
    /// (window pixels), `scale` window pixels per unit.
    pub fn draw_debug(&self, p: &mut Painter<'_>, origin: [f32; 2], scale: f32) {
        let at = |x: f32, y: f32| [origin[0] + x * scale, origin[1] + y * scale];
        let level = &self.world.level;
        let t = TILE as f32;
        for ty in 0..level.h {
            for tx in 0..level.w {
                let (x, y) = (tx as f32 * t, ty as f32 * t);
                match level.tile(tx, ty) {
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
                }
            }
        }
        for c in self.world.crystals.iter().filter(|c| c.ready()) {
            let a = c.area;
            p.rect(
                at(a.x as f32 + 2.0, a.y as f32 + 2.0),
                [4.0 * scale, 4.0 * scale],
                Color::rgb8(253, 196, 67),
            );
        }
        for b in &self.world.blocks {
            let r = b.rect();
            p.rect(
                at(r.x as f32, r.y as f32),
                [r.w as f32 * scale, r.h as f32 * scale],
                Color::rgb8(154, 106, 58),
            );
        }
        let (x, y) = self.position();
        let r = self.hitbox();
        let color = state_color(self.state());
        p.rect(at(x, y), [r.w as f32 * scale, r.h as f32 * scale], color);
        // An eye on the facing side.
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
