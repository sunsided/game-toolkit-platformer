//! [`read_intent`]: keyboard and gamepad to an [`Intent`]; [`read_keys`]: the clock and
//! debug keys.
//!
//! The defaults follow Hollow Knight's: dash on the right trigger (keyboard C), the dive
//! is down + cast (B, or quick cast on RB; keyboard down + A), as Desolate Dive is. Grab,
//! which Hollow Knight doesn't have, takes the super-dash slot (LT, also LB; keyboard S):
//! it holds walls and blocks.
//!
//! | Action | Keyboard | Gamepad (Xbox names) |
//! |---|---|---|
//! | move, aim, climb, look | arrows | left stick, d-pad |
//! | jump | Z (also Space) | A |
//! | dash | C | RT |
//! | grab (hold) | S | LT, LB |
//! | dive | down + A | down + B, down + RB |

use game_toolkit::prelude::*;

use crate::session::Keys;
use crate::sim::Intent;

/// Stick values below this are ignored.
const DEADZONE: f32 = 0.25;

/// Read this frame's intent from the keyboard and the first gamepad.
pub fn read_intent(input: &Input) -> Intent {
    let jump = [Key::KeyZ, Key::Space];
    let axis =
        |neg: Key, pos: Key| (input.key_held(pos) as i32 - input.key_held(neg) as i32) as f32;
    let mut i = Intent {
        move_x: axis(Key::ArrowLeft, Key::ArrowRight),
        move_y: axis(Key::ArrowUp, Key::ArrowDown),
        jump_pressed: jump.iter().any(|&k| input.key_pressed(k)),
        jump_held: jump.iter().any(|&k| input.key_held(k)),
        dash_pressed: input.key_pressed(Key::KeyC),
        grab: input.key_held(Key::KeyS),
        dive_pressed: false,
    };
    let mut cast = input.key_pressed(Key::KeyA);
    if let Some(g) = input.first_gamepad() {
        let dz = |v: f32| if v.abs() < DEADZONE { 0.0 } else { v };
        let dpad = |neg, pos| (g.button_held(pos) as i32 - g.button_held(neg) as i32) as f32;
        let sx = dz(g.axis(Axis::LeftStickX)) + dpad(Button::DPadLeft, Button::DPadRight);
        // gilrs: stick up is positive; the sim's y points down.
        let sy = -dz(g.axis(Axis::LeftStickY)) + dpad(Button::DPadUp, Button::DPadDown);
        if sx != 0.0 {
            i.move_x = sx.clamp(-1.0, 1.0);
        }
        if sy != 0.0 {
            i.move_y = sy.clamp(-1.0, 1.0);
        }
        // gilrs: RightTrigger2 / LeftTrigger2 are the triggers, RightTrigger / LeftTrigger
        // the bumpers.
        i.jump_pressed |= g.button_pressed(Button::South);
        i.jump_held |= g.button_held(Button::South);
        i.dash_pressed |= g.button_pressed(Button::RightTrigger2);
        i.grab |= g.button_held(Button::LeftTrigger2) || g.button_held(Button::LeftTrigger);
        cast |= g.button_pressed(Button::East) || g.button_pressed(Button::RightTrigger);
    }
    i.dive_pressed = cast && i.down();
    i
}

/// Read the clock and debug keys: P pause, `.` step, `[` `]` slower/faster, `\` normal,
/// F1 overlay (compact, full, off), F2 next feel preset, R back to the spawn.
pub fn read_keys(input: &Input) -> Keys {
    let p = |k| input.key_pressed(k);
    Keys {
        pause: p(Key::KeyP),
        step: p(Key::Period),
        slower: p(Key::BracketLeft),
        faster: p(Key::BracketRight),
        normal: p(Key::Backslash),
        overlay: p(Key::F1),
        reset: p(Key::KeyR),
        preset: p(Key::F2),
    }
}
