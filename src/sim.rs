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
pub mod contacts;
pub mod crystal;
pub mod custom_id;
pub mod custom_tile;
pub mod event;
pub mod hold;
pub mod intent;
pub mod level;
pub mod moves;
pub mod player;
pub mod rect;
pub mod spot;
pub mod surface;
pub mod terrain;
pub mod tuning;

pub use ability::{Ability, Step};
pub use contacts::Contacts;
pub use crystal::Crystal;
pub use custom_id::CustomId;
pub use custom_tile::CustomTile;
pub use event::{Event, JumpKind};
pub use hold::Hold;
pub use intent::Intent;
pub use level::{Level, TILE, Tile};
pub use moves::Moves;
pub use player::{Mode, Player};
pub use rect::Rect;
pub use spot::Spot;
pub use surface::Surface;
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
    /// Where it starts, and comes back to after falling out (units).
    pub spawn: Spot,
    /// Put it back at [`World::spawn`] when it falls out (an [`Event::FellOut`] is
    /// reported either way).
    pub respawn: bool,
    /// What it rode when it finished moving this step (see [`Player::riding`]), for
    /// [`World::settle`].
    riding: Option<Rect>,
    /// The conveyor speed under its feet when it finished moving this step, units/s.
    belt: f32,
    /// Steps taken so far.
    pub tick: u64,
    /// What happened in the last step.
    pub events: Vec<Event>,
    /// Moves of your own, run in every step in this order (see [`Ability`]).
    pub abilities: Vec<Box<dyn Ability>>,
}

impl World {
    /// A character standing at `spawn` (units).
    pub fn new(tuning: Tuning, spawn: Spot) -> Self {
        Self {
            player: Player::at(spawn.top_left(tuning.size), &tuning),
            tuning,
            spawn,
            respawn: true,
            riding: None,
            belt: 0.0,
            tick: 0,
            events: Vec::new(),
            abilities: Vec::new(),
        }
    }

    /// A character at `level`'s spawn tile.
    pub fn on(level: &Level, tuning: Tuning) -> Self {
        Self::new(tuning, level.start())
    }

    /// The first ability of type `T`, to read its state (a charge level, a phase): for a
    /// driver of its animation, or the game.
    pub fn ability<T: Ability + 'static>(&self) -> Option<&T> {
        self.abilities
            .iter()
            .find_map(|a| a.as_any().downcast_ref::<T>())
    }

    /// [`World::ability`], to change.
    pub fn ability_mut<T: Ability + 'static>(&mut self) -> Option<&mut T> {
        self.abilities
            .iter_mut()
            .find_map(|a| a.as_any_mut().downcast_mut::<T>())
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
    /// refills, bouncy ground), the terrain doesn't update yet.
    pub fn act(&mut self, terrain: &mut dyn Terrain, intent: &Intent) {
        self.act_among(terrain, intent, &[]);
    }

    /// [`World::act`] beside other characters, whose bodies its abilities see in
    /// [`Step::others`] (a [`crate::Party`] passes them).
    pub fn act_among(&mut self, terrain: &mut dyn Terrain, intent: &Intent, others: &[Rect]) {
        self.events.clear();
        let mut moves = Moves::ALL;
        self.abilities(terrain, intent, &mut moves, others, |a, s| a.before(s));
        self.player.step(
            &*terrain,
            intent,
            &moves,
            &self.tuning,
            DT,
            &mut self.events,
        );
        self.bounce(&*terrain);
        self.abilities(terrain, intent, &mut moves, others, |a, s| a.after(s));
        let body = self.player.rect();
        if self.player.dashes < self.tuning.dashes && terrain.refill(body) {
            self.player.dashes = self.tuning.dashes;
            self.events.push(Event::Recharged { crystal: true });
        }
        self.riding = self.player.riding();
        self.belt = if self.player.grounded {
            terrain.surface(self.player.feet()).belt
        } else {
            0.0
        };
    }

    /// A landing on bouncy ground ([`Surface::bounce`]) this step bounces back up.
    fn bounce(&mut self, terrain: &dyn Terrain) {
        let impact = self.events.iter().find_map(|e| match e {
            Event::Landed { impact, .. } => Some(*impact),
            _ => None,
        });
        let Some(impact) = impact else { return };
        let bounce = terrain.surface(self.player.feet()).bounce;
        if bounce > 0.0 && impact > Surface::BOUNCE_MIN {
            self.player.launch(self.player.vx, -impact * bounce);
        }
    }

    /// The last part of a step, after the terrain's update: moving ground (and conveyor
    /// belts) carry the character (`extra` adds to how far, for riding on other
    /// characters), and it may be squished or fall out.
    pub fn settle(&mut self, terrain: &mut dyn Terrain, extra: Option<(f32, f32)>) {
        let belt = (self.belt != 0.0).then_some((self.belt * DT, 0.0));
        let carry = [self.riding.map(|r| terrain.carry(r)), extra, belt]
            .into_iter()
            .flatten()
            .reduce(|(x, y), (ex, ey)| (x + ex, y + ey));
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

    /// Start over at `at` (units): a fresh body, standing, with everything refilled, and
    /// an [`Event::Respawned`] in [`World::events`]. For hazards and checkpoints;
    /// [`World::spawn`] stays as it is.
    pub fn respawn_at(&mut self, at: Spot) {
        self.player = Player::at(at.top_left(self.tuning.size), &self.tuning);
        self.riding = None;
        self.belt = 0.0;
        self.events.push(Event::Respawned);
    }

    /// Run `hook` of every ability.
    fn abilities(
        &mut self,
        terrain: &mut dyn Terrain,
        intent: &Intent,
        moves: &mut Moves,
        others: &[Rect],
        hook: fn(&mut dyn Ability, &mut Step<'_>),
    ) {
        let mut s = Step {
            player: &mut self.player,
            intent,
            terrain,
            tuning: &self.tuning,
            moves,
            events: &mut self.events,
            others,
            dt: DT,
        };
        for a in &mut self.abilities {
            hook(a.as_mut(), &mut s);
        }
    }
}

#[cfg(test)]
mod tests;
