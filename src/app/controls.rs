//! [`read_intent`]: keyboard and gamepad to an [`Intent`] through a [`Scheme`];
//! [`read_keys`]: the clock and debug keys.
//!
//! Each feel preset brings its game's control scheme ([`Scheme::of`]); F2 switches both.
//! Movement is always the arrows, the left stick and the d-pad.
//!
//! | Action | Hollow Knight (hybrid, Silksong too) | Celeste | Super Meat Boy |
//! |---|---|---|---|
//! | jump | Z, Space / A | C, Space / A, Y | Space, Z / A |
//! | dash | C / RT | X / X, B | - |
//! | super dash (hold) | S / LT | - | - |
//! | grab (hold) | D / LB | Z / RT, LT, RB, LB | - |
//! | dive | down + A / down + B, down + RB | - | - |
//! | sprint (hold) | Shift / X | - | Shift, X / X, RT |
//!
//! Hollow Knight's own slots: dash RT, super dash (Crystal Heart) LT, dive down + cast.
//! Grab, which it doesn't have, takes the dream nail's keyboard key (D) and the left
//! bumper. Sprint is Super Meat Boy's run button. Silksong's own layout isn't modeled.
//!
//! Triggers count when the pad reports them as buttons or only as an analog axis (Z).

use game_toolkit::prelude::*;

use crate::session::Keys;
use crate::sim::{Intent, Preset};

/// Stick values below this are ignored.
const DEADZONE: f32 = 0.25;

/// Which keys and buttons do what. An action with no keys and no buttons can't be done.
#[derive(Clone, Copy, Debug)]
pub struct Scheme {
    /// Jump keys.
    pub jump: &'static [Key],
    /// Jump buttons.
    pub jump_pad: &'static [Button],
    /// Dash keys.
    pub dash: &'static [Key],
    /// Dash buttons.
    pub dash_pad: &'static [Button],
    /// Grab keys (held).
    pub grab: &'static [Key],
    /// Grab buttons (held).
    pub grab_pad: &'static [Button],
    /// Cast keys: with down held, a dive.
    pub cast: &'static [Key],
    /// Cast buttons.
    pub cast_pad: &'static [Button],
    /// Sprint keys (held).
    pub sprint: &'static [Key],
    /// Sprint buttons (held).
    pub sprint_pad: &'static [Button],
    /// Super dash keys (held).
    pub super_dash: &'static [Key],
    /// Super dash buttons (held).
    pub super_dash_pad: &'static [Button],
}

// gilrs names: RightTrigger2 / LeftTrigger2 are the triggers, RightTrigger / LeftTrigger
// the bumpers; South, East, West, North are A, B, X, Y.

/// Hollow Knight's defaults plus grab and sprint. Also the hybrid's and Silksong's.
const HOLLOW_KNIGHT: Scheme = Scheme {
    jump: &[Key::KeyZ, Key::Space],
    jump_pad: &[Button::South],
    dash: &[Key::KeyC],
    dash_pad: &[Button::RightTrigger2],
    grab: &[Key::KeyD],
    grab_pad: &[Button::LeftTrigger],
    cast: &[Key::KeyA],
    cast_pad: &[Button::East, Button::RightTrigger],
    sprint: &[Key::ShiftLeft, Key::ShiftRight],
    sprint_pad: &[Button::West],
    super_dash: &[Key::KeyS],
    super_dash_pad: &[Button::LeftTrigger2],
};

/// Celeste's defaults.
const CELESTE: Scheme = Scheme {
    jump: &[Key::KeyC, Key::Space],
    jump_pad: &[Button::South, Button::North],
    dash: &[Key::KeyX],
    dash_pad: &[Button::West, Button::East],
    grab: &[Key::KeyZ],
    grab_pad: &[
        Button::RightTrigger2,
        Button::LeftTrigger2,
        Button::RightTrigger,
        Button::LeftTrigger,
    ],
    cast: &[],
    cast_pad: &[],
    sprint: &[],
    sprint_pad: &[],
    super_dash: &[],
    super_dash_pad: &[],
};

/// Super Meat Boy: jump, and hold to run.
const MEAT_BOY: Scheme = Scheme {
    jump: &[Key::Space, Key::KeyZ],
    jump_pad: &[Button::South],
    dash: &[],
    dash_pad: &[],
    grab: &[],
    grab_pad: &[],
    cast: &[],
    cast_pad: &[],
    sprint: &[Key::ShiftLeft, Key::ShiftRight, Key::KeyX],
    sprint_pad: &[Button::West, Button::RightTrigger2],
    super_dash: &[],
    super_dash_pad: &[],
};

impl Scheme {
    /// The control scheme that goes with a feel preset (`None`: Hollow Knight's).
    pub fn of(preset: Option<Preset>) -> Self {
        match preset {
            Some(Preset::Celeste) => CELESTE,
            Some(Preset::MeatBoy) => MEAT_BOY,
            // Silksong's own layout isn't modeled; it gets its predecessor's.
            Some(Preset::Hybrid | Preset::HollowKnight | Preset::Silksong) | None => HOLLOW_KNIGHT,
        }
    }
}

/// Trigger state from the last frame, to see presses of triggers that only report an
/// analog axis.
#[derive(Clone, Copy, Debug, Default)]
pub struct Triggers {
    /// Left trigger was down.
    left: bool,
    /// Right trigger was down.
    right: bool,
}

/// A trigger counts as down past this.
const TRIGGER: f32 = 0.5;

/// Read this frame's intent from the keyboard and the first gamepad, through `scheme`.
pub fn read_intent(input: &Input, scheme: &Scheme, triggers: &mut Triggers) -> Intent {
    let pressed = |ks: &[Key]| ks.iter().any(|&k| input.key_pressed(k));
    let held = |ks: &[Key]| ks.iter().any(|&k| input.key_held(k));
    let axis =
        |neg: Key, pos: Key| (input.key_held(pos) as i32 - input.key_held(neg) as i32) as f32;
    let mut i = Intent {
        move_x: axis(Key::ArrowLeft, Key::ArrowRight),
        move_y: axis(Key::ArrowUp, Key::ArrowDown),
        jump_pressed: pressed(scheme.jump),
        jump_held: held(scheme.jump),
        dash_pressed: pressed(scheme.dash),
        grab: held(scheme.grab),
        dive_pressed: false,
        sprint: held(scheme.sprint),
        super_dash: held(scheme.super_dash),
    };
    let mut cast = pressed(scheme.cast);
    if let Some(g) = input.first_gamepad() {
        // Triggers: as buttons, or as the Z axes some pads report instead.
        let lt = g.button_held(Button::LeftTrigger2) || g.axis(Axis::LeftZ) > TRIGGER;
        let rt = g.button_held(Button::RightTrigger2) || g.axis(Axis::RightZ) > TRIGGER;
        let (lt_pressed, rt_pressed) = (lt && !triggers.left, rt && !triggers.right);
        *triggers = Triggers {
            left: lt,
            right: rt,
        };
        let held_b = |b: Button| match b {
            Button::LeftTrigger2 => lt,
            Button::RightTrigger2 => rt,
            _ => g.button_held(b),
        };
        let pressed_b = |b: Button| match b {
            Button::LeftTrigger2 => lt_pressed,
            Button::RightTrigger2 => rt_pressed,
            _ => g.button_pressed(b),
        };
        let pressed = |bs: &[Button]| bs.iter().any(|&b| pressed_b(b));
        let held = |bs: &[Button]| bs.iter().any(|&b| held_b(b));
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
        i.jump_pressed |= pressed(scheme.jump_pad);
        i.jump_held |= held(scheme.jump_pad);
        i.dash_pressed |= pressed(scheme.dash_pad);
        i.grab |= held(scheme.grab_pad);
        i.sprint |= held(scheme.sprint_pad);
        i.super_dash |= held(scheme.super_dash_pad);
        cast |= pressed(scheme.cast_pad);
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
