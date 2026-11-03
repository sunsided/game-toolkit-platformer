//! [`Animate`]: an [`Ability`] that drives the animation parameters of its own move, so a
//! custom move is one registration ([`crate::Builder::animated`]) instead of an ability
//! plus a driver that must agree with it.
//!
//! The movement (`sim`) knows nothing of animation; this trait lives on the facade's side
//! and the [`crate::Platformer`] runs it after each step, after the built-in driver and
//! before your own drivers ([`crate::Builder::driver`]), with the ability's own state.

use std::any::TypeId;
use std::mem::discriminant;

use crate::anim::{ParamId, ParamKind, Params, SpecError};
use crate::sim::{Ability, World};

/// The animation side of an [`Ability`]: write its parameters (`"charging"`, `"charge"`)
/// from its own state and the step's [`World::events`].
///
/// List them in [`Animate::PARAMS`]: the graph is checked against them when it is built
/// ([`crate::Builder::build`], [`crate::Platformer::add_animated`],
/// [`crate::Platformer::set_graph`]), so a misspelled or missing one is an error there
/// instead of a panic in the middle of a game, and [`Animate::drive`] gets their ids
/// without looking them up by name every step.
///
/// ```
/// use game_toolkit_platformer::anim::{ParamId, ParamKind, Params};
/// use game_toolkit_platformer::prelude::*;
/// use game_toolkit_platformer::sim::{Ability, World};
///
/// #[derive(Clone, Debug, Default)]
/// struct Charge { level: f32 }
/// impl Ability for Charge {}
///
/// impl Animate for Charge {
///     const PARAMS: &'static [(&'static str, ParamKind)] =
///         &[("charge", ParamKind::Float), ("charging", ParamKind::Bool)];
///
///     fn drive(&self, _world: &World, p: &mut Params, ids: &[ParamId]) {
///         p.set_float(ids[0], self.level);
///         p.set_bool(ids[1], self.level > 0.0);
///     }
/// }
///
/// // The built-in graph doesn't declare them: the builder says so.
/// let built = Platformer::builder().animated(Charge::default()).build();
/// assert!(built.is_err());
/// ```
pub trait Animate {
    /// The parameters [`Animate::drive`] writes, by name and kind (a trigger's `ttl` is
    /// the graph's business and isn't compared). Every one must be declared in the graph.
    const PARAMS: &'static [(&'static str, ParamKind)];

    /// Set `params` after a step, from this ability's state and `world`. `ids` are the
    /// ids of [`Animate::PARAMS`] in the graph, in that order.
    fn drive(&self, world: &World, params: &mut Params, ids: &[ParamId]);
}

/// Runs the [`Animate`] half of the ability of one type in [`World::abilities`], found by
/// its type on every step, so adding, removing or reordering abilities can't make it
/// drive another one.
#[derive(Clone)]
pub(crate) struct AnimHook {
    /// The ability's type.
    pub ty: TypeId,
    /// The parameters it writes ([`Animate::PARAMS`]).
    params: &'static [(&'static str, ParamKind)],
    /// Their ids in the current graph ([`AnimHook::resolve`]).
    ids: Vec<ParamId>,
    /// Finds it in the world and calls its [`Animate::drive`].
    drive: fn(&World, &mut Params, &[ParamId]),
}

impl AnimHook {
    /// The hook for the ability of type `T`, not yet resolved against a graph.
    pub fn of<T: Ability + Animate + 'static>() -> Self {
        Self {
            ty: TypeId::of::<T>(),
            params: T::PARAMS,
            ids: Vec::new(),
            drive: |w, p, ids| {
                if let Some(t) = w.ability::<T>() {
                    t.drive(w, p, ids);
                }
            },
        }
    }

    /// The ids of its parameters in `params`, or which one the graph lacks or declares as
    /// another kind.
    pub fn lookup(&self, params: &Params) -> Result<Vec<ParamId>, SpecError> {
        self.params
            .iter()
            .map(|&(name, kind)| {
                let id = params
                    .id(name)
                    .ok_or_else(|| SpecError::UnknownParam(name.into()))?;
                let declared = params.kind(id);
                if discriminant(&declared) == discriminant(&kind) {
                    Ok(id)
                } else {
                    Err(SpecError::WrongKind {
                        param: name.into(),
                        declared,
                    })
                }
            })
            .collect()
    }

    /// Look its parameters up in `params` (a new graph's) and keep their ids.
    pub fn resolve(&mut self, params: &Params) -> Result<(), SpecError> {
        self.ids = self.lookup(params)?;
        Ok(())
    }

    /// Keep `ids` ([`AnimHook::lookup`] in a new graph) as its parameters' ids.
    pub fn set_ids(&mut self, ids: Vec<ParamId>) {
        self.ids = ids;
    }

    /// Drive `params` from the ability in `world`, if it is there.
    pub fn run(&self, world: &World, params: &mut Params) {
        (self.drive)(world, params, &self.ids);
    }

    /// Add `hook` to `hooks` unless one for its type is there already.
    pub fn add(hooks: &mut Vec<AnimHook>, hook: AnimHook) {
        if !hooks.iter().any(|h| h.ty == hook.ty) {
            hooks.push(hook);
        }
    }
}
