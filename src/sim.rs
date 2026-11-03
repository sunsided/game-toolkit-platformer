//! The deterministic simulation: a level of solid tiles and the hero moving through it.
//!
//! No toolkit types. [`World::step`] advances one fixed step of [`DT`]; the same
//! [`Intent`]s give the same results, so a cloned world plays out identically (the tests
//! rely on that, and so will rewinds and replays).
//!
//! Movement follows the Celeste school: integer pixel positions with a sub-pixel
//! remainder, moved one pixel at a time against the tile grid, so collisions are exact and
//! nothing tunnels. All feel numbers live in [`Tuning`].
//!
//! On its own: an [`Intent`] in each fixed step, [`Event`]s out.
//!
//! ```
//! use game_toolkit_platformer::sim::{Event, Intent, JumpKind, Level, Preset, World};
//!
//! let mut level = Level::demo();
//! let mut world = World::on(&level, Preset::Precise.tuning());
//! world.step(&mut level, &Intent::default());
//! world.step(&mut level, &Intent { jump_pressed: true, jump_held: true, ..Intent::default() });
//! assert!(world.events.iter().any(|e| matches!(e, Event::Jumped { kind: JumpKind::Ground, .. })));
//! ```

pub mod ability;
pub mod crystal;
pub mod event;
pub mod intent;
pub mod level;
pub mod moves;
pub mod player;
pub mod rect;
pub mod terrain;
pub mod tuning;

pub use ability::{Ability, Step};
pub use crystal::Crystal;
pub use event::{Event, JumpKind};
pub use intent::Intent;
pub use level::{Level, TILE, Tile};
pub use moves::Moves;
pub use player::{Mode, Player};
pub use rect::Rect;
pub use terrain::Terrain;
pub use tuning::{Preset, Tuning};

/// The fixed simulation step in seconds (120 Hz).
pub const DT: f32 = 1.0 / 120.0;

/// The character's movement: its body, its feel, and the bookkeeping between steps. The
/// world around it is a [`Terrain`] passed to every [`World::step`], so it stays yours.
#[derive(Clone, Debug)]
pub struct World {
    /// The character's body.
    pub player: Player,
    /// The movement feel.
    pub tuning: Tuning,
    /// Where it starts, and comes back to after falling out (hitbox top-left, units).
    pub spawn: (i32, i32),
    /// Put it back at [`World::spawn`] when it falls out (an [`Event::FellOut`] is
    /// reported either way).
    pub respawn: bool,
    /// What it rode when it finished moving this step (see [`Player::riding`]), for
    /// [`World::settle`].
    riding: Option<Rect>,
    /// Steps taken so far.
    pub tick: u64,
    /// What happened in the last step.
    pub events: Vec<Event>,
    /// Moves of your own, run in every step in this order (see [`Ability`]).
    pub abilities: Vec<Box<dyn Ability>>,
}

impl World {
    /// A character standing with its hitbox's top-left at `spawn` (units).
    pub fn new(tuning: Tuning, spawn: (i32, i32)) -> Self {
        Self {
            player: Player::at(spawn, &tuning),
            tuning,
            spawn,
            respawn: true,
            riding: None,
            tick: 0,
            events: Vec::new(),
            abilities: Vec::new(),
        }
    }

    /// A character at `level`'s spawn tile.
    pub fn on(level: &Level, tuning: Tuning) -> Self {
        Self::new(tuning, level.spawn_point(tuning.size.0, tuning.size.1))
    }

    /// Advance one step of [`DT`] with `intent` in `terrain`. [`World::events`] then holds
    /// what happened in this step.
    ///
    /// It runs [`World::act`], the terrain's [`Terrain::update`] and [`World::settle`];
    /// several characters on one terrain run those apart, so the terrain updates once per
    /// step (see [`crate::Party`]).
    pub fn step(&mut self, terrain: &mut dyn Terrain, intent: &Intent) {
        self.act(terrain, intent);
        terrain.update(DT, &[self.player.rect()]);
        self.settle(terrain, None);
    }

    /// The first part of a step: the character moves (your abilities, the built-in moves,
    /// refills), the terrain doesn't update yet.
    pub fn act(&mut self, terrain: &mut dyn Terrain, intent: &Intent) {
        self.events.clear();
        let mut moves = Moves::ALL;
        self.abilities(terrain, intent, &mut moves, |a, s| a.before(s));
        self.player.step(
            &*terrain,
            intent,
            &moves,
            &self.tuning,
            DT,
            &mut self.events,
        );
        self.abilities(terrain, intent, &mut moves, |a, s| a.after(s));
        let body = self.player.rect();
        if self.player.dashes < self.tuning.dashes && terrain.refill(body) {
            self.player.dashes = self.tuning.dashes;
            self.events.push(Event::Recharged { crystal: true });
        }
        self.riding = self.player.riding();
    }

    /// The last part of a step, after the terrain's update: moving ground carries the
    /// character (`extra` adds to how far, for riding on other characters), and it may be
    /// squished or fall out.
    pub fn settle(&mut self, terrain: &mut dyn Terrain, extra: Option<(f32, f32)>) {
        let carry = match (self.riding.map(|r| terrain.carry(r)), extra) {
            (Some((x, y)), Some((ex, ey))) => Some((x + ex, y + ey)),
            (c, None) | (None, c) => c,
        };
        self.player.ride(&*terrain, carry, &self.tuning, DT);
        let body = self.player.rect();
        if terrain.solid(body) {
            self.events.push(Event::Squished);
        }
        if terrain.fell_out(body) {
            self.events.push(Event::FellOut);
            if self.respawn {
                self.respawn_at(self.spawn);
            }
        }
        self.tick += 1;
    }

    /// Start over with the hitbox's top-left at `at` (units): a fresh body, standing, with
    /// everything refilled, and an [`Event::Respawned`] in [`World::events`]. For hazards
    /// and checkpoints; [`World::spawn`] stays as it is.
    pub fn respawn_at(&mut self, at: (i32, i32)) {
        self.player = Player::at(at, &self.tuning);
        self.riding = None;
        self.events.push(Event::Respawned);
    }

    /// Run `hook` of every ability.
    fn abilities(
        &mut self,
        terrain: &mut dyn Terrain,
        intent: &Intent,
        moves: &mut Moves,
        hook: fn(&mut dyn Ability, &mut Step<'_>),
    ) {
        let mut s = Step {
            player: &mut self.player,
            intent,
            terrain,
            tuning: &self.tuning,
            moves,
            events: &mut self.events,
            dt: DT,
        };
        for a in &mut self.abilities {
            hook(a.as_mut(), &mut s);
        }
    }
}

#[cfg(test)]
mod tests;
