//! [`Controls`]: game-toolkit's keyboard and gamepad input to an [`Intent`], through a
//! [`Scheme`] of keys and buttons.
//!
//! Each feel preset comes with a control scheme ([`Scheme::of`]); any other [`Scheme`] can
//! be built field by field. Movement is always the arrows, the left stick and the d-pad.
//!
//! | Action | `SNAPPY` (hybrid, snappy, drifter) | `PRECISE` | `MOMENTUM` |
//! |---|---|---|---|
//! | jump | Z, Space / A | C, Space / A, Y | Space, Z / A |
//! | dash | C / RT | X / X, B | - |
//! | super dash (hold) | S / LT | - | - |
//! | grab (hold) | D / LB | Z / RT, LT, RB, LB | - |
//! | dive | down + A / down + B, down + RB | - | - |
//! | sprint (hold) | Shift / X | - | Shift, X / X, RT |
//!
//! The layouts follow the default controls of the games the presets are tuned toward:
//! `SNAPPY` Hollow Knight's (dash on RT, the charged dash on LT, the dive as down + cast),
//! with grab and sprint added where it has none; `PRECISE` Celeste's; `MOMENTUM` Super Meat
//! Boy's, with sprint as its run button.
//!
//! Triggers count when the pad reports them as buttons or only as an analog axis (Z).

use game_toolkit::prelude::*;

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

impl Scheme {
    /// After Hollow Knight's default layout, plus grab and sprint. The hybrid, snappy and
    /// drifter presets' scheme.
    pub const SNAPPY: Scheme = Scheme {
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

    /// After Celeste's default layout. The precise preset's scheme.
    pub const PRECISE: Scheme = Scheme {
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

    /// After Super Meat Boy's: jump, and hold to run. The momentum preset's scheme.
    pub const MOMENTUM: Scheme = Scheme {
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

    /// The control scheme that goes with a feel preset (`None`: Hollow Knight's).
    pub fn of(preset: Option<Preset>) -> Self {
        match preset {
            Some(Preset::Precise) => Self::PRECISE,
            Some(Preset::Momentum) => Self::MOMENTUM,
            Some(Preset::Hybrid | Preset::Snappy | Preset::Drifter) | None => Self::SNAPPY,
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

/// Reads a scheme's keys and buttons into an [`Intent`] each frame (remembering the
/// gamepad triggers, which some pads only report as an axis).
#[derive(Clone, Copy, Debug)]
pub struct Controls {
    /// Which keys and buttons do what.
    pub scheme: Scheme,
    /// The triggers last frame.
    triggers: Triggers,
}

impl Controls {
    /// Controls with `scheme`.
    pub fn new(scheme: Scheme) -> Self {
        Self {
            scheme,
            triggers: Triggers::default(),
        }
    }

    /// The controls of `preset`'s game.
    pub fn for_preset(preset: Preset) -> Self {
        Self::new(Scheme::of(Some(preset)))
    }

    /// This frame's intent.
    pub fn read(&mut self, input: &Input) -> Intent {
        read_intent(input, &self.scheme, &mut self.triggers)
    }
}

impl Default for Controls {
    fn default() -> Self {
        Self::new(Scheme::SNAPPY)
    }
}
