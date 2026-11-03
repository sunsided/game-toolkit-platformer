//! [`HeroDriver`]: the simulation's facts and events as the hero graph's parameters.

use crate::anim::{Driver, ParamId, Params};
use crate::sim::{Event, JumpKind, Mode, Shove, World};

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
    /// `pushing`
    pushing: ParamId,
    /// `pulling`
    pulling: ParamId,
    /// `holding`
    holding: ParamId,
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
    /// driver and [`super::spec`] belong together.
    pub fn new(params: &Params) -> Self {
        let id = |n: &str| {
            params
                .id(n)
                .unwrap_or_else(|| panic!("the hero graph declares {n:?}"))
        };
        Self {
            grounded: id("grounded"),
            skid: id("skid"),
            rolling: id("rolling"),
            dashing: id("dashing"),
            climbing: id("climbing"),
            slipping: id("slipping"),
            wall_slide: id("wall_slide"),
            hard_landing: id("hard_landing"),
            wings: id("wings"),
            pushing: id("pushing"),
            pulling: id("pulling"),
            holding: id("holding"),
            gliding: id("gliding"),
            super_bonk: id("super_bonk"),
            super_launch: id("super_launch"),
            super_flying: id("super_flying"),
            charged: id("charged"),
            charging: id("charging"),
            speed: id("speed"),
            vy: id("vy"),
            dash_y: id("dash_y"),
            look: id("look"),
            jump: id("jump"),
            double: id("double"),
            wall_jump: id("wall_jump"),
            hop: id("hop"),
            drop: id("drop"),
            land: id("land"),
            roll: id("roll"),
            hard: id("hard"),
            dash: id("dash"),
            grab: id("grab"),
            dive: id("dive"),
            impact: id("impact"),
        }
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
        p.set_bool(self.pushing, matches!(pl.shove, Shove::Push { .. }));
        p.set_bool(self.pulling, matches!(pl.shove, Shove::Pull { .. }));
        p.set_bool(self.holding, matches!(pl.shove, Shove::Hold { .. }));
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
