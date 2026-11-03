//! [`Animate`]: an [`Ability`] that drives the animation parameters of its own move, so a
//! custom move is one registration ([`crate::Builder::animated`]) instead of an ability
//! plus a driver that must agree with it.
//!
//! The movement (`sim`) knows nothing of animation; this trait lives on the facade's side
//! and the [`crate::Platformer`] runs it after each step, after the built-in driver and
//! before your own drivers ([`crate::Builder::driver`]), with the ability's own state.

use std::any::TypeId;

use crate::anim::Params;
use crate::sim::{Ability, World};

/// The animation side of an [`Ability`]: write its parameters (`"charging"`, `"charge"`)
/// from its own state and the step's [`World::events`]. Declare them in your graph.
pub trait Animate {
    /// Set `params` after a step, from this ability's state and `world`.
    fn drive(&self, world: &World, params: &mut Params);
}

/// Runs the [`Animate`] half of the ability of one type in [`World::abilities`], found by
/// its type on every step, so adding, removing or reordering abilities can't make it
/// drive another one.
#[derive(Clone, Copy)]
pub(crate) struct AnimHook {
    /// The ability's type.
    pub ty: TypeId,
    /// Finds it in the world and calls its [`Animate::drive`].
    pub drive: fn(&World, &mut Params),
}

impl AnimHook {
    /// The hook for the ability of type `T`.
    pub fn of<T: Ability + Animate + 'static>() -> Self {
        Self {
            ty: TypeId::of::<T>(),
            drive: |w, p| {
                if let Some(t) = w.ability::<T>() {
                    t.drive(w, p);
                }
            },
        }
    }

    /// Drive `params` from the ability in `world`, if it is there.
    pub fn run(&self, world: &World, params: &mut Params) {
        (self.drive)(world, params);
    }

    /// Add `hook` to `hooks` unless one for its type is there already.
    pub fn add(hooks: &mut Vec<AnimHook>, hook: AnimHook) {
        if !hooks.iter().any(|h| h.ty == hook.ty) {
            hooks.push(hook);
        }
    }
}
