//! [`GraphSpec`]: a state graph by names, compiled into a validated [`Graph`].
//!
//! Everything refers to parameters and states by name, so a spec can be written in code
//! with the builder methods or produced from data. [`GraphSpec::compile`] resolves the
//! names, checks that every condition uses a parameter of the right kind, and orders the
//! transitions by priority.

use std::fmt;

use super::cond::Use;
use super::{
    Clip, Cond, Graph, Name, ParamId, ParamKind, Params, Rate, Source, State, StateId, Transition,
};

/// A state by names.
#[derive(Clone, Debug, PartialEq)]
pub struct StateSpec {
    /// Its name, unique in the graph.
    pub name: Name,
    /// What it plays.
    pub clip: Clip,
    /// How fast, by parameter name.
    pub rate: Rate<Name>,
    /// Seconds only lock-breaking transitions may leave it.
    pub lock: f32,
    /// Where to go when the clip finishes.
    pub next: Option<Name>,
}

impl StateSpec {
    /// A state `name` playing `clip` at rate 1, unlocked, staying when the clip ends.
    pub fn new(name: impl Into<Name>, clip: Clip) -> Self {
        Self {
            name: name.into(),
            clip,
            rate: Rate::Fixed(1.0),
            lock: 0.0,
            next: None,
        }
    }

    /// Play at `|param| / per` speed, clamped to `min..=max`.
    pub fn rate_by(mut self, param: impl Into<Name>, per: f32, min: f32, max: f32) -> Self {
        self.rate = Rate::Param {
            param: param.into(),
            per,
            min,
            max,
        };
        self
    }

    /// Only lock-breaking transitions leave it for the first `secs`.
    pub fn lock(mut self, secs: f32) -> Self {
        self.lock = secs;
        self
    }

    /// Go to `next` when the clip finishes.
    pub fn then(mut self, next: impl Into<Name>) -> Self {
        self.next = Some(next.into());
        self
    }
}

/// A transition by names.
#[derive(Clone, Debug, PartialEq)]
pub struct TransitionSpec {
    /// Source states; empty means any state.
    pub from: Vec<Name>,
    /// With `from` empty: states it does not apply to.
    pub except: Vec<Name>,
    /// Target state.
    pub to: Name,
    /// When it fires.
    pub when: Cond<Name>,
    /// Higher goes first.
    pub priority: i32,
    /// Considered while the current state is locked.
    pub breaks_lock: bool,
    /// May restart the state it starts from.
    pub restart: bool,
}

impl TransitionSpec {
    /// From any of `from` to `to` when `when` holds.
    pub fn new<S: Into<Name>>(
        from: impl IntoIterator<Item = S>,
        to: impl Into<Name>,
        when: Cond<Name>,
    ) -> Self {
        Self {
            from: from.into_iter().map(Into::into).collect(),
            except: Vec::new(),
            to: to.into(),
            when,
            priority: 0,
            breaks_lock: false,
            restart: false,
        }
    }

    /// From any state to `to` when `when` holds.
    pub fn any(to: impl Into<Name>, when: Cond<Name>) -> Self {
        Self::new(std::iter::empty::<Name>(), to, when)
    }

    /// Not from these states (with [`TransitionSpec::any`]).
    pub fn except<S: Into<Name>>(mut self, states: impl IntoIterator<Item = S>) -> Self {
        self.except = states.into_iter().map(Into::into).collect();
        self
    }

    /// Set the priority.
    pub fn priority(mut self, p: i32) -> Self {
        self.priority = p;
        self
    }

    /// Fire even while the current state is locked.
    pub fn breaks_lock(mut self) -> Self {
        self.breaks_lock = true;
        self
    }

    /// Allow restarting the current state.
    pub fn restart(mut self) -> Self {
        self.restart = true;
        self
    }
}

/// A whole graph by names.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GraphSpec {
    /// Declared parameters.
    pub params: Vec<(Name, ParamKind)>,
    /// The states; the first is the entry unless [`GraphSpec::entry`] says otherwise.
    pub states: Vec<StateSpec>,
    /// The transitions.
    pub transitions: Vec<TransitionSpec>,
    /// The starting state's name; `None`: the first state.
    pub entry: Option<Name>,
}

/// Why a spec doesn't compile.
#[derive(Clone, Debug, PartialEq)]
pub enum SpecError {
    /// There are no states.
    NoStates,
    /// Two states share a name.
    DuplicateState(Name),
    /// Two parameters share a name.
    DuplicateParam(Name),
    /// A name that isn't a state.
    UnknownState(Name),
    /// A name that isn't a parameter.
    UnknownParam(Name),
    /// A parameter used as the wrong kind (a float tested as a trigger, ...).
    WrongKind {
        /// The parameter.
        param: Name,
        /// Its declared kind.
        declared: ParamKind,
    },
}

impl fmt::Display for SpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpecError::NoStates => write!(f, "the graph has no states"),
            SpecError::DuplicateState(n) => write!(f, "state {n:?} is declared twice"),
            SpecError::DuplicateParam(n) => write!(f, "parameter {n:?} is declared twice"),
            SpecError::UnknownState(n) => write!(f, "no state called {n:?}"),
            SpecError::UnknownParam(n) => write!(f, "no parameter called {n:?}"),
            SpecError::WrongKind { param, declared } => {
                write!(
                    f,
                    "parameter {param:?} is a {declared:?} and can't be used that way"
                )
            }
        }
    }
}

impl std::error::Error for SpecError {}

impl GraphSpec {
    /// An empty spec.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declare a bool parameter.
    pub fn bool(mut self, name: impl Into<Name>) -> Self {
        self.params.push((name.into(), ParamKind::Bool));
        self
    }

    /// Declare a float parameter.
    pub fn float(mut self, name: impl Into<Name>) -> Self {
        self.params.push((name.into(), ParamKind::Float));
        self
    }

    /// Declare a trigger that waits `ttl` seconds to be consumed.
    pub fn trigger(mut self, name: impl Into<Name>, ttl: f32) -> Self {
        self.params.push((name.into(), ParamKind::Trigger { ttl }));
        self
    }

    /// Add a state.
    pub fn state(mut self, s: StateSpec) -> Self {
        self.states.push(s);
        self
    }

    /// Add a transition.
    pub fn transition(mut self, t: TransitionSpec) -> Self {
        self.transitions.push(t);
        self
    }

    /// Start in the state `name`.
    pub fn entry(mut self, name: impl Into<Name>) -> Self {
        self.entry = Some(name.into());
        self
    }

    /// Resolve names, check kinds, order transitions by priority.
    pub fn compile(&self) -> Result<Graph, SpecError> {
        let mut params = Params::default();
        for (name, kind) in &self.params {
            if params.id(name).is_some() {
                return Err(SpecError::DuplicateParam(name.clone()));
            }
            params.declare(name.clone(), *kind);
        }
        if self.states.is_empty() {
            return Err(SpecError::NoStates);
        }
        let names: Vec<&Name> = self.states.iter().map(|s| &s.name).collect();
        for (i, n) in names.iter().enumerate() {
            if names[..i].contains(n) {
                return Err(SpecError::DuplicateState((*n).clone()));
            }
        }
        let state = |n: &Name| {
            names
                .iter()
                .position(|s| *s == n)
                .map(|i| StateId(i as u16))
                .ok_or_else(|| SpecError::UnknownState(n.clone()))
        };
        let mut param = |n: &Name, u: Use| -> Result<ParamId, SpecError> {
            let id = params
                .id(n)
                .ok_or_else(|| SpecError::UnknownParam(n.clone()))?;
            let declared = params.kind(id);
            let ok = matches!(
                (u, declared),
                (Use::Bool, ParamKind::Bool)
                    | (Use::Float, ParamKind::Float)
                    | (Use::Trigger, ParamKind::Trigger { .. })
            );
            if ok {
                Ok(id)
            } else {
                Err(SpecError::WrongKind {
                    param: n.clone(),
                    declared,
                })
            }
        };

        let mut states = Vec::with_capacity(self.states.len());
        for s in &self.states {
            let rate = match &s.rate {
                Rate::Fixed(r) => Rate::Fixed(*r),
                Rate::Param {
                    param: p,
                    per,
                    min,
                    max,
                } => Rate::Param {
                    param: param(p, Use::Float)?,
                    per: *per,
                    min: *min,
                    max: *max,
                },
            };
            states.push(State {
                name: s.name.clone(),
                clip: s.clip.clone(),
                rate,
                lock: s.lock,
                next: s.next.as_ref().map(state).transpose()?,
            });
        }

        let mut transitions = Vec::with_capacity(self.transitions.len());
        for t in &self.transitions {
            let ids = |v: &[Name]| v.iter().map(state).collect::<Result<Vec<_>, _>>();
            let from = if t.from.is_empty() {
                Source::Any {
                    except: ids(&t.except)?,
                }
            } else {
                Source::States(ids(&t.from)?)
            };
            transitions.push(Transition {
                from,
                to: state(&t.to)?,
                when: t.when.resolve(&mut param)?,
                priority: t.priority,
                breaks_lock: t.breaks_lock,
                restart: t.restart,
            });
        }
        transitions.sort_by_key(|t| std::cmp::Reverse(t.priority));

        let entry = match &self.entry {
            Some(n) => state(n)?,
            None => StateId(0),
        };
        Ok(Graph {
            params,
            states,
            transitions,
            entry,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::Playback;

    fn clip() -> Clip {
        Clip::uniform(Playback::Loop, &[0], 0.1)
    }

    #[test]
    fn unknown_names_and_wrong_kinds_are_errors() {
        let base = GraphSpec::new()
            .float("speed")
            .state(StateSpec::new("idle", clip()))
            .state(StateSpec::new("run", clip()));
        let bad_state =
            base.clone()
                .transition(TransitionSpec::new(["idle"], "walk", Cond::Always));
        assert_eq!(
            bad_state.compile(),
            Err(SpecError::UnknownState("walk".into()))
        );
        let bad_kind = base.clone().transition(TransitionSpec::new(
            ["idle"],
            "run",
            Cond::Trigger("speed".into()),
        ));
        assert!(matches!(
            bad_kind.compile(),
            Err(SpecError::WrongKind { .. })
        ));
        let ok = base.transition(TransitionSpec::new(
            ["idle"],
            "run",
            Cond::Above("speed".into(), 1.0),
        ));
        assert!(ok.compile().is_ok());
    }

    #[test]
    fn transitions_are_ordered_by_priority_stably() {
        let g = GraphSpec::new()
            .state(StateSpec::new("a", clip()))
            .state(StateSpec::new("b", clip()))
            .state(StateSpec::new("c", clip()))
            .transition(TransitionSpec::any("b", Cond::Always))
            .transition(TransitionSpec::any("c", Cond::Always).priority(5))
            .transition(TransitionSpec::any("a", Cond::Always))
            .compile()
            .unwrap();
        let order: Vec<_> = g
            .transitions
            .iter()
            .map(|t| g.state(t.to).name.as_ref())
            .collect();
        assert_eq!(order, ["c", "b", "a"]);
    }
}
