//! [`HeroDriver`]: the simulation's facts and events as the hero graph's parameters.

use crate::anim::{Driver, ParamId, ParamKind, Params, SpecError};
use crate::sim::{Event, JumpKind, Mode, World};

/// Parameter ids of the hero graph, looked up once.
#[derive(Clone, Debug)]
pub struct HeroDriver {
    /// `grounded`
    grounded: ParamId,
    /// `skid`
    skid: ParamId,
    /// `rolling`
    rolling: ParamId,
    /// `dashing`
    dashing: ParamId,
    /// `climbing`
    climbing: ParamId,
    /// `slipping`
    slipping: ParamId,
    /// `wall_slide`
    wall_slide: ParamId,
    /// `hard_landing`
    hard_landing: ParamId,
    /// `wings`
    wings: ParamId,
    /// `gliding`
    gliding: ParamId,
    /// `super_bonk`
    super_bonk: ParamId,
    /// `super_launch`
    super_launch: ParamId,
    /// `super_flying`
    super_flying: ParamId,
    /// `charged`
    charged: ParamId,
    /// `charging`
    charging: ParamId,
    /// `speed`
    speed: ParamId,
    /// `vy`
    vy: ParamId,
    /// `dash_y`
    dash_y: ParamId,
    /// `look`
    look: ParamId,
    /// `jump`
    jump: ParamId,
    /// `double`
    double: ParamId,
    /// `wall_jump`
    wall_jump: ParamId,
    /// `hop`
    hop: ParamId,
    /// `drop`
    drop: ParamId,
    /// `land`
    land: ParamId,
    /// `roll`
    roll: ParamId,
    /// `hard`
    hard: ParamId,
    /// `dash`
    dash: ParamId,
    /// `grab`
    grab: ParamId,
    /// `dive`
    dive: ParamId,
    /// `impact`
    impact: ParamId,
}

/// Skidding takes at least this much speed against the facing direction.
const SKID_SPEED: f32 = 30.0;

impl HeroDriver {
    /// Look up the hero graph's parameters in `params`. Panics on a missing one: the
    /// driver and [`super::spec`] belong together. For a graph of your own, use
    /// [`HeroDriver::try_new`].
    pub fn new(params: &Params) -> Self {
        Self::try_new(params).unwrap_or_else(|e| panic!("{e}"))
    }

    /// Look up the parameters this driver writes in `params`, or say which one a graph
    /// doesn't declare. A custom graph has to declare all of them (it may ignore any);
    /// see [`HeroDriver::declaring`] for one that doesn't.
    pub fn try_new(params: &Params) -> Result<Self, SpecError> {
        Self::lookup(&mut params.clone(), false)
    }

    /// Look up the parameters this driver writes in `params`, declaring the ones a graph
    /// of your own left out. The graph never reads those, so it can leave out whatever it
    /// doesn't animate.
    pub fn declaring(params: &mut Params) -> Self {
        Self::lookup(params, true).expect("missing parameters are declared")
    }

    /// Find (or, with `declare`, add) every parameter this driver writes.
    fn lookup(params: &mut Params, declare: bool) -> Result<Self, SpecError> {
        const B: ParamKind = ParamKind::Bool;
        const F: ParamKind = ParamKind::Float;
        const T: ParamKind = ParamKind::Trigger { ttl: 0.0 };
        let mut id = |n: &'static str, kind: ParamKind| match params.id(n) {
            Some(id) => Ok(id),
            None if declare => Ok(params.declare(n, kind)),
            None => Err(SpecError::UnknownParam(n.into())),
        };
        Ok(Self {
            grounded: id("grounded", B)?,
            skid: id("skid", B)?,
            rolling: id("rolling", B)?,
            dashing: id("dashing", B)?,
            climbing: id("climbing", B)?,
            slipping: id("slipping", B)?,
            wall_slide: id("wall_slide", B)?,
            hard_landing: id("hard_landing", B)?,
            wings: id("wings", B)?,
            gliding: id("gliding", B)?,
            super_bonk: id("super_bonk", T)?,
            super_launch: id("super_launch", T)?,
            super_flying: id("super_flying", B)?,
            charged: id("charged", B)?,
            charging: id("charging", B)?,
            speed: id("speed", F)?,
            vy: id("vy", F)?,
            dash_y: id("dash_y", F)?,
            look: id("look", F)?,
            jump: id("jump", T)?,
            double: id("double", T)?,
            wall_jump: id("wall_jump", T)?,
            hop: id("hop", T)?,
            drop: id("drop", T)?,
            land: id("land", T)?,
            roll: id("roll", T)?,
            hard: id("hard", T)?,
            dash: id("dash", T)?,
            grab: id("grab", T)?,
            dive: id("dive", T)?,
            impact: id("impact", T)?,
        })
    }
}

impl Driver<World> for HeroDriver {
    fn drive(&mut self, w: &World, p: &mut Params) {
        let pl = &w.player;
        p.set_bool(self.grounded, pl.grounded);
        p.set_bool(
            self.skid,
            pl.grounded
                && pl.mode == Mode::Normal
                && pl.vx.abs() > SKID_SPEED
                && pl.vx.signum() as i32 != pl.facing,
        );
        p.set_bool(self.rolling, matches!(pl.mode, Mode::Roll { .. }));
        p.set_bool(self.dashing, matches!(pl.mode, Mode::Dash { .. }));
        p.set_bool(self.climbing, pl.mode == Mode::Climb);
        p.set_bool(self.slipping, pl.slipping);
        p.set_bool(self.wall_slide, pl.wall_slide != 0);
        p.set_bool(self.hard_landing, matches!(pl.mode, Mode::HardLand { .. }));
        p.set_bool(self.slipping, pl.slipping);
        p.set_bool(self.wings, w.tuning.wings);
        p.set_bool(self.gliding, pl.gliding);
        let charge = match pl.mode {
            Mode::SuperCharge { t, .. } => Some(t),
            _ => None,
        };
        p.set_bool(self.charging, charge.is_some());
        p.set_bool(
            self.charged,
            charge.is_some_and(|t| t >= w.tuning.super_charge),
        );
        p.set_bool(self.super_flying, matches!(pl.mode, Mode::SuperDash { .. }));
        // Signed seconds held: negative looking up, positive looking down.
        p.set_float(self.look, pl.look as f32 * pl.look_time);
        p.set_float(self.speed, pl.vx.abs());
        p.set_float(self.vy, pl.vy);
        if let Mode::Dash { dy, .. } = pl.mode {
            p.set_float(self.dash_y, dy);
        }
        for e in &w.events {
            let t = match e {
                Event::Jumped {
                    kind: JumpKind::Double,
                    ..
                } => self.double,
                Event::Jumped {
                    kind: JumpKind::Wall,
                    ..
                } => self.wall_jump,
                Event::Jumped { .. } => self.jump,
                Event::Landed { .. } => self.land,
                Event::RollStarted => self.roll,
                Event::HardLanded => self.hard,
                Event::Dashed { .. } => self.dash,
                Event::Grabbed => self.grab,
                Event::ClimbHop => self.hop,
                Event::DroppedThrough => self.drop,
                Event::DiveStarted => self.dive,
                Event::DiveImpact { .. } => self.impact,
                Event::SuperDashStarted { .. } => self.super_launch,
                Event::SuperDashEnded { bonk: true } => self.super_bonk,
                _ => continue,
            };
            p.fire(t);
        }
    }
}
