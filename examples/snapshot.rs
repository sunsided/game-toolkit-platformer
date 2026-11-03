//! Headless filmstrips of the hero's moves, for checking animation without a window:
//! `cargo run --example snapshot -- [out_dir] [move...]` (or `task snapshot`) plays each
//! scripted move on the demo level and writes a PNG at 3x: the level, the hero drawn every
//! few steps along the way (onion skin), and the states the animator went through.
//!
//! This works because nothing below the app layer touches the toolkit: a [`Session`] runs
//! headless and draws into any raster.

use animation::gfx::font::{self, Font};
use animation::gfx::{Canvas, H, Raster, W, palette as pal, png};
use animation::render::{backdrop, hero};
use animation::session::{Overlay, Session};
use animation::sim::{Intent, Preset, TILE, World, player};

/// Steps between onion-skin copies.
const EVERY: usize = 5;

/// A scripted move.
struct Script {
    /// File name and caption.
    name: &'static str,
    /// Feel preset; `None`: the default.
    preset: Option<Preset>,
    /// Start here (hitbox top-left, pixels) instead of the spawn.
    at: Option<(i32, i32)>,
    /// Steps to play.
    steps: usize,
    /// The intent at step `k`.
    intent: fn(usize, &World) -> Intent,
}

/// Jump held for `hold` steps from step `at`, otherwise `base`.
fn jump_at(k: usize, at: usize, hold: usize, base: Intent) -> Intent {
    Intent {
        jump_pressed: k == at,
        jump_held: (at..at + hold).contains(&k),
        ..base
    }
}

/// Moving with the stick at `x`.
fn go(x: f32) -> Intent {
    Intent {
        move_x: x,
        ..Intent::default()
    }
}

/// Moving right.
const RIGHT: Intent = Intent {
    move_x: 1.0,
    move_y: 0.0,
    jump_pressed: false,
    jump_held: false,
    dash_pressed: false,
    grab: false,
    dive_pressed: false,
    sprint: false,
    super_dash: false,
};

/// The side (-1, 1) of a wall within 2 px, or 0.
fn wall_near(w: &World) -> i32 {
    let r = w.player.rect();
    [-1, 1]
        .into_iter()
        .find(|&s| (1..=2).any(|d| w.level.overlaps(r.shifted(s * d, 0))))
        .unwrap_or(0)
}

/// Bounce between walls: jump off whichever wall is near (once falling or slow), with the
/// stick `aim` (-1 away from it, 1 toward it); start with a jump at step 5.
fn bounce(k: usize, w: &World, aim: f32) -> Intent {
    if k < 5 {
        return Intent::default();
    }
    let side = wall_near(w);
    let x = if side != 0 {
        aim * side as f32
    } else {
        // In flight: keep going away (the facing after a wall jump), or turn back.
        -aim * w.player.facing as f32
    };
    Intent {
        move_x: x,
        jump_pressed: k == 5 || (side != 0 && w.player.vy > -30.0 && !w.player.grounded),
        jump_held: true,
        ..Intent::default()
    }
}

/// Top of the narrow chimney's lid.
const LID_A: i32 = 8 * TILE;

/// The scripted moves.
const SCRIPTS: &[Script] = &[
    Script {
        name: "jump",
        preset: None,
        at: None,
        steps: 150,
        intent: |k, _| jump_at(k, 20, 30, RIGHT),
    },
    Script {
        name: "double-jump",
        preset: None,
        at: None,
        steps: 150,
        intent: |k, _| {
            let i = jump_at(k, 10, 20, Intent::default());
            if k >= 45 {
                jump_at(k, 45, 15, Intent::default())
            } else {
                i
            }
        },
    },
    Script {
        name: "wings",
        preset: Some(Preset::HollowKnight),
        at: None,
        steps: 150,
        intent: |k, _| {
            let i = jump_at(k, 10, 20, Intent::default());
            if k >= 45 {
                jump_at(k, 45, 15, Intent::default())
            } else {
                i
            }
        },
    },
    Script {
        name: "glide",
        preset: Some(Preset::Silksong),
        at: Some((27 * TILE, 5 * TILE - player::H)),
        steps: 300,
        intent: |k, _| Intent {
            move_x: 1.0,
            // Jump, press again in the air (the double jump), keep holding: glide.
            jump_pressed: k == 5 || k == 40,
            jump_held: (5..25).contains(&k) || k >= 40,
            ..Intent::default()
        },
    },
    Script {
        name: "run-roll",
        preset: None,
        at: None,
        steps: 110,
        intent: |k, _| Intent {
            move_x: 1.0,
            sprint: true,
            move_y: if k == 40 { 1.0 } else { 0.0 },
            ..Intent::default()
        },
    },
    Script {
        name: "super-dash",
        preset: None,
        at: None,
        steps: 260,
        intent: |k, _| Intent {
            super_dash: (5..115).contains(&k),
            ..Intent::default()
        },
    },
    Script {
        name: "wall-super-dash",
        preset: None,
        at: Some((14 * TILE - player::W, 11 * TILE)),
        steps: 220,
        intent: |k, _| Intent {
            move_x: if k < 30 { 1.0 } else { 0.0 },
            super_dash: (30..135).contains(&k),
            ..Intent::default()
        },
    },
    Script {
        name: "roll",
        preset: None,
        at: Some((16 * TILE, LID_A - player::H)),
        steps: 160,
        intent: |_, _| go(-1.0),
    },
    Script {
        name: "hard-landing",
        preset: None,
        at: Some((15 * TILE, LID_A - player::H)),
        steps: 160,
        intent: |k, w| {
            if k < 5 || w.player.grounded {
                go(-1.0)
            } else {
                Intent::default()
            }
        },
    },
    Script {
        name: "dash-up",
        preset: None,
        at: None,
        steps: 110,
        intent: |k, _| {
            let i = jump_at(k, 5, 10, Intent::default());
            Intent {
                move_y: if k == 20 { -1.0 } else { 0.0 },
                dash_pressed: k == 20,
                ..i
            }
        },
    },
    Script {
        name: "dash-down",
        preset: None,
        at: Some((16 * TILE, LID_A - player::H)),
        steps: 90,
        intent: |k, _| {
            let i = jump_at(k, 5, 30, Intent::default());
            Intent {
                move_y: if k == 40 { 1.0 } else { 0.0 },
                dash_pressed: k == 40,
                ..i
            }
        },
    },
    Script {
        name: "climb",
        preset: None,
        at: Some((31 * TILE + 1, 25 * TILE - player::H)),
        steps: 300,
        intent: |k, _| {
            let base = Intent {
                move_x: if k < 2 { -1.0 } else { 0.0 },
                grab: k >= 5,
                move_y: if k > 20 { -1.0 } else { 0.0 },
                ..Intent::default()
            };
            jump_at(k, 5, 15, base)
        },
    },
    Script {
        name: "chimney-bounce",
        preset: None,
        at: Some((16 * TILE, 25 * TILE - player::H)),
        steps: 220,
        intent: |k, w| bounce(k, w, -1.0),
    },
    Script {
        name: "wall-scale",
        preset: Some(Preset::HollowKnight),
        at: Some((25 * TILE, 25 * TILE - player::H)),
        steps: 260,
        intent: |k, w| {
            let i = bounce(k, w, 1.0);
            if wall_near(w) == 0 {
                Intent { move_x: -1.0, ..i }
            } else {
                i
            }
        },
    },
    Script {
        name: "meat-boy-bounce",
        preset: Some(Preset::MeatBoy),
        at: Some((27 * TILE, 14 * TILE)),
        steps: 220,
        intent: |k, w| bounce(k, w, -1.0),
    },
    Script {
        name: "dive",
        preset: None,
        at: None,
        steps: 100,
        intent: |k, _| {
            let i = jump_at(k, 5, 30, Intent::default());
            Intent {
                dive_pressed: k == 40,
                ..i
            }
        },
    },
    Script {
        name: "drop-through",
        preset: None,
        at: Some((6 * TILE, 22 * TILE - player::H)),
        steps: 80,
        intent: |k, _| Intent {
            move_y: if (10..14).contains(&k) { 1.0 } else { 0.0 },
            ..jump_at(k, 12, 2, Intent::default())
        },
    },
    Script {
        name: "push-pull",
        preset: None,
        at: None,
        steps: 400,
        intent: |k, _| match k {
            0..180 => RIGHT,
            180..200 => Intent {
                grab: true,
                ..Intent::default()
            },
            200..360 => Intent {
                move_x: -1.0,
                grab: true,
                ..Intent::default()
            },
            _ => Intent::default(),
        },
    },
    Script {
        name: "look",
        preset: None,
        at: None,
        steps: 160,
        intent: |k, _| Intent {
            move_y: match k {
                10..70 => -1.0,
                90..150 => 1.0,
                _ => 0.0,
            },
            ..Intent::default()
        },
    },
    Script {
        name: "idle-fidget",
        preset: None,
        at: None,
        steps: 780,
        intent: |_, _| Intent::default(),
    },
];

/// Plays the scripts and saves their filmstrips, printing every path written.
fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().unwrap_or_else(|| "snapshots".into());
    let only: Vec<String> = args.collect();
    std::fs::create_dir_all(&dir).expect("create output dir");
    for sc in SCRIPTS {
        if !only.is_empty() && !only.iter().any(|n| n == sc.name) {
            continue;
        }
        let mut s = Session::new();
        if let Some(p) = sc.preset {
            s.world.tuning = p.tuning();
            s.preset = Some(p);
        }
        if let Some((x, y)) = sc.at {
            let p = &mut s.world.player;
            p.x = x;
            p.y = y;
            p.peak_y = y;
        }
        let mut canvas = backdrop(&s.world.level);
        let mut seen: Vec<String> = Vec::new();
        for k in 0..sc.steps {
            let i = (sc.intent)(k, &s.world);
            s.step(&i);
            let state = s.animator.state_name().to_string();
            if seen.last() != Some(&state) {
                seen.push(state);
            }
            if k % EVERY == 0 || k + 1 == sc.steps {
                let (x, y) = s.world.player.exact_pos();
                let p = &s.world.player;
                hero::draw(&mut canvas, x, y, s.animator.sprite(), p.facing, None, 1.0);
            }
        }
        // The final frame on top: blocks, crystals, effects, the hero where it ended.
        s.overlay = Overlay::Off;
        animation::render::scene(&mut canvas, &s, 0.0);
        caption(&mut canvas, sc.name, &seen);
        let path = format!("{dir}/{}.png", sc.name);
        std::fs::write(&path, png::encode(&canvas, 3)).expect("write snapshot");
        println!("{path}");
    }
}

/// The move's name and the states it went through, wrapped across the top.
fn caption(c: &mut Canvas, name: &str, states: &[String]) {
    c.blend_rect(0, 0, W, 22, pal::BLACK, 0.6);
    font::draw(c, Font::Normal, 3, 2, name, pal::YELLOW);
    let (mut x, mut y) = (3, 11);
    for (i, s) in states.iter().enumerate() {
        let text = if i + 1 < states.len() {
            format!("{s} >")
        } else {
            s.clone()
        };
        let w = Font::Tiny.measure(&text, 1) + 4;
        if x + w > W - 3 {
            x = 3;
            y += 6;
        }
        font::draw(c, Font::Tiny, x, y, &text, pal::PALE);
        x += w;
    }
    let _ = H;
}
