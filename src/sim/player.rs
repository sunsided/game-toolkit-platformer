//! [`Player`]: the hero's body: position, speed, timers, and the movement rules.
//!
//! [`Player::step`] runs the abilities in a fixed order, each in its own module:
//!
//! 1. timers: jump and dash buffers, coyote time, refills on the ground, mode clocks
//! 2. [`dash`](self::dash): start a buffered dash (8-way, from the stick)
//! 3. [`dive`](self::dive): start a dive in the air; the dive's own motion
//! 4. [`climb`](self::climb): grab, climb, hop over the top, run out of stamina
//! 5. running (horizontal speed by mode)
//! 6. [`jump`](self::jump): ground, coyote, roll, wall, climb and air jumps; drop-through
//! 7. falling: gravity with hangtime, variable jump, jump cut, wall slide
//! 8. [`collide`](self::collide): move unit by unit, corner correction
//! 9. [`land`](self::land): landings (plain, roll, hard, dive impact), leaving the ground
//!
//! Your own moves hook in before and after through [`super::Ability`].

mod climb;
mod collide;
mod dash;
mod dive;
mod jump;
mod land;
mod super_dash;

use super::{Contacts, Event, Intent, JumpKind, Moves, Rect, Terrain, Tuning};

/// The default hitbox width in units ([`Tuning::size`] sets it per character).
pub const W: i32 = 6;
/// The default hitbox height in units ([`Tuning::size`] sets it per character).
pub const H: i32 = 12;

/// Seconds over which [`Player::lift`] follows the ridden ground's speed. Ground that moves
/// in whole units moves 1, 0, 1, 0 units per step at half a unit per step; smoothed, that
/// reads as the steady speed it is.
pub const LIFT_SMOOTHING: f32 = 0.1;

/// What the body is busy with, beyond plain running and jumping. New moves bring new
/// modes, so matches need a `_` arm.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Mode {
    /// Free movement.
    Normal,
    /// Rolling out of a high landing.
    Roll {
        /// Seconds since the roll started.
        t: f32,
    },
    /// Recovering from a hard landing.
    HardLand {
        /// Seconds since touching down.
        t: f32,
    },
    /// Dashing in a fixed direction, gravity off.
    Dash {
        /// Seconds since the dash started.
        t: f32,
        /// Horizontal direction (normalized with `dy`).
        dx: f32,
        /// Vertical direction, positive down.
        dy: f32,
    },
    /// Holding on to the wall on the facing side.
    Climb,
    /// Diving: a short hang, then straight down.
    Dive {
        /// Seconds since the dive started.
        t: f32,
    },
    /// Charging a super dash, standing (`wall` 0) or holding on to the wall on side `wall`.
    SuperCharge {
        /// Seconds charged.
        t: f32,
        /// Side of the wall held, or 0 on the ground.
        wall: i32,
    },
    /// Flying a super dash: straight and level until a wall or a cancel.
    SuperDash {
        /// Direction, -1 or 1.
        dir: i32,
    },
    /// Recovering from a dive's impact.
    DiveLand {
        /// Seconds since the impact.
        t: f32,
    },
    /// A move of your own ([`super::Ability`]) has the body: no built-in move starts or
    /// runs, no gravity; it still moves by its speed, collides and lands.
    Custom {
        /// Which of your moves (see [`super::CustomId`]; the same kind of id as
        /// [`Event::Custom`]).
        id: super::CustomId,
        /// Seconds since it started (counted by the movement).
        t: f32,
    },
}

/// The hero's body. What a game or an [`Ability`](super::Ability) may change is public:
/// position (a nudge; [`Player::teleport`] for a jump elsewhere), speed, facing, mode and
/// refills. The movement's own state (timers, latches, what it stands on) is read through
/// methods: written by hand it could leave the body in a state the movement never
/// produces.
#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    /// Left edge of the hitbox, units (the movement's space; see `docs/scale.md`).
    pub x: i32,
    /// Top edge of the hitbox, units.
    pub y: i32,
    /// Hitbox width in units, from [`Tuning::size`].
    pub(crate) w: i32,
    /// Hitbox height in units, from [`Tuning::size`].
    pub(crate) h: i32,
    /// Fraction of a unit moved sideways but not yet applied to [`Player::x`].
    rem_x: f32,
    /// Fraction of a unit moved vertically but not yet applied to [`Player::y`].
    rem_y: f32,
    /// Horizontal speed, units/s (pixels/s only at a scale of 1; see `docs/scale.md`).
    pub vx: f32,
    /// Vertical speed, units/s, positive down.
    pub vy: f32,
    /// 1 facing right, -1 facing left.
    pub facing: i32,
    /// Standing on something.
    pub(crate) grounded: bool,
    /// What the body is busy with.
    pub mode: Mode,
    /// Coyote time left.
    pub(crate) coyote: f32,
    /// Jump buffer time left: a press waiting to become a jump.
    pub(crate) buffer: f32,
    /// Dash buffer time left: a press waiting to become a dash.
    pub(crate) dash_buffer: f32,
    /// Variable-jump time left: holding jump keeps the upward speed.
    pub(crate) var_jump: f32,
    /// The upward speed the variable jump sustains.
    var_jump_speed: f32,
    /// The current jump can still be cut by releasing.
    can_cut: bool,
    /// Air jumps left.
    pub air_jumps: u8,
    /// Dashes left.
    pub dashes: u8,
    /// Seconds until the next dash may start.
    pub(crate) dash_cooldown: f32,
    /// Climbing stamina left.
    pub stamina: f32,
    /// Seconds the stick is overridden after a wall jump.
    pub(crate) force_move: f32,
    /// The stick's override direction during [`Player::force_move`].
    force_dir: i32,
    /// Seconds platforms still let the hero through after dropping.
    pub(crate) drop_through: f32,
    /// Side of the wall being slid down (-1, 1), or 0.
    pub(crate) wall_slide: i32,
    /// Wall-stick time left: pressing away from the wall slid down doesn't let go yet.
    pub(crate) wall_stick: f32,
    /// Holding on to a wall but sliding down it (a slipping or tired grip).
    pub(crate) slipping: bool,
    /// Seconds before a wall can be grabbed again after hopping over its top.
    pub(crate) regrab: f32,
    /// Looking up (-1) or down (1) while standing still, or 0.
    pub(crate) look: i32,
    /// Seconds the current look has been held.
    pub(crate) look_time: f32,
    /// A jump was pressed in the air and is still held: falling, it glides.
    glide_armed: bool,
    /// Gliding.
    pub(crate) gliding: bool,
    /// Down was held last step, to see a fresh tap.
    was_down: bool,
    /// Super dash was held last step, to see a fresh press.
    was_super: bool,
    /// Highest (smallest) `y` of this airtime (units), for the drop height on landing.
    pub(crate) peak_y: i32,
    /// The speed of the moving ground (or wall) ridden, units/s, smoothed over about
    /// [`LIFT_SMOOTHING`] seconds: a jump keeps it.
    pub(crate) lift: (f32, f32),
    /// Seconds [`Player::lift`] still counts after leaving what it rode.
    lift_time: f32,
    /// What its movement ran into in the last step.
    pub(crate) contacts: Contacts,
}

impl Player {
    /// Standing with its hitbox's top-left at `x, y` (units), with everything refilled.
    pub fn at((x, y): (i32, i32), t: &Tuning) -> Self {
        Self {
            x,
            y,
            w: t.size.0,
            h: t.size.1,
            rem_x: 0.0,
            rem_y: 0.0,
            vx: 0.0,
            vy: 0.0,
            facing: 1,
            grounded: true,
            mode: Mode::Normal,
            coyote: 0.0,
            buffer: 0.0,
            dash_buffer: 0.0,
            var_jump: 0.0,
            var_jump_speed: 0.0,
            can_cut: false,
            air_jumps: t.air_jumps,
            dashes: t.dashes,
            dash_cooldown: 0.0,
            stamina: t.stamina,
            force_move: 0.0,
            force_dir: 0,
            drop_through: 0.0,
            wall_slide: 0,
            wall_stick: 0.0,
            slipping: false,
            regrab: 0.0,
            look: 0,
            look_time: 0.0,
            glide_armed: false,
            gliding: false,
            was_down: false,
            was_super: false,
            peak_y: y,
            lift: (0.0, 0.0),
            lift_time: 0.0,
            contacts: Contacts::default(),
        }
    }

    /// Set the speed from outside: a spring, a bounce pad, knockback. Whatever the body was
    /// busy with ends (a dash, a dive, a climb), a jump in progress can no longer be
    /// sustained or cut, and going up leaves the ground for good (no coyote jump).
    pub fn launch(&mut self, vx: f32, vy: f32) {
        self.vx = vx;
        self.vy = vy;
        self.stop_jump();
        self.mode = Mode::Normal;
        self.force_move = 0.0;
        self.wall_slide = 0;
        self.wall_stick = 0.0;
        self.slipping = false;
        self.glide_armed = false;
        self.lift = (0.0, 0.0);
        if vy < 0.0 {
            self.grounded = false;
            self.coyote = 0.0;
        }
    }

    /// Put the hitbox's top-left at `x, y` (units) without moving through what's between:
    /// a door, a portal. The sub-pixel remainder is dropped and the airtime starts over;
    /// speed and state stay (follow with [`Player::launch`] to stop it). Teleported into
    /// the air while standing, it walks off as if from a ledge.
    pub fn teleport(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
        self.rem_x = 0.0;
        self.rem_y = 0.0;
        self.peak_y = y;
        self.wall_slide = 0;
        self.wall_stick = 0.0;
        self.lift = (0.0, 0.0);
    }

    /// Refill dashes, air jumps and climbing stamina, as touching the ground does.
    pub fn refill(&mut self, t: &Tuning) {
        self.dashes = t.dashes;
        self.air_jumps = t.air_jumps;
        self.stamina = t.stamina;
    }

    /// The hitbox.
    pub fn rect(&self) -> Rect {
        Rect {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
        }
    }

    /// Position with the sub-pixel remainder, for drawing between steps.
    pub fn exact_pos(&self) -> (f32, f32) {
        (self.x as f32 + self.rem_x, self.y as f32 + self.rem_y)
    }

    /// One step of `dt` seconds (see the module docs for the order), with the built-in
    /// moves `moves` allows.
    pub fn step(
        &mut self,
        level: &dyn Terrain,
        i: &Intent,
        moves: &Moves,
        t: &Tuning,
        dt: f32,
        ev: &mut Vec<Event>,
    ) {
        let move_x = if self.force_move > 0.0 {
            self.force_dir as f32
        } else {
            i.move_x.clamp(-1.0, 1.0)
        };
        self.tick_timers(i, t, dt, ev);
        self.tick_mode(t, dt, ev);
        if moves.dash {
            self.try_dash(i, t, ev);
        }
        if moves.dive {
            self.try_dive(i, t, ev);
        }
        self.dive_motion(t);
        // What's ruled out acts as if its button weren't held: a climb lets go.
        let held = Intent {
            grab: i.grab && moves.climb,
            super_dash: i.super_dash && moves.super_dash,
            ..*i
        };
        self.climb(level, &held, t, dt, ev);
        self.super_dash(level, &held, t, ev);
        self.run(level, move_x, t.sprint && i.sprint, moves.max_run, t, dt);
        let airborne = !self.grounded && self.coyote <= 0.0;
        let before = ev.len();
        let jumped = moves.jump && self.jump(level, i, move_x, t, ev);
        let off_wall = ev[before..].iter().any(|e| {
            matches!(
                e,
                Event::Jumped {
                    kind: JumpKind::Wall | JumpKind::Climb,
                    ..
                }
            )
        });
        if i.jump_pressed && airborne && !off_wall {
            self.glide_armed = true;
        }
        if !i.jump_held || self.mode != Mode::Normal || !moves.glide {
            self.glide_armed = false;
        }
        self.fall(level, i, move_x, t, dt, ev);

        let landing_vy = self.vy;
        self.contacts = Contacts::default();
        let moved_at = ev.len();
        let toward = if self.vx == 0.0 {
            0
        } else {
            self.vx.signum() as i32
        };
        let blocked = self.move_x(level, self.vx * dt);
        let flush = toward != 0 && self.wall(level, toward);
        if blocked || flush {
            self.contacts.wall = toward;
        }
        // Standing flush against the wall it runs toward, the sub-pixel remainder would let
        // speed build for a few steps before the next whole unit is blocked: a running
        // animation that starts and stops. On the ground the speed stays at zero instead; in
        // the air it is kept, so a jump along a wall still carries over its top.
        if flush && !blocked && self.grounded && self.mode == Mode::Normal {
            self.vx = 0.0;
        }
        if blocked {
            self.vx = 0.0;
            if let Mode::SuperDash { dir } = self.mode {
                self.super_bonk(dir, ev);
            }
            if matches!(self.mode, Mode::Roll { .. }) {
                self.mode = Mode::Normal;
                ev.push(Event::RollEnded);
            }
        }
        self.move_y(level, self.vy * dt, t.corner_correction, ev);
        self.land(level, move_x, landing_vy, jumped, t, ev);
        for e in &ev[moved_at..] {
            match e {
                Event::Bonked => self.contacts.ceiling = true,
                Event::Landed { .. } => self.contacts.floor = true,
                _ => {}
            }
        }
        self.update_look(i, move_x, dt);
        self.run_roll(i, t, ev);
    }

    /// Open or close the glide, reporting the change.
    fn set_gliding(&mut self, on: bool, ev: &mut Vec<Event>) {
        if on != self.gliding {
            self.gliding = on;
            ev.push(if on {
                Event::GlideStarted
            } else {
                Event::GlideEnded
            });
        }
    }

    /// A fresh tap of down while running at speed on the ground rolls.
    fn run_roll(&mut self, i: &Intent, t: &Tuning, ev: &mut Vec<Event>) {
        let down = i.down();
        let tap = down && !self.was_down;
        self.was_down = down;
        if !t.run_roll
            || !tap
            || !self.grounded
            || self.mode != Mode::Normal
            || self.vx.abs() < t.run_roll_speed
        {
            return;
        }
        self.facing = self.vx.signum() as i32;
        self.vx = self.facing as f32 * self.vx.abs().max(t.roll_speed);
        self.mode = Mode::Roll { t: 0.0 };
        ev.push(Event::RollStarted);
    }

    /// Holding up or down while standing still looks that way.
    fn update_look(&mut self, i: &Intent, move_x: f32, dt: f32) {
        let still =
            self.grounded && self.mode == Mode::Normal && move_x == 0.0 && self.vx.abs() < 1.0;
        let dir = match () {
            _ if !still => 0,
            _ if i.move_y < -0.5 => -1,
            _ if i.move_y > 0.5 => 1,
            _ => 0,
        };
        if dir != self.look {
            self.look = dir;
            self.look_time = 0.0;
        } else if dir != 0 {
            self.look_time += dt;
        }
    }

    /// Buffers, coyote time, refills on the ground, cooldowns.
    fn tick_timers(&mut self, i: &Intent, t: &Tuning, dt: f32, ev: &mut Vec<Event>) {
        let tick = |v: f32| (v - dt).max(0.0);
        self.buffer = if i.jump_pressed {
            t.jump_buffer
        } else {
            tick(self.buffer)
        };
        self.dash_buffer = if i.dash_pressed {
            t.dash_buffer
        } else {
            tick(self.dash_buffer)
        };
        self.dash_cooldown = tick(self.dash_cooldown);
        self.force_move = tick(self.force_move);
        self.drop_through = tick(self.drop_through);
        self.regrab = tick(self.regrab);
        if self.grounded {
            self.coyote = t.coyote_time;
            self.air_jumps = t.air_jumps;
            self.stamina = t.stamina;
            let dashing =
                matches!(self.mode, Mode::Dash { t: dt_, .. } if dt_ < t.dash_recharge_after);
            if !dashing && self.dashes < t.dashes {
                self.dashes = t.dashes;
                ev.push(Event::Recharged { crystal: false });
            }
        } else {
            self.coyote = tick(self.coyote);
            self.wall_refill(t, ev);
        }
    }

    /// On a wall (sliding down it or holding on, as of the last step), refill air jumps
    /// and, if the tuning says so, dashes (a design choice: walls as a place to recover).
    fn wall_refill(&mut self, t: &Tuning, ev: &mut Vec<Event>) {
        if self.wall_slide == 0 && self.mode != Mode::Climb {
            return;
        }
        if t.wall_refills_air_jumps {
            self.air_jumps = t.air_jumps;
        }
        if t.wall_refills_dashes && self.dashes < t.dashes {
            self.dashes = t.dashes;
            ev.push(Event::Recharged { crystal: false });
        }
    }

    /// The clocks of timed modes: rolls, recoveries and dashes run out.
    fn tick_mode(&mut self, t: &Tuning, dt: f32, ev: &mut Vec<Event>) {
        if let Mode::Roll { t: x }
        | Mode::HardLand { t: x }
        | Mode::Dash { t: x, .. }
        | Mode::Dive { t: x }
        | Mode::DiveLand { t: x }
        | Mode::SuperCharge { t: x, .. }
        | Mode::Custom { t: x, .. } = &mut self.mode
        {
            let before = *x;
            *x += dt;
            if matches!(self.mode, Mode::SuperCharge { .. })
                && before < t.super_charge
                && self.mode_time() >= t.super_charge
            {
                ev.push(Event::SuperCharged);
            }
        }
        let time = self.mode_time();
        match self.mode {
            Mode::Roll { .. } if time >= t.roll_time => {
                self.mode = Mode::Normal;
                ev.push(Event::RollEnded);
            }
            Mode::HardLand { .. } if time >= t.hard_land_time => {
                self.mode = Mode::Normal;
                ev.push(Event::Recovered);
            }
            Mode::DiveLand { .. } if time >= t.dive_land_time => {
                self.mode = Mode::Normal;
                ev.push(Event::Recovered);
            }
            Mode::Dash { .. } if time >= t.dash_time => {
                self.end_dash(t);
                ev.push(Event::DashEnded);
            }
            _ => {}
        }
    }

    /// Seconds in the current timed mode (0 for untimed ones).
    pub fn mode_time(&self) -> f32 {
        match self.mode {
            Mode::Normal | Mode::Climb | Mode::SuperDash { .. } => 0.0,
            Mode::SuperCharge { t, .. }
            | Mode::Roll { t }
            | Mode::HardLand { t }
            | Mode::Dash { t, .. }
            | Mode::Dive { t }
            | Mode::DiveLand { t }
            | Mode::Custom { t, .. } => t,
        }
    }

    /// Horizontal speed: rolls keep their speed, recoveries brake, dashes, dives and climbs
    /// set their own, otherwise accelerate toward the stick, up to `max_run` at most.
    fn run(
        &mut self,
        level: &dyn Terrain,
        move_x: f32,
        sprint: bool,
        max_run: f32,
        t: &Tuning,
        dt: f32,
    ) {
        let side = self.wall_slide;
        if self.mode == Mode::Normal
            && side != 0
            && move_x * (side as f32) < 0.0
            && self.wall_stick > 0.0
            && self.wall(level, side)
        {
            // Stuck: pressing away from the wall doesn't pull off it yet.
            self.wall_stick = (self.wall_stick - dt).max(0.0);
            self.vx = 0.0;
            return;
        }
        match self.mode {
            Mode::Roll { .. } => {
                self.vx = self.facing as f32 * self.vx.abs().max(t.roll_speed);
            }
            Mode::HardLand { .. } | Mode::DiveLand { .. } => {
                self.vx = approach(self.vx, 0.0, t.run_decel * dt);
            }
            Mode::Dash { .. }
            | Mode::Dive { .. }
            | Mode::Climb
            | Mode::SuperCharge { .. }
            | Mode::SuperDash { .. }
            | Mode::Custom { .. } => {}
            Mode::Normal => {
                let top = if sprint { t.sprint_speed } else { t.run_speed }.min(max_run);
                if move_x != 0.0 {
                    self.facing = move_x.signum() as i32;
                }
                let target = move_x * top;
                let accel = if move_x == 0.0 {
                    t.run_decel
                } else if self.vx * move_x < 0.0 {
                    t.turn_accel
                } else if self.vx.abs() > top {
                    t.overspeed_decel
                } else {
                    t.run_accel
                };
                // On the ground, the surface scales the grip (ice, sticky ground).
                let air = if self.grounded {
                    level.surface(self.feet()).friction
                } else {
                    t.air_control
                };
                self.vx = approach(self.vx, target, accel * air * dt);
            }
        }
    }

    /// Gravity (lighter at the apex while jump is held, heavier falling), the variable
    /// jump's sustain, the jump cut on release, and sliding down walls. Only free and
    /// rolling bodies fall; the other modes set their own vertical speed.
    fn fall(
        &mut self,
        level: &dyn Terrain,
        i: &Intent,
        move_x: f32,
        t: &Tuning,
        dt: f32,
        ev: &mut Vec<Event>,
    ) {
        let stuck = self.wall_slide != 0 && self.vx == 0.0 && self.wall_stick > 0.0;
        let stuck_side = self.wall_slide;
        self.wall_slide = 0;
        if self.grounded || !matches!(self.mode, Mode::Normal | Mode::Roll { .. }) {
            self.wall_stick = 0.0;
            self.set_gliding(false, ev);
            return;
        }
        let before = self.vy;
        let mult = if i.jump_held && self.vy.abs() < t.apex_threshold {
            t.apex_gravity
        } else if self.vy > 0.0 {
            t.fall_gravity
        } else {
            1.0
        };
        let mut max = if i.down() { t.fast_fall } else { t.max_fall };
        let side = move_x.signum() as i32;
        if move_x != 0.0 && self.vy > 0.0 && self.wall(level, side) {
            max = max.min(t.wall_slide_speed);
            self.wall_slide = side;
            self.wall_stick = t.wall_stick_time;
        } else if stuck && self.wall(level, stuck_side) {
            max = max.min(t.wall_slide_speed);
            self.wall_slide = stuck_side;
        } else {
            self.wall_stick = 0.0;
        }
        let glide = t.glide
            && self.glide_armed
            && i.jump_held
            && before > 0.0
            && self.wall_slide == 0
            && self.mode == Mode::Normal;
        self.set_gliding(glide, ev);
        if glide {
            max = max.min(t.glide_fall_speed);
        }
        // Over the cap (a fast fall released, a wall caught), brake toward it; otherwise
        // accelerate up to it.
        self.vy = if before > max {
            approach(before, max, t.gravity * dt)
        } else {
            (before + t.gravity * mult * dt).min(max)
        };
        if self.var_jump > 0.0 {
            if i.jump_held {
                self.vy = self.vy.min(-self.var_jump_speed);
            } else {
                self.var_jump = 0.0;
            }
            self.var_jump = (self.var_jump - dt).max(0.0);
        }
        if self.can_cut && !i.jump_held && self.vy < 0.0 {
            self.vy *= t.jump_cut;
            self.can_cut = false;
            ev.push(Event::JumpCut);
        }
        if self.vy >= 0.0 {
            self.can_cut = false;
            if before < 0.0 {
                ev.push(Event::Apex);
            }
        }
    }

    /// Whether a solid wall touches the hitbox on `side` (-1 left, 1 right).
    pub fn wall(&self, level: &dyn Terrain, side: i32) -> bool {
        side != 0 && level.solid(self.rect().shifted(side, 0))
    }

    /// Whether a wall the character may hold on to touches the hitbox on `side`: solid,
    /// and its [`super::Surface`] climbable.
    pub fn climbable(&self, level: &dyn Terrain, side: i32) -> bool {
        self.wall(level, side) && level.surface(self.beside(side)).climbable
    }

    /// The row just under the hitbox.
    pub fn feet(&self) -> Rect {
        let r = self.rect();
        Rect {
            y: r.bottom(),
            h: 1,
            ..r
        }
    }

    /// The column just beside the hitbox on `side` (-1 left, 1 right).
    pub fn beside(&self, side: i32) -> Rect {
        let r = self.rect();
        let x = if side > 0 { r.right() } else { r.x - 1 };
        Rect { x, w: 1, ..r }
    }

    /// What the body rides on: the row under its feet while standing, the column beside it
    /// while holding on to a wall, or nothing.
    pub fn riding(&self) -> Option<Rect> {
        if self.grounded {
            Some(self.feet())
        } else if self.mode == Mode::Climb {
            Some(self.beside(self.facing))
        } else {
            None
        }
    }

    /// Carried `dx, dy` units in a step of `dt` by the moving ground (or moving wall) it
    /// rides, or `None` when it rides nothing. Moved through the same unit-by-unit
    /// collision, testing only the edge it moves toward, so the ground it rides (now
    /// overlapping the feet when rising) doesn't hold it back. Its speed becomes
    /// [`Player::lift`], which a jump keeps for [`Tuning::coyote_time`] after leaving.
    pub fn ride(&mut self, level: &dyn Terrain, carry: Option<(f32, f32)>, t: &Tuning, dt: f32) {
        let Some((dx, dy)) = carry else {
            self.lift_time = (self.lift_time - dt).max(0.0);
            if self.lift_time <= 0.0 {
                self.lift = (0.0, 0.0);
            }
            return;
        };
        let a = 1.0 - (-dt / LIFT_SMOOTHING).exp();
        self.lift.0 += (dx / dt - self.lift.0) * a;
        self.lift.1 += (dy / dt - self.lift.1) * a;
        self.lift_time = t.coyote_time;
        self.carry(level, dx, dy);
    }

    /// Forget any jump in progress: no sustain, no cut.
    fn stop_jump(&mut self) {
        self.var_jump = 0.0;
        self.can_cut = false;
    }
}

/// Move `v` toward `target` by at most `step`.
pub(crate) fn approach(v: f32, target: f32, step: f32) -> f32 {
    if v < target {
        (v + step).min(target)
    } else {
        (v - step).max(target)
    }
}

/// The movement's own state, to read (drivers, debug overlays).
impl Player {
    /// Hitbox width in units, from [`Tuning::size`].
    pub fn w(&self) -> i32 {
        self.w
    }

    /// Hitbox height in units, from [`Tuning::size`].
    pub fn h(&self) -> i32 {
        self.h
    }

    /// Standing on something.
    pub fn grounded(&self) -> bool {
        self.grounded
    }

    /// Coyote time left.
    pub fn coyote(&self) -> f32 {
        self.coyote
    }

    /// Jump buffer time left: a press waiting to become a jump.
    pub fn buffer(&self) -> f32 {
        self.buffer
    }

    /// Dash buffer time left: a press waiting to become a dash.
    pub fn dash_buffer(&self) -> f32 {
        self.dash_buffer
    }

    /// Variable-jump time left: holding jump keeps the upward speed.
    pub fn var_jump(&self) -> f32 {
        self.var_jump
    }

    /// Seconds until the next dash may start.
    pub fn dash_cooldown(&self) -> f32 {
        self.dash_cooldown
    }

    /// Seconds the stick is overridden after a wall jump.
    pub fn force_move(&self) -> f32 {
        self.force_move
    }

    /// Seconds platforms still let the hero through after dropping.
    pub fn drop_through(&self) -> f32 {
        self.drop_through
    }

    /// Side of the wall being slid down (-1, 1), or 0.
    pub fn wall_slide(&self) -> i32 {
        self.wall_slide
    }

    /// Wall-stick time left: pressing away from the wall slid down doesn't let go yet.
    pub fn wall_stick(&self) -> f32 {
        self.wall_stick
    }

    /// Holding on to a wall but sliding down it (a slipping or tired grip).
    pub fn slipping(&self) -> bool {
        self.slipping
    }

    /// Seconds before a wall can be grabbed again after hopping over its top.
    pub fn regrab(&self) -> f32 {
        self.regrab
    }

    /// Looking up (-1) or down (1) while standing still, or 0.
    pub fn look(&self) -> i32 {
        self.look
    }

    /// Seconds the current look has been held.
    pub fn look_time(&self) -> f32 {
        self.look_time
    }

    /// Gliding.
    pub fn gliding(&self) -> bool {
        self.gliding
    }

    /// Highest (smallest) `y` of this airtime (units), for the drop height on landing.
    pub fn peak_y(&self) -> i32 {
        self.peak_y
    }

    /// The speed of the moving ground (or wall) ridden, units/s, smoothed over about
    /// [`LIFT_SMOOTHING`] seconds: a jump keeps it.
    pub fn lift(&self) -> (f32, f32) {
        self.lift
    }

    /// What its movement ran into in the last step.
    pub fn contacts(&self) -> Contacts {
        self.contacts
    }
}
