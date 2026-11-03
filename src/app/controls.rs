//! [`read_intent`]: keyboard and gamepad to an [`Intent`]; [`read_keys`]: the clock and
//! debug keys.
//!
//! | Action | Keyboard | Gamepad |
//! |---|---|---|
//! | move, aim, climb | arrows / WASD | left stick, d-pad |
//! | jump | Space, C, K | South (A) |
//! | dash | X, J | West (X) |
//! | grab (hold) | Z, L, Shift | right shoulder / trigger |
//! | dive | V, I | East (B) |

use game_toolkit::prelude::*;

use crate::session::Keys;
use crate::sim::Intent;

/// Stick values below this are ignored.
const DEADZONE: f32 = 0.25;

/// Read this frame's intent from the keyboard and the first gamepad.
pub fn read_intent(input: &Input) -> Intent {
    let held = |ks: &[Key]| ks.iter().any(|&k| input.key_held(k));
    let pressed = |ks: &[Key]| ks.iter().any(|&k| input.key_pressed(k));
    let axis = |neg: &[Key], pos: &[Key]| (held(pos) as i32 - held(neg) as i32) as f32;
    let jump = [Key::Space, Key::KeyC, Key::KeyK];
    let dash = [Key::KeyX, Key::KeyJ];
    let grab = [Key::KeyZ, Key::KeyL, Key::ShiftLeft, Key::ShiftRight];
    let dive = [Key::KeyV, Key::KeyI];
    let mut i = Intent {
        move_x: axis(&[Key::ArrowLeft, Key::KeyA], &[Key::ArrowRight, Key::KeyD]),
        move_y: axis(&[Key::ArrowUp, Key::KeyW], &[Key::ArrowDown, Key::KeyS]),
        jump_pressed: pressed(&jump),
        jump_held: held(&jump),
        dash_pressed: pressed(&dash),
        grab: held(&grab),
        dive_pressed: pressed(&dive),
    };
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
        i.jump_pressed |= g.button_pressed(Button::South);
        i.jump_held |= g.button_held(Button::South);
        i.dash_pressed |= g.button_pressed(Button::West);
        i.grab |= g.button_held(Button::RightTrigger) || g.button_held(Button::RightTrigger2);
        i.dive_pressed |= g.button_pressed(Button::East);
    }
    i
}

/// Read the clock and debug keys: P pause, `.` step, `[` `]` slower/faster, `\` normal,
/// F1 debug overlay, F2 next feel preset, R back to the spawn.
pub fn read_keys(input: &Input) -> Keys {
    let p = |k| input.key_pressed(k);
    Keys {
        pause: p(Key::KeyP),
        step: p(Key::Period),
        slower: p(Key::BracketLeft),
        faster: p(Key::BracketRight),
        normal: p(Key::Backslash),
        debug: p(Key::F1),
        reset: p(Key::KeyR),
        preset: p(Key::F2),
    }
}
