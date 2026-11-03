//! [`Pose`]: what each sprite number means for the body: squash and stretch, lean, lift.
//! Both renderers (the mouse and the debug blocks) and the cape and tail simulation read
//! it, so a sprite has one shape everywhere.

use super::sprite as s;
use crate::gfx::{Rgb, palette as pal};

/// One pose: the body's size, lean and lift, its debug color, and wings if spread.
#[derive(Clone, Copy, Debug)]
pub struct Pose {
    /// Block width.
    pub w: f32,
    /// Block height.
    pub h: f32,
    /// Rotation in degrees, positive leaning forward.
    pub rot: f32,
    /// Vertical offset of the feet, pixels (negative up).
    pub dy: f32,
    /// Body color.
    pub c: Rgb,
    /// Wings spread at this angle from straight back, degrees (Monarch Wings frames).
    pub wings: Option<f32>,
}

/// A pose of `w` x `h`, rotated `rot` degrees, colored `c`.
const fn p(w: f32, h: f32, rot: f32, c: Rgb) -> Pose {
    Pose {
        w,
        h,
        rot,
        dy: 0.0,
        c,
        wings: None,
    }
}

/// The pose for `sprite`.
pub fn pose(sprite: u16) -> Pose {
    let (g, a, r, d, m, y) = (
        pal::ORANGE,
        pal::CYAN,
        pal::RED,
        pal::YELLOW,
        pal::MAGENTA,
        pal::GREEN,
    );
    match sprite {
        // Standing tall, leaning back a touch, chest out: the heroic stance.
        s::IDLE_A => p(6.0, 12.0, -4.0, g),
        s::IDLE_B => p(6.4, 11.6, -4.0, g),
        s::LOOK_UP_A => p(6.0, 12.0, -10.0, g),
        s::LOOK_UP_B => p(6.0, 13.0, -18.0, g),
        s::LOOK_DOWN_A => p(7.0, 11.0, 12.0, g),
        s::LOOK_DOWN_B => p(8.0, 9.0, 22.0, g),
        // The fidget: reach for the hilt, raise the sword to the sky (chin up), hold it
        // with a glint, sheathe it.
        48 => p(6.0, 12.0, -2.0, g),
        49 => p(6.0, 12.4, -8.0, pal::PALE),
        50 => p(6.0, 12.6, -9.0, pal::PALE),
        51 => p(6.2, 11.8, -3.0, g),
        52..=55 => {
            let spread = [60.0, 150.0, 100.0, 30.0][(sprite - 52) as usize];
            Pose {
                wings: Some(spread),
                dy: -1.0,
                ..p(6.0, 12.0, 0.0, a)
            }
        }
        56..=59 => Pose {
            dy: -(((sprite - 56) % 2) as f32),
            ..p(7.0, 11.0, 24.0, g)
        },
        60..=63 => Pose {
            dy: -(((sprite - 60) % 2) as f32),
            ..p(7.0, 11.0, -20.0, g)
        },
        s::BOX_HOLD => p(7.0, 11.0, 6.0, g),
        71 | 72 => Pose {
            dy: -(((sprite - 71) % 2) as f32),
            ..p(8.0, 9.0, 8.0, pal::YELLOW)
        },
        73 => p(9.0, 8.0, 10.0, pal::WHITE),
        74 => p(9.0, 8.0, 10.0, pal::YELLOW),
        75 | 76 => p(13.0 + (sprite - 75) as f32, 7.0, 0.0, pal::YELLOW),
        s::SUPER_BONK => p(7.0, 10.0, -25.0, r),
        67 | 69 => p(8.0, 10.0, 20.0, g),
        68 | 70 => Pose {
            dy: -1.0,
            ..p(7.0, 11.0, 15.0, g)
        },
        65 | 66 => Pose {
            wings: Some(if sprite == 65 { 80.0 } else { 95.0 }),
            ..p(6.0, 11.0, if sprite == 65 { 4.0 } else { -4.0 }, a)
        },
        s::RUN_A | s::RUN_C => p(7.0, 11.0, 10.0, g),
        s::RUN_B | s::RUN_D => Pose {
            dy: -1.0,
            ..p(6.0, 12.0, 6.0, g)
        },
        s::SKID => p(8.0, 10.0, -18.0, g),
        s::TAKEOFF => p(8.0, 9.0, 0.0, a),
        s::RISE => p(5.0, 14.0, 0.0, a),
        s::APEX => p(7.0, 11.0, 0.0, a),
        s::FALL_A => p(6.0, 13.0, 0.0, a),
        s::FALL_B => p(5.0, 13.0, 4.0, a),
        12..=15 => Pose {
            dy: -2.0,
            ..p(7.0, 10.0, 90.0 * (sprite - 12) as f32, pal::PALE)
        },
        s::LAND_A => p(9.0, 8.0, 0.0, g),
        s::LAND_B => p(7.0, 10.0, 0.0, g),
        s::HARD_A => p(10.0, 6.0, 0.0, r),
        s::HARD_B => p(8.0, 9.0, 0.0, r),
        20..=23 => Pose {
            dy: -1.0,
            ..p(8.0, 8.0, 90.0 * (sprite - 20) as f32, m)
        },
        24 | 25 => p(11.0 + (sprite - 24) as f32, 8.0, 0.0, d),
        26 | 27 => p(5.0, 14.0 + (sprite - 26) as f32, 0.0, d),
        28 | 29 => p(5.0, 14.0 + (sprite - 28) as f32, 180.0, d),
        30..=33 => Pose {
            dy: -(((sprite - 30) % 2) as f32),
            ..p(6.0, 12.0, -6.0, y)
        },
        34 | 35 => p(6.0, 12.0 - (sprite - 34) as f32, -10.0, y),
        s::WALL_JUMP => p(8.0, 10.0, 20.0, a),
        s::HOP => p(7.0, 10.0, 15.0, y),
        s::DROP => p(5.0, 13.0, 0.0, a),
        s::DIVE_WINDUP => Pose {
            dy: -2.0,
            ..p(8.0, 8.0, 0.0, r)
        },
        40 | 41 => p(5.0, 15.0 + (sprite - 40) as f32, 180.0, r),
        s::DIVE_LAND_A => p(12.0, 6.0, 0.0, r),
        s::DIVE_LAND_B => p(8.0, 9.0, 0.0, r),
        _ => p(6.0, 12.0, 0.0, pal::WHITE),
    }
}
