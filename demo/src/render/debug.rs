//! The debug overlay: what the controller and the body are doing, step by step, kept to
//! the screen's corners. F1 cycles compact, full and off.
//!
//! - Top left: the animator (state, frame, lock; in full: recent states) and the body
//!   (mode, speeds, resources, buffers and timers as bars).
//! - Top right: parameters. Compact lists only the active ones (bools on, floats off zero,
//!   pending triggers); full lists all.
//! - Bottom left: the event log. Bottom right: the preset's keys, the preset, the clock.

use crate::anim::ParamKind;
use crate::gfx::font::{self, Font};
use crate::gfx::{H, Raster, Rgb, W, palette as pal};
use crate::session::{Overlay, Session};
use crate::sim::{Mode, Preset, player};

/// Line pitch of the tiny font.
const LINE: i32 = 6;
/// Opacity of the panels behind the text.
const PANEL: f32 = 0.6;
/// Log lines shown in compact mode.
const COMPACT_LOG: usize = 3;

/// One line of overlay text.
type Line = (String, Rgb);

/// Draw the overlay in the session's mode.
pub fn draw(r: &mut dyn Raster, s: &Session) {
    let full = match s.overlay {
        Overlay::Off => return,
        Overlay::Compact => false,
        Overlay::Full => true,
    };
    let p = &s.world.player;
    if full {
        // The hitbox, only in the full overlay: it covers the hero otherwise.
        let k = super::K;
        r.rect_outline(
            p.x * k - 1,
            p.y * k - 1,
            player::W * k + 2,
            player::H * k + 2,
            pal::STEEL,
        );
    }
    body(r, s, full);
    params(r, s, full);
    log(r, s, full);
    status(r, s);
}

/// Width of the widest line.
fn width(lines: &[Line]) -> i32 {
    lines
        .iter()
        .map(|(t, _)| Font::Tiny.measure(t, 1))
        .max()
        .unwrap_or(0)
}

/// Lines on a dim panel, left- or right-aligned at `x`, from `y` down. Returns the panel
/// height.
fn panel(r: &mut dyn Raster, lines: &[Line], x: i32, y: i32, right: bool, min_w: i32) -> i32 {
    if lines.is_empty() {
        return 0;
    }
    let w = width(lines).max(min_w) + 4;
    let h = LINE * lines.len() as i32 + 2;
    let px = if right { x - w } else { x };
    r.blend_rect(px, y, w, h, pal::BLACK, PANEL);
    for (k, (t, c)) in lines.iter().enumerate() {
        let tx = if right {
            x - 2 - Font::Tiny.measure(t, 1)
        } else {
            x + 2
        };
        font::draw(r, Font::Tiny, tx, y + 2 + LINE * k as i32, t, *c);
    }
    h
}

/// A short name for the body's mode.
fn mode(m: Mode) -> &'static str {
    match m {
        Mode::Normal => "FREE",
        Mode::Roll { .. } => "ROLL",
        Mode::HardLand { .. } => "HARD",
        Mode::Dash { .. } => "DASH",
        Mode::Climb => "CLIMB",
        Mode::Dive { .. } => "DIVE",
        Mode::DiveLand { .. } => "IMPACT",
        Mode::SuperCharge { .. } => "CHARGE",
        Mode::SuperDash { .. } => "SUPER",
        Mode::Custom { .. } => "OWN",
        _ => "?",
    }
}

/// Top left: animator and body.
fn body(r: &mut dyn Raster, s: &Session, full: bool) {
    let (a, p, t) = (&s.animator, &s.world.player, &s.world.tuning);
    let lock = if a.locked() { " LOCK" } else { "" };
    let mut lines: Vec<Line> = vec![
        (
            format!(
                "{} F{} {:.2}{lock}",
                a.state_name(),
                a.frame(),
                a.time_in_state()
            ),
            if a.locked() { pal::RED } else { pal::YELLOW },
        ),
        (
            format!(
                "{} {:+.0},{:+.0} D{} A{}",
                mode(p.mode),
                p.vx,
                p.vy,
                p.dashes,
                p.air_jumps
            ),
            pal::PALE,
        ),
    ];
    if full {
        let w = crate::session::Wind::at(s.world.tick, p.y as f32);
        lines.push((
            format!("BREEZE {:.0} GUST {:+.0} {:.2}", w.breeze, w.push.0, w.gust),
            pal::STEEL_LT,
        ));
        let hist: Vec<String> = a
            .history()
            .map(|(id, secs)| format!("{}{:.0}", a.graph().state(id).name, secs * 1000.0))
            .collect();
        lines.push((hist.join(" "), pal::STEEL_LT));
    }
    // Room for the meters below the text.
    let meters = [
        ("J", p.buffer / t.jump_buffer, pal::YELLOW),
        ("C", p.coyote / t.coyote_time, pal::GREEN),
        ("D", p.dash_buffer / t.dash_buffer, pal::YELLOW),
        ("V", p.var_jump / t.var_jump_time, pal::CYAN),
        ("S", p.stamina / t.stamina, pal::GREEN),
    ];
    let meter_w = 14;
    let min_w = meter_w * meters.len() as i32;
    let h = panel(r, &lines, 0, 0, false, min_w);
    let y = h;
    r.blend_rect(0, y, min_w + 4, LINE + 1, pal::BLACK, PANEL);
    for (k, (label, v, c)) in meters.iter().enumerate() {
        let x = 2 + k as i32 * meter_w;
        font::draw(r, Font::Tiny, x, y, label, pal::STEEL_LT);
        r.fill_rect(x + 4, y + 1, 8, 3, pal::STONE_DK);
        r.fill_rect(
            x + 4,
            y + 1,
            (8.0 * v.clamp(0.0, 1.0)).round() as i32,
            3,
            *c,
        );
    }
}

/// Top right: parameters.
fn params(r: &mut dyn Raster, s: &Session, full: bool) {
    let mut lines: Vec<Line> = Vec::new();
    for (_, name, kind, v) in s.animator.params.iter() {
        let line = match kind {
            ParamKind::Bool if v != 0.0 => Some((name.to_string(), pal::GREEN)),
            ParamKind::Bool => full.then(|| (name.to_string(), pal::STEEL)),
            ParamKind::Float if v.abs() >= 0.5 => Some((format!("{name} {v:+.0}"), pal::PALE)),
            ParamKind::Float => full.then(|| (format!("{name} 0"), pal::STEEL)),
            ParamKind::Trigger { .. } if v > 0.0 => Some((format!("{name}!"), pal::YELLOW)),
            ParamKind::Trigger { .. } => full.then(|| (name.to_string(), pal::STEEL)),
        };
        lines.extend(line);
    }
    panel(r, &lines, W, 0, true, 0);
}

/// Bottom left: the newest events.
fn log(r: &mut dyn Raster, s: &Session, full: bool) {
    let n = if full { s.log.len() } else { COMPACT_LOG };
    let lines: Vec<Line> = s
        .log
        .iter()
        .skip(s.log.len().saturating_sub(n))
        .map(|(tick, text)| (format!("{tick} {text}"), pal::STEEL_LT))
        .collect();
    let h = LINE * lines.len() as i32 + 2;
    panel(r, &lines, 0, H - h, false, 0);
}

/// The keyboard legend of the preset's control scheme (mirrors `app::controls`).
fn keys(p: Option<Preset>) -> &'static str {
    match p {
        Some(Preset::Precise) => "C JUMP X DASH Z GRAB",
        Some(Preset::Momentum) => "SPACE JUMP SHIFT RUN",
        Some(Preset::Drifter) => "Z JUMP, HOLD: GLIDE C DASH",
        Some(Preset::Snappy) => "Z JUMP C DASH S SUPER DN+A DIVE",
        _ => "Z JUMP C DASH S SUPER D GRAB DN+A DIVE SHIFT SPRINT",
    }
}

/// Bottom right: the keys, the preset and the clock.
fn status(r: &mut dyn Raster, s: &Session) {
    let preset = s.preset.map_or("custom", |p| p.name()).to_uppercase();
    let clock = if s.clock.paused {
        "PAUSED".to_string()
    } else {
        format!("X{}", s.clock.speed())
    };
    let lines = [
        (keys(s.preset).to_string(), pal::STEEL_LT),
        (format!("{preset} {clock}"), pal::CYAN),
    ];
    panel(r, &lines, W, H - 2 * LINE - 2, true, 0);
}
