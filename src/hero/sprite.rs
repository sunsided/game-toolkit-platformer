//! The hero's sprite numbers. Each is one pose; the renderer decides what a pose looks
//! like (placeholder shapes now, atlas cells once there is art).

/// Standing, breathing in.
pub const IDLE_A: u16 = 0;
/// Standing, breathing out.
pub const IDLE_B: u16 = 1;
/// Run cycle, contact.
pub const RUN_A: u16 = 2;
/// Run cycle, passing.
pub const RUN_B: u16 = 3;
/// Run cycle, other contact.
pub const RUN_C: u16 = 4;
/// Run cycle, other passing.
pub const RUN_D: u16 = 5;
/// Skidding into a turn.
pub const SKID: u16 = 6;
/// Pushing off the ground.
pub const TAKEOFF: u16 = 7;
/// Rising, stretched.
pub const RISE: u16 = 8;
/// Floating at the apex.
pub const APEX: u16 = 9;
/// Falling, cloak up.
pub const FALL_A: u16 = 10;
/// Falling, cloak flapping.
pub const FALL_B: u16 = 11;
/// Air-jump flip, a quarter turn per frame.
pub const FLIP: [u16; 4] = [12, 13, 14, 15];
/// Landing squash, deep.
pub const LAND_A: u16 = 16;
/// Landing squash, recovering.
pub const LAND_B: u16 = 17;
/// Hard landing, crouched on impact.
pub const HARD_A: u16 = 18;
/// Hard landing, getting up.
pub const HARD_B: u16 = 19;
/// Landing roll, a quarter turn per frame.
pub const ROLL: [u16; 4] = [20, 21, 22, 23];
/// Dashing sideways, two frames of streak.
pub const DASH_SIDE: [u16; 2] = [24, 25];
/// Dashing upward.
pub const DASH_UP: [u16; 2] = [26, 27];
/// Dashing downward.
pub const DASH_DOWN: [u16; 2] = [28, 29];
/// Climb cycle.
pub const CLIMB: [u16; 4] = [30, 31, 32, 33];
/// Sliding down a wall.
pub const WALL_SLIDE: [u16; 2] = [34, 35];
/// Pushing off a wall.
pub const WALL_JUMP: u16 = 36;
/// Hopping over the top of a wall.
pub const HOP: u16 = 37;
/// Dropping through a platform.
pub const DROP: u16 = 38;
/// Curling up before a dive.
pub const DIVE_WINDUP: u16 = 39;
/// Diving, two frames of streak.
pub const DIVE: [u16; 2] = [40, 41];
/// Dive impact.
pub const DIVE_LAND_A: u16 = 42;
/// Rising from the dive impact.
pub const DIVE_LAND_B: u16 = 43;
/// Looking up, raising the head.
pub const LOOK_UP_A: u16 = 44;
/// Looking up, held.
pub const LOOK_UP_B: u16 = 45;
/// Looking down, lowering the head.
pub const LOOK_DOWN_A: u16 = 46;
/// Looking down, held.
pub const LOOK_DOWN_B: u16 = 47;
/// Idle fidget after standing still a while: glance back, stretch, settle.
pub const FIDGET: [u16; 4] = [48, 49, 50, 51];
/// Air jump with wings (Monarch Wings): spread, beat down, glide, fold.
pub const WINGS: [u16; 4] = [52, 53, 54, 55];
/// Pushing a block, a leaning walk cycle.
pub const PUSH: [u16; 4] = [56, 57, 58, 59];
/// Pulling a block, leaning back, walking backwards.
pub const PULL: [u16; 4] = [60, 61, 62, 63];
/// Holding a block without moving.
pub const BOX_HOLD: u16 = 64;
/// Gliding under the open cloak, swaying.
pub const GLIDE: [u16; 2] = [65, 66];
/// Sprint cycle: longer strides, leaning further in.
pub const SPRINT: [u16; 4] = [67, 68, 69, 70];
/// Charging a super dash: crouched, gathering crystal light.
pub const CHARGE: [u16; 2] = [71, 72];
/// Fully charged, flashing: let go to launch.
pub const CHARGED: [u16; 2] = [73, 74];
/// Flying a super dash, stretched flat.
pub const SUPER: [u16; 2] = [75, 76];
/// The super dash hit a wall.
pub const SUPER_BONK: u16 = 77;
/// Number of sprites.
pub const COUNT: u16 = 78;
