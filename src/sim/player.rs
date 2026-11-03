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
//! 8. [`collide`](self::collide): move pixel by pixel, corner correction
//! 9. [`land`](self::land): landings (plain, roll, hard, dive impact), leaving the ground

mod climb;
mod collide;
mod dash;
mod dive;
mod jump;
mod land;

use super::{Event, Intent, JumpKind, Level, Rect, TILE, Tuning};

/// Hitbox width in pixels.
pub const W: i32 = 6;
/// Hitbox height in pixels.
pub const H: i32 = 12;

/// The hero's hold on a block, decided by the world each step before the hero moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shove {
    /// No block involved.
    None,
    /// Grabbing a block on `side` without moving.
    Hold {
        /// Side of the block, -1 or 1.
        side: i32,
    },
    /// Walking into a block on `side`, pushing it.
    Push {
        /// Side of the block.
        side: i32,
    },
    /// Grabbing a block on `side` and walking away, pulling it.
    Pull {
        /// Side of the block.
        side: i32,
    },
}

/// What the body is busy with, beyond plain running and jumping.
#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// Recovering from a dive's impact.
    DiveLand {
        /// Seconds since the impact.
        t: f32,
    },
}

/// The hero's body.
#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    /// Left edge of the hitbox, grid pixels.
    pub x: i32,
    /// Top edge of the hitbox, grid pixels.
    pub y: i32,
    /// Sub-pixel horizontal position not yet moved.
    rem_x: f32,
    /// Sub-pixel vertical position not yet moved.
    rem_y: f32,
    /// Horizontal speed, px/s.
    pub vx: f32,
    /// Vertical speed, px/s, positive down.
    pub vy: f32,
    /// 1 facing right, -1 facing left.
    pub facing: i32,
    /// Standing on something.
    pub grounded: bool,
    /// What the body is busy with.
    pub mode: Mode,
    /// Coyote time left.
    pub coyote: f32,
    /// Jump buffer time left: a press waiting to become a jump.
    pub buffer: f32,
    /// Dash buffer time left: a press waiting to become a dash.
    pub dash_buffer: f32,
    /// Variable-jump time left: holding jump keeps the upward speed.
    pub var_jump: f32,
    /// The upward speed the variable jump sustains.
    var_jump_speed: f32,
    /// The current jump can still be cut by releasing.
    can_cut: bool,
    /// Air jumps left.
    pub air_jumps: u8,
    /// Dashes left.
    pub dashes: u8,
    /// Seconds until the next dash may start.
    pub dash_cooldown: f32,
    /// Climbing stamina left.
    pub stamina: f32,
    /// Seconds the stick is overridden after a wall jump.
    pub force_move: f32,
    /// The stick's override direction during [`Player::force_move`].
    force_dir: i32,
    /// Seconds platforms still let the hero through after dropping.
    pub drop_through: f32,
    /// Side of the wall being slid down (-1, 1), or 0.
    pub wall_slide: i32,
    /// Wall-stick time left: pressing away from the wall slid down doesn't let go yet.
    pub wall_stick: f32,
    /// Holding on to a wall but sliding down it (a slipping or tired grip).
    pub slipping: bool,
    /// Seconds before a wall can be grabbed again after hopping over its top.
    pub regrab: f32,
    /// Looking up (-1) or down (1) while standing still, or 0.
    pub look: i32,
    /// Seconds the current look has been held.
    pub look_time: f32,
    /// The hold on a block this step (set by the world).
    pub shove: Shove,
    /// How far the block has given, 0 (stuck) to 1 (sliding freely); scales push and
    /// pull speed (set by the world).
    pub shove_give: f32,
    /// A jump was pressed in the air and is still held: falling, it glides.
    glide_armed: bool,
    /// Gliding.
    pub gliding: bool,
    /// Down was held last step, to see a fresh tap.
    was_down: bool,
    /// Highest (smallest) `y` of this airtime, for the drop height on landing.
    pub peak_y: i32,
}

impl Player {
    /// Standing on the level's spawn tile, with everything refilled.
    pub fn spawn(level: &Level, t: &Tuning) -> Self {
        let (sx, sy) = level.spawn;
        let y = (sy + 1) * TILE - H;
        Self {
            x: sx * TILE + (TILE - W) / 2,
            y,
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
            shove: Shove::None,
            shove_give: 0.0,
            glide_armed: false,
            gliding: false,
            was_down: false,
            peak_y: y,
        }
    }

    /// The hitbox.
    pub fn rect(&self) -> Rect {
        Rect {
            x: self.x,
            y: self.y,
            w: W,
            h: H,
        }
    }

    /// Position with the sub-pixel remainder, for drawing between steps.
    pub fn exact_pos(&self) -> (f32, f32) {
        (self.x as f32 + self.rem_x, self.y as f32 + self.rem_y)
    }

    /// One step of `dt` seconds (see the module docs for the order).
    pub fn step(&mut self, level: &Level, i: &Intent, t: &Tuning, dt: f32, ev: &mut Vec<Event>) {
        let move_x = if self.force_move > 0.0 {
            self.force_dir as f32
        } else {
            i.move_x.clamp(-1.0, 1.0)
        };
        self.tick_timers(i, t, dt, ev);
        self.tick_mode(t, dt, ev);
        self.try_dash(i, t, ev);
        self.try_dive(i, t, ev);
        self.dive_motion(t);
        self.climb(level, i, t, dt, ev);
        self.run(level, move_x, t.sprint && i.sprint, t, dt);
        let airborne = !self.grounded && self.coyote <= 0.0;
        let before = ev.len();
        let jumped = self.jump(level, i, move_x, t, ev);
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
        if !i.jump_held || self.mode != Mode::Normal {
            self.glide_armed = false;
        }
        self.fall(level, i, move_x, t, dt, ev);

        let landing_vy = self.vy;
        if self.move_x(level, self.vx * dt) {
            self.vx = 0.0;
            if matches!(self.mode, Mode::Roll { .. }) {
                self.mode = Mode::Normal;
                ev.push(Event::RollEnded);
            }
        }
        self.move_y(level, self.vy * dt, t.corner_correction, ev);
        self.land(level, move_x, landing_vy, jumped, t, ev);
        self.look(i, move_x, dt);
        self.run_roll(i, t, ev);

        if self.y > level.floor_y() + 4 * TILE {
            *self = Self::spawn(level, t);
            ev.push(Event::Respawned);
        }
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
    fn look(&mut self, i: &Intent, move_x: f32, dt: f32) {
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
    /// and, if the tuning says so, dashes: Hollow Knight gives both back on a wall cling.
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
        | Mode::DiveLand { t: x } = &mut self.mode
        {
            *x += dt;
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
            Mode::Normal | Mode::Climb => 0.0,
            Mode::Roll { t }
            | Mode::HardLand { t }
            | Mode::Dash { t, .. }
            | Mode::Dive { t }
            | Mode::DiveLand { t } => t,
        }
    }

    /// Horizontal speed: rolls keep their speed, recoveries brake, dashes, dives and climbs
    /// set their own, otherwise accelerate toward the stick.
    fn run(&mut self, level: &Level, move_x: f32, sprint: bool, t: &Tuning, dt: f32) {
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
            Mode::Dash { .. } | Mode::Dive { .. } | Mode::Climb => {}
            Mode::Normal => {
                let top = match self.shove {
                    Shove::None if sprint => t.sprint_speed,
                    Shove::None => t.run_speed,
                    Shove::Push { .. } => t.push_speed * self.shove_give,
                    Shove::Pull { .. } => t.pull_speed * self.shove_give,
                    Shove::Hold { .. } => 0.0,
                };
                match self.shove {
                    // Facing stays on the block while holding it.
                    Shove::Pull { side } | Shove::Hold { side } => self.facing = side,
                    _ if move_x != 0.0 => self.facing = move_x.signum() as i32,
                    _ => {}
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
                let air = if self.grounded { 1.0 } else { t.air_control };
                self.vx = approach(self.vx, target, accel * air * dt);
            }
        }
    }

    /// Gravity (lighter at the apex while jump is held, heavier falling), the variable
    /// jump's sustain, the jump cut on release, and sliding down walls. Only free and
    /// rolling bodies fall; the other modes set their own vertical speed.
    fn fall(
        &mut self,
        level: &Level,
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
    pub fn wall(&self, level: &Level, side: i32) -> bool {
        side != 0 && level.overlaps(self.rect().shifted(side, 0))
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
