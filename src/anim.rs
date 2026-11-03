//! A pluggable, configurable animation controller: a state graph over named parameters,
//! independent of what it animates and of how frames are drawn.
//!
//! - [`Params`]: the controller's inputs, declared by name: bools, floats and triggers.
//!   Triggers are one-shot flags with a time to live: fired while a state is locked, they
//!   wait (the animation side of input queueing) until a transition consumes them or they
//!   expire.
//! - [`Clip`]: frames (a sprite index and a duration each), loop/once/ping-pong playback,
//!   and named markers (footsteps, impact frames) reported when a frame starts.
//! - [`Cond`]: when a transition fires, as a tree over parameters, the clip finishing and
//!   time in state.
//! - [`GraphSpec`]: the whole graph by names (params, states, transitions), data that can
//!   be written in code or loaded from a file, and [`GraphSpec::compile`]d into a
//!   validated [`Graph`] that refers to everything by id.
//! - [`Animator`]: runs one [`Graph`]: per update, advance the clip, pick the
//!   highest-priority transition whose condition holds (states can lock out all but
//!   lock-breaking transitions for a while), consume its triggers, enter the target.
//! - [`Driver`]: the plug that turns a subject (a simulated body, a cutscene, a test
//!   script) into parameter values each update.
//!
//! The controller never looks at the simulation, and the simulation never looks at it:
//! a driver maps facts to parameters, the graph maps parameters to sprites.

pub mod animator;
pub mod clip;
pub mod cond;
pub mod driver;
pub mod graph;
pub mod params;
pub mod spec;

pub use animator::{AnimEvent, Animator};
pub use clip::{Clip, Frame, Playback};
pub use cond::Cond;
pub use driver::Driver;
pub use graph::{Graph, Rate, Source, State, StateId, Transition};
pub use params::{ParamId, ParamKind, Params};
pub use spec::{GraphSpec, SpecError, StateSpec, TransitionSpec};

/// A state or parameter name: borrowed for specs written in code (`"run"` costs nothing),
/// owned for specs built from data.
pub type Name = std::borrow::Cow<'static, str>;
