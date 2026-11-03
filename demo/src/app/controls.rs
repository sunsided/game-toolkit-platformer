//! [`read_keys`]: the demo's clock and debug keys. Reading movement input is the library's
//! job ([`game_toolkit_platformer::controls`]).

use game_toolkit::prelude::*;

use crate::session::Keys;

/// Read the clock and debug keys: P pause, `.` step, `[` `]` slower/faster, `\` normal,
/// F1 overlay (compact, full, off), F2 next feel preset, F3 art or diagnostic look, R back to the spawn.
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
        skin: p(Key::F3),
    }
}
