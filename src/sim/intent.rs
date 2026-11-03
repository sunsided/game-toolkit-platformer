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
}

impl Intent {
    /// The stick points down.
    pub fn down(&self) -> bool {
        self.move_y > 0.5
    }
}
