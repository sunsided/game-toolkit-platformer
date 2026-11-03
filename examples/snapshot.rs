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
use animation::session::Session;
use animation::sim::{Intent, TILE, World, player};

/// Steps between onion-skin copies.
const EVERY: usize = 5;

/// A scripted move.
struct Script {
    /// File name and caption.
    name: &'static str,
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

/// Moving right.
const RIGHT: Intent = Intent {
    move_x: 1.0,
    move_y: 0.0,
    jump_pressed: false,
    jump_held: false,
    dash_pressed: false,
    grab: false,
    dive_pressed: false,
};

/// The scripted moves.
const SCRIPTS: &[Script] = &[
    Script {
        name: "jump",
        at: None,
        steps: 150,
        intent: |k, _| jump_at(k, 20, 30, RIGHT),
    },
    Script {
        name: "double-jump",
        at: None,
        steps: 170,
        intent: |k, _| {
            let i = jump_at(k, 10, 20, RIGHT);
            if k >= 45 {
                jump_at(k, 45, 15, RIGHT)
            } else {
                i
            }
        },
    },
    Script {
        name: "roll",
        at: Some((3 * TILE, 9 * TILE - player::H)),
        steps: 200,
        intent: |_, _| RIGHT,
    },
    Script {
        name: "hard-landing",
        at: Some((5 * TILE, 9 * TILE - player::H)),
        steps: 200,
        intent: |k, w| {
            if k < 5 || w.player.grounded {
                RIGHT
            } else {
                Intent::default()
            }
        },
    },
    Script {
        name: "dash-up",
        at: None,
        steps: 120,
        intent: |k, _| {
            let i = jump_at(k, 5, 10, RIGHT);
            if k == 25 {
                Intent {
                    move_y: -1.0,
                    dash_pressed: true,
                    ..i
                }
            } else {
                i
            }
        },
    },
    Script {
        name: "dash-down",
        at: Some((15 * TILE, 17 * TILE - player::H)),
        steps: 90,
        intent: |k, _| {
            let i = jump_at(k, 5, 30, Intent::default());
            if k == 40 {
                Intent {
                    move_y: 1.0,
                    dash_pressed: true,
                    ..i
                }
            } else {
                i
            }
        },
    },
    Script {
        name: "climb",
        at: Some((42 * TILE - player::W, 18 * TILE - player::H)),
        steps: 260,
        intent: |k, _| {
            let base = Intent {
                grab: k >= 5,
                move_y: if k > 20 { -1.0 } else { 0.0 },
                ..Intent::default()
            };
            jump_at(k, 5, 15, base)
        },
    },
    Script {
        name: "wall-jump",
        at: Some((47 * TILE - player::W - 8, 25 * TILE - player::H)),
        steps: 140,
        intent: |k, _| {
            let base = if k < 50 {
                RIGHT
            } else {
                Intent {
                    move_x: -1.0,
                    ..Intent::default()
                }
            };
            let i = jump_at(k, 5, 20, base);
            if k >= 30 {
                jump_at(
                    k,
                    30,
                    20,
                    Intent {
                        move_x: if k < 50 { 1.0 } else { -1.0 },
                        ..Intent::default()
                    },
                )
            } else {
                i
            }
        },
    },
    Script {
        name: "dive",
        at: Some((15 * TILE, 17 * TILE - player::H)),
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
        at: Some((11 * TILE, 21 * TILE - player::H)),
        steps: 80,
        intent: |k, _| Intent {
            move_y: if (10..14).contains(&k) { 1.0 } else { 0.0 },
            ..jump_at(k, 12, 2, Intent::default())
        },
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
