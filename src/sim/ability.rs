//! [`Ability`]: a move of your game's own (an attack lunge, a ledge grab, a grapple, a
//! pogo), hooked into [`World::step`](super::World::step) without forking the movement.
//!
//! An ability runs twice per step: [`Ability::before`] ahead of the built-in moves, and
//! [`Ability::after`] once they moved the body. It may change anything public on the
//! [`Player`] (speed, facing, refills, [`Player::launch`] and [`Player::teleport`]), read
//! the [`Intent`] (your own buttons arrive in [`Intent::custom`]), read and change the
//! terrain (your own type through [`find_mut`](trait.Terrain.html#method.find_mut): break a
//! block, flip a switch, use up a pickup; the movement sees the change at once), and report
//! [`Event::Custom`]s (with up to four numbers of data: a knockback, a hit target).
//!
//! Name your modes and events with a [`CustomId`](super::CustomId) after your crate
//! (`CustomId::new("my_crate::lunge")`): abilities from different crates then never have
//! to agree on numbers. Custom buttons ([`Intent::custom`]) belong to the game, which binds
//! them to keys: an ability meant for others takes its button's bit as a parameter.
//!
//! To replace a built-in move, rule it out and do your own in its place. To react to
//! collisions, read [`Player::contacts`] in [`Ability::after`]. In a [`crate::Party`],
//! [`Step::others`] holds the other characters' bodies.
//!
//! To rule out single built-in moves for a step (no dash while attacking, no wall grab
//! while carrying, slow walking while pushing), clear them in [`Step::moves`] in
//! [`Ability::before`].
//!
//! To take the body over for a while, put it in [`Mode::Custom`](super::Mode::Custom):
//! no built-in move starts or runs then (no running, no gravity, no jumps or dashes), but
//! the body still moves by its speed through the collision, lands and is carried by moving
//! ground. The mode's clock counts up on its own; set [`Mode::Normal`](super::Mode::Normal)
//! to hand the body back.
//!
//! Abilities are part of the world: a cloned world clones them, and it must play out
//! identically, so keep their state in the ability (no clocks or randomness from outside)
//! and derive `Clone`.
//!
//! ```
//! use game_toolkit_platformer::sim::{Ability, CustomId, Event, Intent, Mode, Preset, Spot, Step, World};
//!
//! /// Custom button 0 lunges forward on the ground.
//! #[derive(Clone, Debug)]
//! struct Lunge;
//!
//! /// The lunge's mode and event id, named after the crate so no other ability's collides.
//! const LUNGE: CustomId = CustomId::new("my_game::lunge");
//!
//! impl Ability for Lunge {
//!     fn before(&mut self, s: &mut Step<'_>) {
//!         let p = &mut *s.player;
//!         match p.mode {
//!             Mode::Normal if s.intent.pressed(0) && p.grounded => {
//!                 p.mode = Mode::Custom { id: LUNGE, t: 0.0 };
//!                 p.vx = 240.0 * p.facing as f32;
//!                 p.vy = 0.0;
//!                 s.events.push(Event::custom(LUNGE).with([p.vx, 0.0, 0.0, 0.0]));
//!             }
//!             Mode::Custom { id: LUNGE, t } if t >= 0.15 => p.mode = Mode::Normal,
//!             _ => {}
//!         }
//!         // No dash out of the lunge's recovery, while custom button 0 is held.
//!         s.moves.dash = !s.intent.held(0);
//!     }
//! }
//!
//! let mut world = World::new(Preset::Hybrid.tuning(), Spot::TopLeft(20, 88));
//! world.abilities.push(Box::new(Lunge));
//! let mut floor = |r: game_toolkit_platformer::sim::Rect| r.bottom() > 100;
//! world.step(&mut floor, &Intent { custom_pressed: 1, ..Intent::default() });
//! assert!(world.events.iter().any(|e| e.custom_data(LUNGE).is_some()));
//! ```

use std::any::Any;
use std::fmt::Debug;

use super::{Event, Intent, Moves, Player, Rect, Terrain, Tuning};

/// What an [`Ability`] gets to work with in one step.
pub struct Step<'a> {
    /// The body.
    pub player: &'a mut Player,
    /// This step's input.
    pub intent: &'a Intent,
    /// The world around it; changes count from here on, in this step too.
    pub terrain: &'a mut dyn Terrain,
    /// The movement feel.
    pub tuning: &'a Tuning,
    /// The built-in moves allowed this step: clear some in [`Ability::before`]. Changes in
    /// [`Ability::after`] come too late for this step (and the next starts from
    /// [`Moves::ALL`] again).
    pub moves: &'a mut Moves,
    /// This step's events so far; push yours ([`Event::Custom`]).
    pub events: &'a mut Vec<Event>,
    /// The other characters' bodies on the same terrain (a [`crate::Party`]), as the step
    /// started for those yet to move and as they ended up for those already moved; empty
    /// for a character on its own.
    pub others: &'a [Rect],
    /// The step's length in seconds ([`super::DT`]).
    pub dt: f32,
}

/// A move of your own, run in every [`World::step`](super::World::step). Derive `Clone`
/// and `Debug`; both hooks default to nothing.
pub trait Ability: AbilityClone + Debug {
    /// Ahead of the built-in moves: start your move, steer it, end it; rule out built-in
    /// moves ([`Step::moves`]).
    fn before(&mut self, _s: &mut Step<'_>) {}

    /// After the built-in moves, collision and landing: react to where the body ended up
    /// and what it ran into ([`Player::contacts`]: a wall, a ceiling, the floor).
    fn after(&mut self, _s: &mut Step<'_>) {}
}

/// Cloning a boxed [`Ability`], so a world with abilities stays `Clone`, and reaching its
/// concrete type ([`super::World::ability`]). Implemented for every `Ability + Clone`; you
/// don't implement it yourself.
pub trait AbilityClone {
    /// A boxed copy.
    fn clone_box(&self) -> Box<dyn Ability>;
    /// Itself, to downcast.
    fn as_any(&self) -> &dyn Any;
    /// Itself, to downcast and change.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Ability + Clone + 'static> AbilityClone for T {
    fn clone_box(&self) -> Box<dyn Ability> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Clone for Box<dyn Ability> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
