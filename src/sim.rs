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

pub mod block;
pub mod crystal;
pub mod event;
pub mod intent;
pub mod level;
pub mod player;
pub mod rect;
pub mod terrain;
pub mod tuning;

pub use block::Block;
pub use crystal::Crystal;
pub use event::{Event, JumpKind};
pub use intent::Intent;
pub use level::{Level, TILE, Tile};
pub use player::{Mode, Player, Shove};
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
    /// Seconds the character has been pushing or pulling the same box the same way: boxes
    /// stick until [`Tuning::push_breakaway`], then speed up over [`Tuning::push_ramp`].
    pub effort: f32,
    /// What the character shoved last step, to tell whether the effort carries on.
    last_shove: Option<(Shove, usize)>,
    /// Steps taken so far.
    pub tick: u64,
    /// What happened in the last step.
    pub events: Vec<Event>,
}

impl World {
    /// A character standing with its hitbox's top-left at `spawn` (units).
    pub fn new(tuning: Tuning, spawn: (i32, i32)) -> Self {
        Self {
            player: Player::at(spawn, &tuning),
            tuning,
            spawn,
            respawn: true,
            effort: 0.0,
            last_shove: None,
            tick: 0,
            events: Vec::new(),
        }
    }

    /// A character at `level`'s spawn tile.
    pub fn on(level: &Level, tuning: Tuning) -> Self {
        Self::new(tuning, level.spawn_point(player::W, player::H))
    }

    /// Advance one step of [`DT`] with `intent` in `terrain`. [`World::events`] then holds
    /// what happened in this step.
    pub fn step(&mut self, terrain: &mut dyn Terrain, intent: &Intent) {
        self.events.clear();
        let shove = self.shove(&*terrain, intent);
        let moving = matches!(shove, Some((Shove::Push { .. } | Shove::Pull { .. }, _)));
        self.effort = if moving && shove == self.last_shove {
            self.effort + DT
        } else if moving {
            DT
        } else {
            0.0
        };
        self.last_shove = shove;
        // Static friction: nothing moves until the character has strained long enough.
        let t = &self.tuning;
        let give = ((self.effort - t.push_breakaway) / t.push_ramp.max(DT)).clamp(0.0, 1.0);
        self.player.shove = shove.map_or(Shove::None, |(s, _)| s);
        self.player.shove_give = give;
        if let Some((Shove::Push { side }, id)) = shove {
            // The box moves first; the character follows into the room it made.
            let amount = side as f32 * self.tuning.push_speed * give * DT;
            terrain.shove(id, amount, self.player.rect());
        }
        let before = self.player.x;
        self.player
            .step(&*terrain, intent, &self.tuning, DT, &mut self.events);
        if let Some((Shove::Pull { .. }, id)) = shove {
            // The box follows the character by as many units as it moved.
            let dx = self.player.x - before;
            terrain.shove(id, dx as f32, self.player.rect());
        }
        let body = self.player.rect();
        if self.player.dashes < self.tuning.dashes && terrain.refill(body) {
            self.player.dashes = self.tuning.dashes;
            self.events.push(Event::Recharged { crystal: true });
        }
        terrain.update(DT, body);
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
        self.effort = 0.0;
        self.last_shove = None;
        self.events.push(Event::Respawned);
    }

    /// The box the character strains against but which hasn't budged yet, if any (by the
    /// terrain's id): it may rumble.
    pub fn straining(&self) -> Option<usize> {
        let (_, i) = self.last_shove?;
        (self.effort > 0.0 && self.effort < self.tuning.push_breakaway).then_some(i)
    }

    /// The character's hold on a box this step, and which box. Standing beside a box at
    /// rest, walking into it pushes; holding grab facing it holds it, and walking away
    /// pulls.
    fn shove(&self, terrain: &dyn Terrain, i: &Intent) -> Option<(Shove, usize)> {
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
            // Within 2 units: a box that just moved a unit ahead still counts.
            let Some(b) = (1..=2).find_map(|d| terrain.pushable(p.rect().shifted(side * d, 0)))
            else {
                continue;
            };
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
