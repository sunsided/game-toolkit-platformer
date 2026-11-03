//! The hero's animation graph.
//!
//! The pattern: entering a state that answers an event (jump, dash, land, ...) is a
//! trigger, fired by the driver from the simulation's events; staying in or leaving a
//! continuous state (running, climbing, dashing) is a bool or float the driver sets every
//! step. Transitions out of an event are lock-breaking where the event should cut the
//! current animation short.
//!
//! ```text
//!   idle <-> run <-> skid        ground, by speed and skid
//!     \        |
//!      `-> takeoff -> rise -> apex -> fall -> land -> idle
//!          double (flip), wall_jump, hop, drop: one-shots into rise / fall
//!   any -> roll | hard_land | dash_{side,up,down} | climb <-> grip_slip | wall_slide
//!   any -> dive_windup -> dive -> dive_land
//! ```

use std::sync::Arc;

use super::sprite as s;
use crate::anim::{Clip, Cond, Graph, GraphSpec, Name, Playback, StateSpec, TransitionSpec};

/// Below this horizontal speed the hero counts as standing.
const STILL: f32 = 10.0;
/// Within this vertical speed of zero the hero is at the apex.
const APEX: f32 = 40.0;

/// A bool parameter is on.
fn is(p: &'static str) -> Cond<Name> {
    Cond::Is(p.into())
}

/// A bool parameter is off.
fn not(p: &'static str) -> Cond<Name> {
    Cond::not(is(p))
}

/// A trigger is pending.
fn trig(p: &'static str) -> Cond<Name> {
    Cond::Trigger(p.into())
}

/// A float parameter is above `v`.
fn above(p: &'static str, v: f32) -> Cond<Name> {
    Cond::Above(p.into(), v)
}

/// A float parameter is below `v`.
fn below(p: &'static str, v: f32) -> Cond<Name> {
    Cond::Below(p.into(), v)
}

/// A looping clip of `sprites`, `secs` each.
fn looping(sprites: &[u16], secs: f32) -> Clip {
    Clip::uniform(Playback::Loop, sprites, secs)
}

/// A once-clip of `(sprite, secs)` frames.
fn once(frames: &[(u16, f32)]) -> Clip {
    Clip::new(Playback::Once, frames)
}

/// The air states, which land.
const AIR: [&str; 8] = [
    "takeoff",
    "rise",
    "apex",
    "fall",
    "double",
    "wall_jump",
    "hop",
    "drop",
];
/// The plain ground states, which can walk off ledges.
const GROUND: [&str; 4] = ["idle", "run", "skid", "land"];
/// The dash states.
const DASHES: [&str; 3] = ["dash_side", "dash_up", "dash_down"];

/// The hero's graph by names: parameters, states, transitions.
pub fn spec() -> GraphSpec {
    let pairs =
        |v: &[u16], secs: f32| -> Vec<(u16, f32)> { v.iter().map(|&x| (x, secs)).collect() };
    GraphSpec::new()
        // Continuous facts, set every step.
        .bool("grounded")
        .bool("skid")
        .bool("rolling")
        .bool("dashing")
        .bool("climbing")
        .bool("slipping")
        .bool("wall_slide")
        .bool("hard_landing")
        .float("speed")
        .float("vy")
        .float("dash_y")
        // Events. Zero TTL where a same-step event of higher priority must win outright.
        .trigger("jump", 0.05)
        .trigger("double", 0.05)
        .trigger("wall_jump", 0.05)
        .trigger("hop", 0.05)
        .trigger("drop", 0.05)
        .trigger("land", 0.0)
        .trigger("roll", 0.0)
        .trigger("hard", 0.0)
        .trigger("dash", 0.05)
        .trigger("grab", 0.05)
        .trigger("dive", 0.05)
        .trigger("impact", 0.0)
        // Ground.
        .state(StateSpec::new(
            "idle",
            looping(&[s::IDLE_A, s::IDLE_B], 0.5),
        ))
        .state(
            StateSpec::new(
                "run",
                looping(&[s::RUN_A, s::RUN_B, s::RUN_C, s::RUN_D], 0.08)
                    .marker(0, "step")
                    .marker(2, "step"),
            )
            .rate_by("speed", 90.0, 0.4, 1.6),
        )
        .state(StateSpec::new("skid", looping(&[s::SKID], 0.1)))
        .state(StateSpec::new("land", once(&[(s::LAND_A, 0.05), (s::LAND_B, 0.05)])).then("idle"))
        .state(
            StateSpec::new("hard_land", once(&[(s::HARD_A, 0.1), (s::HARD_B, 0.2)]))
                .lock(0.2)
                .then("idle"),
        )
        .state(StateSpec::new("roll", once(&pairs(&s::ROLL, 0.08))))
        // Air.
        .state(StateSpec::new("takeoff", once(&[(s::TAKEOFF, 0.06)])).then("rise"))
        .state(StateSpec::new("rise", looping(&[s::RISE], 0.1)))
        .state(StateSpec::new("apex", looping(&[s::APEX], 0.1)))
        .state(StateSpec::new(
            "fall",
            looping(&[s::FALL_A, s::FALL_B], 0.1),
        ))
        .state(
            StateSpec::new("double", once(&pairs(&s::FLIP, 0.05)))
                .lock(0.1)
                .then("rise"),
        )
        .state(StateSpec::new("wall_jump", once(&[(s::WALL_JUMP, 0.1)])).then("rise"))
        .state(StateSpec::new("hop", once(&[(s::HOP, 0.12)])).then("rise"))
        .state(StateSpec::new("drop", once(&[(s::DROP, 0.1)])).then("fall"))
        // Dashes, climbing, walls.
        .state(StateSpec::new("dash_side", looping(&s::DASH_SIDE, 0.04)))
        .state(StateSpec::new("dash_up", looping(&s::DASH_UP, 0.04)))
        .state(StateSpec::new("dash_down", looping(&s::DASH_DOWN, 0.04)))
        .state(StateSpec::new("climb", looping(&s::CLIMB, 0.12)).rate_by("vy", 45.0, 0.0, 2.0))
        .state(StateSpec::new("wall_slide", looping(&s::WALL_SLIDE, 0.08)))
        .state(StateSpec::new("grip_slip", looping(&s::WALL_SLIDE, 0.12)))
        // Dive.
        .state(StateSpec::new("dive_windup", once(&[(s::DIVE_WINDUP, 0.08)])).then("dive"))
        .state(StateSpec::new("dive", looping(&s::DIVE, 0.04)))
        .state(
            StateSpec::new(
                "dive_land",
                once(&[(s::DIVE_LAND_A, 0.06), (s::DIVE_LAND_B, 0.14)]),
            )
            .lock(0.2)
            .then("idle"),
        )
        // Events, from anywhere, highest first.
        .transition(
            TransitionSpec::any("dive_land", trig("impact"))
                .priority(100)
                .breaks_lock(),
        )
        .transition(
            TransitionSpec::any("dive_windup", trig("dive"))
                .priority(95)
                .breaks_lock(),
        )
        .transition(
            TransitionSpec::any("roll", trig("roll"))
                .priority(90)
                .breaks_lock(),
        )
        .transition(
            TransitionSpec::any("hard_land", trig("hard"))
                .priority(90)
                .breaks_lock(),
        )
        .transition(
            TransitionSpec::any("dash_up", trig("dash").and(below("dash_y", -0.3)))
                .priority(85)
                .breaks_lock()
                .restart(),
        )
        .transition(
            TransitionSpec::any("dash_down", trig("dash").and(above("dash_y", 0.3)))
                .priority(85)
                .breaks_lock()
                .restart(),
        )
        .transition(
            TransitionSpec::any("dash_side", trig("dash"))
                .priority(84)
                .breaks_lock()
                .restart(),
        )
        .transition(
            TransitionSpec::any("double", trig("double"))
                .priority(80)
                .breaks_lock()
                .restart(),
        )
        .transition(
            TransitionSpec::any("wall_jump", trig("wall_jump"))
                .priority(80)
                .breaks_lock(),
        )
        .transition(
            TransitionSpec::any("climb", trig("grab"))
                .priority(75)
                .breaks_lock(),
        )
        .transition(
            TransitionSpec::any("takeoff", trig("jump"))
                .priority(70)
                .breaks_lock(),
        )
        .transition(
            TransitionSpec::any("hop", trig("hop"))
                .priority(70)
                .breaks_lock(),
        )
        .transition(
            TransitionSpec::any("drop", trig("drop"))
                .priority(70)
                .breaks_lock(),
        )
        .transition(
            TransitionSpec::any("land", trig("land"))
                .except(["climb", "grip_slip", "roll", "hard_land", "dive_land"])
                .priority(60)
                .breaks_lock(),
        )
        // Leaving continuous states.
        .transition(
            TransitionSpec::new(DASHES, "idle", not("dashing").and(is("grounded"))).priority(50),
        )
        .transition(
            TransitionSpec::new(DASHES, "rise", not("dashing").and(below("vy", -APEX)))
                .priority(49),
        )
        .transition(TransitionSpec::new(DASHES, "fall", not("dashing")).priority(48))
        .transition(TransitionSpec::new(["climb"], "grip_slip", is("slipping")).priority(51))
        .transition(
            TransitionSpec::new(["grip_slip"], "climb", not("slipping").and(is("climbing")))
                .priority(51),
        )
        .transition(TransitionSpec::new(["grip_slip"], "fall", not("climbing")).priority(50))
        .transition(TransitionSpec::new(["climb"], "fall", not("climbing")).priority(50))
        .transition(
            TransitionSpec::new(
                ["roll"],
                "run",
                not("rolling")
                    .and(is("grounded"))
                    .and(above("speed", STILL)),
            )
            .priority(50),
        )
        .transition(
            TransitionSpec::new(["roll"], "idle", not("rolling").and(is("grounded"))).priority(49),
        )
        .transition(TransitionSpec::new(["roll"], "fall", not("rolling")).priority(48))
        .transition(
            TransitionSpec::new(
                ["hard_land"],
                "run",
                not("hard_landing").and(above("speed", STILL)),
            )
            .priority(50),
        )
        // Walls.
        .transition(
            TransitionSpec::new(["rise", "apex", "fall"], "wall_slide", is("wall_slide"))
                .priority(40),
        )
        .transition(TransitionSpec::new(["wall_slide"], "idle", is("grounded")).priority(41))
        .transition(TransitionSpec::new(["wall_slide"], "fall", not("wall_slide")).priority(40))
        // Ground.
        .transition(TransitionSpec::new(GROUND, "fall", not("grounded")).priority(30))
        .transition(TransitionSpec::new(["run", "idle"], "skid", is("skid")).priority(21))
        .transition(
            TransitionSpec::new(["skid"], "run", not("skid").and(above("speed", STILL)))
                .priority(20),
        )
        .transition(TransitionSpec::new(["skid"], "idle", not("skid")).priority(19))
        .transition(TransitionSpec::new(["idle"], "run", above("speed", STILL)).priority(10))
        .transition(TransitionSpec::new(["run"], "idle", below("speed", STILL)).priority(10))
        .transition(
            TransitionSpec::new(
                ["land"],
                "run",
                above("speed", STILL).and(Cond::After(0.05)),
            )
            .priority(10),
        )
        // Air arc.
        .transition(TransitionSpec::new(["rise"], "fall", above("vy", APEX)).priority(11))
        .transition(TransitionSpec::new(["rise"], "apex", above("vy", -APEX)).priority(10))
        .transition(TransitionSpec::new(["apex"], "fall", above("vy", APEX)).priority(10))
        // Safety net: an air state on the ground (a landing it missed) settles.
        .transition(TransitionSpec::new(
            AIR,
            "idle",
            is("grounded").and(Cond::After(0.05)),
        ))
}

/// The hero's compiled graph. Panics if the spec is broken; the tests compile it.
pub fn graph() -> Arc<Graph> {
    Arc::new(
        spec()
            .compile()
            .expect("the hero's animation graph compiles"),
    )
}
