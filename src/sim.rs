//! The deterministic simulation: a level of solid tiles and the hero moving through it.
//!
//! No toolkit types. [`World::step`] advances one fixed step of [`DT`]; the same
//! [`Intent`]s give the same results, so a cloned world plays out identically (the tests
//! rely on that, and so will rewinds and replays).
//!
//! Movement follows the Celeste school: integer pixel positions with a sub-pixel
//! remainder, moved one pixel at a time against the tile grid, so collisions are exact and
//! nothing tunnels. All feel numbers live in [`Tuning`].

pub mod block;
pub mod crystal;
pub mod event;
pub mod intent;
pub mod level;
pub mod player;
pub mod rect;
pub mod tuning;

pub use block::Block;
pub use crystal::Crystal;
pub use event::{Event, JumpKind};
pub use intent::Intent;
pub use level::{Level, TILE, Tile};
pub use player::{Mode, Player, Shove};
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
    /// The pushable blocks.
    pub blocks: Vec<Block>,
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
        let blocks: Vec<Block> = level.block_spawns.iter().map(|&at| Block::at(at)).collect();
        let mut level = level;
        level.blocks = blocks.iter().map(Block::rect).collect();
        Self {
            level,
            player,
            tuning,
            crystals,
            blocks,
            tick: 0,
            events: Vec::new(),
        }
    }

    /// Advance one step of [`DT`] with `intent`. [`World::events`] then holds what
    /// happened in this step.
    pub fn step(&mut self, intent: &Intent) {
        self.events.clear();
        let shove = self.shove(intent);
        self.player.shove = shove.map_or(Shove::None, |(s, _)| s);
        if let Some((Shove::Push { side }, i)) = shove {
            // The block moves first; the hero follows into the room it made.
            let amount = side as f32 * self.tuning.push_speed * DT;
            let hero = self.player.rect();
            self.blocks[i].shove(&self.level, i, hero, amount);
            self.sync_blocks();
        }
        let before = self.player.x;
        self.player
            .step(&self.level, intent, &self.tuning, DT, &mut self.events);
        if let Some((Shove::Pull { .. }, i)) = shove {
            // The block follows the hero by as many pixels as the hero moved.
            let dx = self.player.x - before;
            let hero = self.player.rect();
            self.blocks[i].shove(&self.level, i, hero, dx as f32);
        }
        let hero = self.player.rect();
        for i in 0..self.blocks.len() {
            self.sync_blocks();
            self.blocks[i].fall(&self.level, i, hero, &self.tuning, DT);
        }
        self.sync_blocks();
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

    /// Copy the blocks' boxes into the level, where collisions see them.
    fn sync_blocks(&mut self) {
        self.level.blocks.clear();
        self.level
            .blocks
            .extend(self.blocks.iter().map(Block::rect));
    }

    /// The hero's hold on a block this step, and which block. Standing beside a resting
    /// block, walking into it pushes; holding grab facing it holds it, and walking away
    /// pulls.
    fn shove(&self, i: &Intent) -> Option<(Shove, usize)> {
        let p = &self.player;
        if !self.tuning.push || !p.grounded || p.mode != Mode::Normal {
            return None;
        }
        let mx = if i.move_x.abs() > 0.3 {
            i.move_x.signum() as i32
        } else {
            0
        };
        for side in [p.facing, -p.facing] {
            // Within 2 px: a block that just moved a pixel ahead still counts.
            let Some(b) = (1..=2).find_map(|d| self.level.block_at(p.rect().shifted(side * d, 0)))
            else {
                continue;
            };
            if !self.blocks[b].grounded {
                continue;
            }
            if mx == side {
                return Some((Shove::Push { side }, b));
            }
            if i.grab && side == p.facing {
                let s = if mx == -side {
                    Shove::Pull { side }
                } else {
                    Shove::Hold { side }
                };
                return Some((s, b));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests;
