//! [`Cond`]: when a transition fires, as a tree of tests on parameters and state.

use super::{ParamId, Params};

/// A condition over parameters `P` (names in a [`GraphSpec`](super::GraphSpec), ids in a
/// compiled [`Graph`](super::Graph)).
#[derive(Clone, Debug, PartialEq)]
pub enum Cond<P = ParamId> {
    /// Always holds.
    Always,
    /// A bool parameter is on.
    Is(P),
    /// A trigger is pending. A transition that fires consumes the triggers it tested.
    Trigger(P),
    /// A float parameter is above the value.
    Above(P, f32),
    /// A float parameter is below the value.
    Below(P, f32),
    /// The state's clip has finished (only [`Playback::Once`](super::Playback) clips do).
    Finished,
    /// At least this many seconds in the state.
    After(f32),
    /// The inner condition does not hold. Triggers under a `Not` are never consumed.
    Not(Box<Cond<P>>),
    /// Every inner condition holds.
    All(Vec<Cond<P>>),
    /// At least one inner condition holds.
    Any(Vec<Cond<P>>),
}

impl<P> Cond<P> {
    /// The negation of `c`.
    #[allow(clippy::should_implement_trait)]
    pub fn not(c: Cond<P>) -> Self {
        Cond::Not(Box::new(c))
    }

    /// Both `self` and `o`.
    pub fn and(self, o: Cond<P>) -> Self {
        match self {
            Cond::All(mut v) => {
                v.push(o);
                Cond::All(v)
            }
            c => Cond::All(vec![c, o]),
        }
    }

    /// The same condition over other parameter references, or the first error `f` gives.
    pub fn resolve<Q, E>(&self, f: &mut dyn FnMut(&P, Use) -> Result<Q, E>) -> Result<Cond<Q>, E> {
        Ok(match self {
            Cond::Always => Cond::Always,
            Cond::Is(p) => Cond::Is(f(p, Use::Bool)?),
            Cond::Trigger(p) => Cond::Trigger(f(p, Use::Trigger)?),
            Cond::Above(p, v) => Cond::Above(f(p, Use::Float)?, *v),
            Cond::Below(p, v) => Cond::Below(f(p, Use::Float)?, *v),
            Cond::Finished => Cond::Finished,
            Cond::After(s) => Cond::After(*s),
            Cond::Not(c) => Cond::Not(Box::new(c.resolve(f)?)),
            Cond::All(v) => Cond::All(v.iter().map(|c| c.resolve(f)).collect::<Result<_, E>>()?),
            Cond::Any(v) => Cond::Any(v.iter().map(|c| c.resolve(f)).collect::<Result<_, E>>()?),
        })
    }
}

/// How a condition uses a parameter, so compiling can check the parameter's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Use {
    /// As a bool.
    Bool,
    /// As a float.
    Float,
    /// As a trigger.
    Trigger,
}

impl Cond {
    /// Whether the condition holds, given the parameters, whether the clip finished and
    /// seconds in the state.
    pub fn eval(&self, p: &Params, finished: bool, time: f32) -> bool {
        match self {
            Cond::Always => true,
            Cond::Is(id) => p.get_bool(*id),
            Cond::Trigger(id) => p.pending(*id),
            Cond::Above(id, v) => p.get_float(*id) > *v,
            Cond::Below(id, v) => p.get_float(*id) < *v,
            Cond::Finished => finished,
            Cond::After(s) => time >= *s,
            Cond::Not(c) => !c.eval(p, finished, time),
            Cond::All(v) => v.iter().all(|c| c.eval(p, finished, time)),
            Cond::Any(v) => v.iter().any(|c| c.eval(p, finished, time)),
        }
    }

    /// The pending triggers this condition tested positively, to consume when its
    /// transition fires.
    pub fn used_triggers(&self, p: &Params, out: &mut Vec<ParamId>) {
        match self {
            Cond::Trigger(id) if p.pending(*id) => out.push(*id),
            Cond::All(v) | Cond::Any(v) => v.iter().for_each(|c| c.used_triggers(p, out)),
            _ => {}
        }
    }
}
