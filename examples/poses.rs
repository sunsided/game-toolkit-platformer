//! A pose sheet: every hero sprite drawn as the mouse, each in its own cell with its
//! number, the cape and tail settled as if standing still for a moment:
//! `cargo run --example poses -- [out.png] [scale]` (or `task poses`). For checking the
//! puppet, and as the reference list for anyone drawing real frames.

use animation::gfx::font::{self, Font};
use animation::gfx::{Canvas, Raster, palette as pal, png};
use animation::hero::rig::Rig;
use animation::hero::sprite::COUNT;
use animation::render::mouse;

/// Cell size in pixels.
const CELL: i32 = 28;
/// Cells per row: 13 x 28 px and 6 rows fit the 384x216 grid, which the drawing clips
/// to.
const COLS: i32 = 13;

/// Draws the sheet and writes it.
fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or_else(|| "snapshots/poses.png".into());
    let scale = args.next().and_then(|s| s.parse().ok()).unwrap_or(4);
    let rows = (COUNT as i32 + COLS - 1) / COLS;
    let mut c = Canvas::new(COLS * CELL, rows * CELL);
    c.fill_rect(0, 0, c.w, c.h, pal::SKY_TOP);
    for sprite in 0..COUNT {
        let (col, row) = (sprite as i32 % COLS, sprite as i32 / COLS);
        let (x0, y0) = (col * CELL, row * CELL);
        c.rect_outline(x0, y0, CELL, CELL, pal::STEEL);
        let (x, y) = ((x0 + 13) as f32, (y0 + 12) as f32);
        let mut rig = Rig::default();
        for _ in 0..240 {
            rig.step(x, y, sprite, 1, (0.0, 0.0), 1.0 / 120.0);
        }
        mouse::draw(
            &mut c,
            x,
            y,
            sprite,
            1,
            Some(&rig),
            (0.0, 0.0),
            false,
            None,
            1.0,
        );
        font::draw(
            &mut c,
            Font::Tiny,
            x0 + 2,
            y0 + 2,
            &sprite.to_string(),
            pal::STEEL_LT,
        );
    }
    if let Some(dir) = std::path::Path::new(&out).parent() {
        std::fs::create_dir_all(dir).expect("create output dir");
    }
    std::fs::write(&out, png::encode(&c, scale)).expect("write the sheet");
    println!("{out}");
}
