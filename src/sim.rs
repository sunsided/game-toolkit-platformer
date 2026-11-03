//! The deterministic simulation: a level of solid tiles and the hero moving through it.
//!
//! No toolkit types. [`World::step`] advances one fixed step of [`DT`]; the same
//! [`Intent`]s give the same results, so a cloned world plays out identically (the tests
//! rely on that, and so will rewinds and replays).
//!
//! Movement follows the Celeste school: integer pixel positions with a sub-pixel
//! remainder, moved one pixel at a time against the tile grid, so collisions are exact and
//! nothing tunnels. All feel numbers live in [`Tuning`].

pub mod crystal;
pub mod event;
pub mod intent;
pub mod level;
pub mod player;
pub mod rect;
pub mod tuning;

pub use crystal::Crystal;
pub use event::{Event, JumpKind};
pub use intent::Intent;
pub use level::{Level, TILE, Tile};
pub use player::{Mode, Player};
pub use rect::Rect;
pub use tuning::{Preset, Tuning};

/// The fixed simulation step in seconds (120 Hz).
pub const DT: f32 = 1.0 / 120.0;

/// A level with the hero in it.
#[derive(Clone, Debug)]
pub struct World {
    /// The solid tiles.
    pub level: Level,
    /// The hero.
    pub player: Player,
    /// The movement feel.
    pub tuning: Tuning,
    /// The dash crystals.
    pub crystals: Vec<Crystal>,
    /// Steps taken so far.
    pub tick: u64,
    /// What happened in the last step.
    pub events: Vec<Event>,
}

impl World {
    /// A world on `level` with the hero standing at its spawn point.
    pub fn new(level: Level, tuning: Tuning) -> Self {
        let player = Player::spawn(&level, &tuning);
        let crystals = level.crystals.iter().map(|&at| Crystal::at(at)).collect();
        Self {
            level,
            player,
            tuning,
            crystals,
            tick: 0,
            events: Vec::new(),
        }
    }

    /// Advance one step of [`DT`] with `intent`. [`World::events`] then holds what
    /// happened in this step.
    pub fn step(&mut self, intent: &Intent) {
        self.events.clear();
        self.player
            .step(&self.level, intent, &self.tuning, DT, &mut self.events);
        let body = self.player.rect();
        for c in &mut self.crystals {
            if c.collect(
                body,
                DT,
                self.player.dashes < self.tuning.dashes,
                &self.tuning,
            ) {
                self.player.dashes = self.tuning.dashes;
                self.events.push(Event::Recharged { crystal: true });
            }
        }
        self.tick += 1;
    }
}

#[cfg(test)]
mod tests;
