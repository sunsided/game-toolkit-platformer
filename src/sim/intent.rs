//! [`Intent`]: what the player wants this step, independent of keys or pads.

/// One step's input. Edges (`*_pressed`) are true for exactly one step; the session
/// latches presses that land between steps so none get lost in slow motion.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Intent {
    /// Horizontal stick, -1 (left) to 1 (right).
    pub move_x: f32,
    /// Vertical stick, -1 (up) to 1 (down). Down fast-falls, aims dashes, climbs down and,
    /// with jump, drops through platforms.
    pub move_y: f32,
    /// Jump was pressed since the last step.
    pub jump_pressed: bool,
    /// Jump is held.
    pub jump_held: bool,
    /// Dash was pressed since the last step.
    pub dash_pressed: bool,
    /// Grab is held: hold on to walls.
    pub grab: bool,
    /// Dive was pressed since the last step.
    pub dive_pressed: bool,
    /// Sprint is held: a higher top speed.
    pub sprint: bool,
    /// Super dash is held: charge it (on the ground or a wall); release to launch.
    pub super_dash: bool,
    /// Your game's own buttons that are held (attack, interact, ...), one bit each; the
    /// movement ignores them. Bind keys to them with `controls::Scheme::custom`.
    pub custom: u32,
    /// Your game's own buttons pressed since the last step, one bit each: latched by the
    /// `Platformer` like the built-in presses, so none get lost between steps.
    pub custom_pressed: u32,
}

impl Intent {
    /// The stick points down.
    pub fn down(&self) -> bool {
        self.move_y > 0.5
    }

    /// Whether custom button `bit` (0 to 31) is held.
    pub fn held(&self, bit: u8) -> bool {
        self.custom & (1 << bit) != 0
    }

    /// Whether custom button `bit` (0 to 31) was pressed since the last step.
    pub fn pressed(&self, bit: u8) -> bool {
        self.custom_pressed & (1 << bit) != 0
    }
}
