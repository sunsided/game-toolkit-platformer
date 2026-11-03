//! [`Graph`]: a compiled, validated state graph: states with clips, transitions by
//! priority, the parameter template. Build one with [`GraphSpec`](super::GraphSpec).

use super::{Clip, Cond, Name, ParamId, Params};

/// A state, by index into its [`Graph`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StateId(pub(crate) u16);

/// How fast a state plays its clip.
#[derive(Clone, Debug, PartialEq)]
pub enum Rate<P = ParamId> {
    /// A fixed multiplier.
    Fixed(f32),
    /// Scaled by a float parameter's magnitude: `|param| / per`, clamped to `min..=max`
    /// (a run cycle that keeps pace with the speed; a climb that stops when the hero does).
    Param {
        /// The float parameter.
        param: P,
        /// The parameter value that plays at rate 1.
        per: f32,
        /// Slowest rate.
        min: f32,
        /// Fastest rate.
        max: f32,
    },
}

/// A state of the graph.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    /// Its name.
    pub name: Name,
    /// What it plays.
    pub clip: Clip,
    /// How fast.
    pub rate: Rate,
    /// Seconds after entering during which only lock-breaking transitions are considered.
    pub lock: f32,
    /// Where to go when the clip finishes and no transition fired.
    pub next: Option<StateId>,
}

/// Where a transition can start.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    /// From these states.
    States(Vec<StateId>),
    /// From any state except these.
    Any {
        /// States it doesn't apply to.
        except: Vec<StateId>,
    },
}

impl Source {
    /// Whether the transition applies in state `s`.
    pub fn applies(&self, s: StateId) -> bool {
        match self {
            Source::States(v) => v.contains(&s),
            Source::Any { except } => !except.contains(&s),
        }
    }
}

/// A transition between states.
#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    /// Where it can start.
    pub from: Source,
    /// Where it goes.
    pub to: StateId,
    /// When it fires.
    pub when: Cond,
    /// Higher goes first; ties in declaration order.
    pub priority: i32,
    /// Considered even while the current state is locked.
    pub breaks_lock: bool,
    /// May fire into the state it starts from, restarting it.
    pub restart: bool,
}

/// A compiled state graph.
#[derive(Clone, Debug, PartialEq)]
pub struct Graph {
    /// The parameters, with their initial values; every animator gets a copy.
    pub params: Params,
    /// The states.
    pub states: Vec<State>,
    /// The transitions, highest priority first.
    pub transitions: Vec<Transition>,
    /// The starting state.
    pub entry: StateId,
}

impl Graph {
    /// The state `id`.
    pub fn state(&self, id: StateId) -> &State {
        &self.states[id.0 as usize]
    }

    /// The state called `name`.
    pub fn find(&self, name: &str) -> Option<StateId> {
        self.states
            .iter()
            .position(|s| s.name == name)
            .map(|i| StateId(i as u16))
    }

    /// Every state id.
    pub fn ids(&self) -> impl Iterator<Item = StateId> {
        (0..self.states.len()).map(|i| StateId(i as u16))
    }
}
