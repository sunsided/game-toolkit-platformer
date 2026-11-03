//! The debug overlay: what the controller and the body are doing, step by step.
//!
//! Left: the animator (state, frame, lock, recent states) and the body (mode, speeds,
//! buffers, resources). Right: every parameter, pending triggers lit. Bottom: the event
//! log. Plus the hitbox and the clock.

use crate::anim::ParamKind;
use crate::gfx::font::{self, Font};
use crate::gfx::{H, Raster, Rgb, W, palette as pal};
use crate::session::Session;

/// Line pitch of the tiny font.
const LINE: i32 = 6;

/// Draw the overlay.
pub fn draw(r: &mut dyn Raster, s: &Session) {
    let p = &s.world.player;
    let a = &s.animator;
    let t = &s.world.tuning;
    r.rect_outline(
        p.x - 1,
        p.y - 1,
        crate::sim::player::W + 2,
        crate::sim::player::H + 2,
        pal::STEEL,
    );

    // Dim backing panels, so the text reads over tiles.
    r.blend_rect(0, 0, 124, 2 + 10 * LINE, pal::BLACK, 0.55);
    let params_w = a
        .params
        .iter()
        .map(|(_, name, _, _)| Font::Tiny.measure(name, 1) + 24)
        .max()
        .unwrap_or(0);
    let params_h = 3 + LINE * a.params.iter().count() as i32;
    r.blend_rect(
        W - params_w - 4,
        0,
        params_w + 4,
        params_h,
        pal::BLACK,
        0.55,
    );
    let log_h = 3 + LINE * s.log.len() as i32;
    r.blend_rect(0, H - log_h - 1, 200, log_h + 1, pal::BLACK, 0.55);

    let mut y = 2;
    let mut line = |r: &mut dyn Raster, text: &str, c: Rgb| {
        font::draw(r, Font::Tiny, 3, y, text, c);
        y += LINE;
    };
    let lock = if a.locked() { " LOCK" } else { "" };
    line(
        r,
        &format!(
            "{} F{} {:.2}S{lock}",
            a.state_name(),
            a.frame(),
            a.time_in_state()
        ),
        if a.locked() { pal::RED } else { pal::YELLOW },
    );
    let hist: Vec<String> = a
        .history()
        .map(|(id, secs)| format!("{}{:.0}", a.graph().state(id).name, secs * 1000.0))
        .collect();
    line(r, &hist.join(" "), pal::STEEL_LT);
    line(r, &format!("{:?}", p.mode).to_uppercase(), pal::PALE);
    line(
        r,
        &format!(
            "VX {:+.0} VY {:+.0} {}",
            p.vx,
            p.vy,
            if p.grounded { "GROUND" } else { "AIR" }
        ),
        pal::PALE,
    );
    meter(r, 3, y, "JUMP BUF", p.buffer / t.jump_buffer, pal::YELLOW);
    meter(
        r,
        3,
        y + LINE,
        "COYOTE",
        p.coyote / t.coyote_time,
        pal::GREEN,
    );
    meter(
        r,
        3,
        y + 2 * LINE,
        "DASH BUF",
        p.dash_buffer / t.dash_buffer,
        pal::YELLOW,
    );
    meter(
        r,
        3,
        y + 3 * LINE,
        "STAMINA",
        p.stamina / t.stamina,
        pal::GREEN,
    );
    meter(
        r,
        3,
        y + 4 * LINE,
        "VAR JUMP",
        p.var_jump / t.var_jump_time,
        pal::CYAN,
    );
    font::draw(
        r,
        Font::Tiny,
        3,
        y + 5 * LINE,
        &format!(
            "DASH {} AIR {} WALL {}",
            p.dashes, p.air_jumps, p.wall_slide
        ),
        pal::PALE,
    );

    let mut py = 2;
    for (_, name, kind, v) in a.params.iter() {
        let (text, c) = match kind {
            ParamKind::Bool => (
                format!("{name} {}", if v != 0.0 { "ON" } else { "-" }),
                if v != 0.0 { pal::GREEN } else { pal::STEEL },
            ),
            ParamKind::Float => (format!("{name} {v:+.0}"), pal::PALE),
            ParamKind::Trigger { .. } => (
                name.to_string(),
                if v > 0.0 { pal::YELLOW } else { pal::STEEL },
            ),
        };
        let w = Font::Tiny.measure(&text, 1);
        font::draw(r, Font::Tiny, W - 3 - w, py, &text, c);
        py += LINE;
    }

    let mut ly = H - 3 - LINE * s.log.len() as i32;
    for (tick, text) in &s.log {
        font::draw(
            r,
            Font::Tiny,
            3,
            ly,
            &format!("{tick} {text}"),
            pal::STEEL_LT,
        );
        ly += LINE;
    }
    let preset = s.preset.map_or("custom", |p| p.name()).to_uppercase();
    let w = Font::Tiny.measure(&preset, 1);
    font::draw(
        r,
        Font::Tiny,
        W - 3 - w,
        H - 3 - 2 * LINE,
        &preset,
        pal::CYAN,
    );
    let clock = if s.clock.paused {
        "PAUSED (. STEP, P GO)".to_string()
    } else {
        format!("X{}", s.clock.speed())
    };
    let w = Font::Tiny.measure(&clock, 1);
    font::draw(r, Font::Tiny, W - 3 - w, H - 3 - LINE, &clock, pal::YELLOW);
}

/// A labeled bar, `v` in 0..1.
fn meter(r: &mut dyn Raster, x: i32, y: i32, label: &str, v: f32, c: Rgb) {
    font::draw(r, Font::Tiny, x, y, label, pal::STEEL_LT);
    let bx = x + 36;
    r.fill_rect(bx, y + 1, 30, 3, pal::STONE_DK);
    r.fill_rect(bx, y + 1, (30.0 * v.clamp(0.0, 1.0)).round() as i32, 3, c);
}
