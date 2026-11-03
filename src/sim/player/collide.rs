//! Moving through the level pixel by pixel: walls, floors, one-way platforms, ceilings and
//! corner corrections.

use super::Player;
use crate::sim::{Event, Rect, Terrain};

impl Player {
    /// Whether the hero stands on something: solid ground, or a platform while not
    /// dropping through.
    pub(super) fn stands(&self, level: &dyn Terrain) -> bool {
        let r = self.rect();
        level.solid(r.shifted(0, 1)) || (self.drop_through <= 0.0 && level.platform(r))
    }

    /// Whether falling with something to stand on within `px` pixels below.
    pub(super) fn ground_below(&self, level: &dyn Terrain, px: i32) -> bool {
        self.vy > 0.0
            && (0..px).any(|d| {
                let r = self.rect().shifted(0, d);
                level.solid(r.shifted(0, 1)) || level.platform(r)
            })
    }

    /// Move `amount` pixels sideways, one pixel at a time. Returns whether a wall stopped
    /// it.
    pub(super) fn move_x(&mut self, level: &dyn Terrain, amount: f32) -> bool {
        self.rem_x += amount;
        let n = self.rem_x.round() as i32;
        if n == 0 {
            return false;
        }
        self.rem_x -= n as f32;
        let s = n.signum();
        for _ in 0..n.abs() {
            if level.solid(self.rect().shifted(s, 0)) {
                self.rem_x = 0.0;
                return true;
            }
            self.x += s;
        }
        false
    }

    /// Carried by moving ground: `dx, dy` units, one at a time, stopping at whatever is
    /// solid at the edge moved toward (or, going down, at anything to stand on).
    pub(super) fn carry(&mut self, level: &dyn Terrain, dx: f32, dy: f32) {
        // Up first: ground that rose into the feet would block a sideways move.
        if dy < 0.0 {
            self.carry_y(level, dy);
            self.carry_x(level, dx);
        } else {
            self.carry_x(level, dx);
            self.carry_y(level, dy);
        }
        if self.grounded {
            self.peak_y = self.y;
        }
    }

    /// The sideways part of [`Player::carry`].
    fn carry_x(&mut self, level: &dyn Terrain, dx: f32) {
        self.rem_x += dx;
        let n = self.rem_x.round() as i32;
        self.rem_x -= n as f32;
        let s = n.signum();
        for _ in 0..n.abs() {
            let r = self.rect();
            let x = if s > 0 { r.right() } else { r.x - 1 };
            if level.solid(Rect { x, w: 1, ..r }) {
                self.rem_x = 0.0;
                break;
            }
            self.x += s;
        }
    }

    /// The vertical part of [`Player::carry`].
    fn carry_y(&mut self, level: &dyn Terrain, dy: f32) {
        self.rem_y += dy;
        let n = self.rem_y.round() as i32;
        self.rem_y -= n as f32;
        let s = n.signum();
        for _ in 0..n.abs() {
            let r = self.rect();
            let blocked = if s > 0 {
                self.stands(level)
            } else {
                level.solid(Rect {
                    y: r.y - 1,
                    h: 1,
                    ..r
                })
            };
            if blocked {
                self.rem_y = 0.0;
                break;
            }
            self.y += s;
        }
    }

    /// Move `amount` pixels vertically, one pixel at a time. Going up into a ceiling
    /// corner, the head is nudged up to `corner` pixels sideways around it; otherwise a
    /// ceiling stops the jump, and the floor (or a platform) stops the fall.
    pub(super) fn move_y(
        &mut self,
        level: &dyn Terrain,
        amount: f32,
        corner: i32,
        ev: &mut Vec<Event>,
    ) {
        self.rem_y += amount;
        let n = self.rem_y.round() as i32;
        if n == 0 {
            return;
        }
        self.rem_y -= n as f32;
        let s = n.signum();
        for _ in 0..n.abs() {
            let blocked = if s > 0 {
                self.stands(level)
            } else {
                level.solid(self.rect().shifted(0, -1))
            };
            if !blocked {
                self.y += s;
                continue;
            }
            if s < 0
                && let Some(dx) = self.corner_nudge(level, corner)
            {
                self.x += dx;
                self.y += s;
                ev.push(Event::CornerCorrected { dx });
                continue;
            }
            self.rem_y = 0.0;
            self.vy = 0.0;
            if s < 0 {
                self.stop_jump();
                ev.push(Event::Bonked);
            }
            return;
        }
    }

    /// The smallest sideways shift (up to `max` pixels, the moving direction first) that
    /// lets the head pass a ceiling corner one pixel up.
    fn corner_nudge(&self, level: &dyn Terrain, max: i32) -> Option<i32> {
        let first = if self.vx < 0.0 { -1 } else { 1 };
        (1..=max).flat_map(|d| [d * first, -d * first]).find(|&dx| {
            !level.solid(self.rect().shifted(dx, 0)) && !level.solid(self.rect().shifted(dx, -1))
        })
    }
}
