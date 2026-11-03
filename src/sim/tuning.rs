//! [`Tuning`]: every number that shapes how the hero moves. Grid pixels and seconds.
//!
//! The defaults ([`Preset::Hybrid`]) sit near Celeste's (same 8 px tiles: run, jump,
//! dash, climb), with Hollow Knight's heavier fall, jump cut, double jump, dive and landing
//! roll mixed in. The other [`Preset`]s lean all the way toward one game's feel; abilities
//! a preset doesn't have are switched off (zero counts, false flags, unreachable drops).

/// A named starting point for [`Tuning`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    /// Everything on: the default.
    Hybrid,
    /// Precise and floaty: 8-way dash, climbing with stamina, no air jump, no cut.
    Celeste,
    /// Snappy: instant run, hard jump cut, double jump, horizontal dash, wall cling that
    /// slips, dive, hard landings.
    HollowKnight,
    /// Fast and slippery: momentum runs, strong wall jumps, wall stick, nothing else.
    MeatBoy,
}

impl Preset {
    /// Every preset, in cycling order.
    pub const ALL: [Preset; 4] = [
        Preset::Hybrid,
        Preset::Celeste,
        Preset::HollowKnight,
        Preset::MeatBoy,
    ];

    /// Display name.
    pub fn name(self) -> &'static str {
        match self {
            Preset::Hybrid => "hybrid",
            Preset::Celeste => "celeste-like",
            Preset::HollowKnight => "hollow-knight-like",
            Preset::MeatBoy => "meat-boy-like",
        }
    }

    /// The next preset, wrapping.
    pub fn next(self) -> Preset {
        let i = Self::ALL.iter().position(|&p| p == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    /// The tuning.
    pub fn tuning(self) -> Tuning {
        let base = Tuning::default();
        let never = i32::MAX;
        match self {
            Preset::Hybrid => base,
            Preset::Celeste => Tuning {
                fall_gravity: 1.0,
                jump_cut: 1.0,
                air_jumps: 0,
                dive: false,
                roll_drop: never,
                hard_land_drop: never,
                wall_stick_time: 0.0,
                ..base
            },
            Preset::HollowKnight => Tuning {
                run_speed: 83.0,
                run_accel: 4000.0,
                turn_accel: 4000.0,
                run_decel: 4000.0,
                air_control: 1.0,
                jump_cut: 0.15,
                apex_gravity: 0.75,
                dash_eight_way: false,
                dash_end_speed: 40.0,
                grab: false,
                wall_slide_speed: 60.0,
                wall_jump_lock: 0.2,
                roll_drop: never,
                ..base
            },
            Preset::MeatBoy => Tuning {
                run_speed: 150.0,
                run_accel: 600.0,
                turn_accel: 1400.0,
                run_decel: 900.0,
                overspeed_decel: 200.0,
                air_control: 0.9,
                max_fall: 220.0,
                fast_fall: 300.0,
                jump_speed: 120.0,
                var_jump_time: 0.18,
                jump_cut: 0.5,
                coyote_time: 0.06,
                air_jumps: 0,
                dashes: 0,
                grab: false,
                dive: false,
                wall_slide_speed: 50.0,
                wall_jump_speed: 170.0,
                wall_jump_lock: 0.05,
                wall_stick_time: 0.12,
                roll_drop: never,
                hard_land_drop: never,
                ..base
            },
        }
    }
}

/// The movement feel. All speeds px/s, accelerations px/s², times seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tuning {
    /// Top running speed.
    pub run_speed: f32,
    /// Acceleration toward the stick's speed on the ground.
    pub run_accel: f32,
    /// Acceleration when the stick points against the motion (turning around).
    pub turn_accel: f32,
    /// Deceleration with the stick released.
    pub run_decel: f32,
    /// Deceleration toward top speed when moving faster than it (after a roll jump or a
    /// dash), so momentum carries.
    pub overspeed_decel: f32,
    /// Multiplier on all horizontal acceleration in the air.
    pub air_control: f32,

    /// Gravity.
    pub gravity: f32,
    /// Gravity multiplier while falling, for a heavier way down.
    pub fall_gravity: f32,
    /// Below this vertical speed (either way) with jump held, the hero is "at the apex".
    pub apex_threshold: f32,
    /// Gravity multiplier at the apex with jump held: the hangtime.
    pub apex_gravity: f32,
    /// Terminal falling speed.
    pub max_fall: f32,
    /// Terminal falling speed with down held.
    pub fast_fall: f32,

    /// Upward speed of a ground jump.
    pub jump_speed: f32,
    /// Horizontal speed added by a ground jump in the stick's direction.
    pub jump_boost: f32,
    /// Seconds after a jump during which holding jump keeps the full upward speed.
    pub var_jump_time: f32,
    /// Releasing jump while rising multiplies the upward speed by this (1: no cut).
    pub jump_cut: f32,
    /// Seconds after leaving a ledge during which a ground jump still works.
    pub coyote_time: f32,
    /// Seconds a jump press waits for the jump to become possible.
    pub jump_buffer: f32,
    /// Seconds after dropping through a platform during which platforms don't hold.
    pub drop_through_time: f32,

    /// Jumps available in the air before landing again.
    pub air_jumps: u8,
    /// Upward speed of an air jump.
    pub double_jump_speed: f32,
    /// Hold window of an air jump, like [`Tuning::var_jump_time`].
    pub double_var_jump_time: f32,
    /// Falling with ground this many pixels below, a jump press waits for the landing
    /// (a queued ground jump) instead of spending the air jump.
    pub land_jump_lookahead: i32,

    /// Dashes available before touching ground or a crystal again (0: no dash).
    pub dashes: u8,
    /// Dashes go eight ways; otherwise only sideways.
    pub dash_eight_way: bool,
    /// Speed during a dash.
    pub dash_speed: f32,
    /// Length of a dash.
    pub dash_time: f32,
    /// Speed handed over when a dash ends.
    pub dash_end_speed: f32,
    /// Multiplier on the handed-over speed of upward dashes.
    pub dash_end_up: f32,
    /// Seconds after a dash starts before another one can.
    pub dash_cooldown: f32,
    /// Seconds a dash press waits for a dash to become possible.
    pub dash_buffer: f32,
    /// Seconds into a dash before the ground refills dashes (so dashing along the floor
    /// isn't free).
    pub dash_recharge_after: f32,
    /// Seconds before a used crystal comes back.
    pub crystal_respawn: f32,

    /// Grabbing walls and climbing is possible at all.
    pub grab: bool,
    /// Holding still on a wall slides down at this speed: 0 holds firm, more is a grip
    /// that slips.
    pub slip_speed: f32,
    /// Below this stamina even a firm grip slips, at [`Tuning::tired_slip_speed`].
    pub tired_stamina: f32,
    /// How fast a tired grip slips.
    pub tired_slip_speed: f32,
    /// Stamina when full; climbing spends it, the ground refills it.
    pub stamina: f32,
    /// Climbing speed up a wall.
    pub climb_up_speed: f32,
    /// Climbing speed down a wall.
    pub climb_down_speed: f32,
    /// Stamina per second climbing up.
    pub climb_up_cost: f32,
    /// Stamina per second holding still on a wall.
    pub climb_still_cost: f32,
    /// Stamina for a jump straight up a wall.
    pub climb_jump_cost: f32,
    /// Upward speed of the hop over the top of a wall.
    pub climb_hop_speed: f32,
    /// Terminal falling speed pressing into a wall without grabbing.
    pub wall_slide_speed: f32,
    /// Horizontal speed of a wall jump.
    pub wall_jump_speed: f32,
    /// Seconds after a wall jump during which the stick can't pull back to the wall.
    pub wall_jump_lock: f32,
    /// A wall this many pixels away still counts for a wall jump.
    pub wall_jump_reach: i32,
    /// Sliding down a wall, pressing away keeps the hero stuck to it this long, so
    /// "away + jump" becomes a wall jump instead of a fall.
    pub wall_stick_time: f32,

    /// Diving is possible at all.
    pub dive: bool,

    /// Seconds hanging in the air before a dive plunges.
    pub dive_windup: f32,
    /// Speed of a dive.
    pub dive_speed: f32,
    /// How long a dive's impact takes away control.
    pub dive_land_time: f32,

    /// Fall height in pixels from which a landing while moving becomes a roll.
    pub roll_drop: i32,
    /// Roll speed (the roll keeps any faster speed it lands with).
    pub roll_speed: f32,
    /// Length of a roll.
    pub roll_time: f32,
    /// Seconds into a roll before it can be cancelled into a jump.
    pub roll_jump_after: f32,
    /// Fall height in pixels from which a landing without moving is a hard landing.
    pub hard_land_drop: i32,
    /// How long a hard landing takes away control.
    pub hard_land_time: f32,

    /// Pixels the head may be nudged sideways around a ceiling corner.
    pub corner_correction: i32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            run_speed: 90.0,
            run_accel: 1000.0,
            turn_accel: 1600.0,
            run_decel: 1200.0,
            overspeed_decel: 400.0,
            air_control: 0.65,

            gravity: 900.0,
            fall_gravity: 1.15,
            apex_threshold: 40.0,
            apex_gravity: 0.5,
            max_fall: 160.0,
            fast_fall: 240.0,

            jump_speed: 105.0,
            jump_boost: 40.0,
            var_jump_time: 0.2,
            jump_cut: 0.6,
            coyote_time: 0.1,
            jump_buffer: 0.1,
            drop_through_time: 0.15,

            air_jumps: 1,
            double_jump_speed: 100.0,
            double_var_jump_time: 0.12,
            land_jump_lookahead: 6,

            dashes: 1,
            dash_eight_way: true,
            dash_speed: 240.0,
            dash_time: 0.15,
            dash_end_speed: 160.0,
            dash_end_up: 0.75,
            dash_cooldown: 0.2,
            dash_buffer: 0.08,
            dash_recharge_after: 0.1,
            crystal_respawn: 2.5,

            grab: true,
            slip_speed: 0.0,
            tired_stamina: 20.0,
            tired_slip_speed: 20.0,
            stamina: 110.0,
            climb_up_speed: 45.0,
            climb_down_speed: 80.0,
            climb_up_cost: 45.0,
            climb_still_cost: 10.0,
            climb_jump_cost: 27.5,
            climb_hop_speed: 120.0,
            wall_slide_speed: 40.0,
            wall_jump_speed: 130.0,
            wall_jump_lock: 0.16,
            wall_jump_reach: 2,
            wall_stick_time: 0.1,

            dive: true,

            dive_windup: 0.08,
            dive_speed: 320.0,
            dive_land_time: 0.2,

            roll_drop: 48,
            roll_speed: 120.0,
            roll_time: 0.32,
            roll_jump_after: 0.05,
            hard_land_drop: 96,
            hard_land_time: 0.3,

            corner_correction: 3,
        }
    }
}
