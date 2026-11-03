//! [`Stage`]: the demo's terrain: the library's [`Level`] with what the demo builds on top
//! of it, boxes to push and pull ([`PushPull`]) and a lift. It names the level as its
//! [`Terrain::base`], so platforms, refills and falling out are the level's, and answers
//! the rest itself.

pub mod block;
pub mod grip;
pub mod lift;
pub mod push_pull;
pub mod shove;

pub use block::Block;
pub use grip::Grip;
pub use lift::Lift;
pub use push_pull::PushPull;
pub use shove::Shove;

use crate::sim::level::DEMO;
use crate::sim::{Level, Rect, TILE, Terrain};

/// The level, its boxes and its lifts.
#[derive(Clone, Debug, PartialEq)]
pub struct Stage {
    /// The tiles, refills and spawn.
    pub level: Level,
    /// The pushable boxes.
    pub blocks: Vec<Block>,
    /// The moving platforms.
    pub lifts: Vec<Lift>,
    /// The hero's hold on a box.
    pub grip: Grip,
}

impl Stage {
    /// Parse tile rows as [`Level::parse`] does, with `B` for a box standing on the
    /// bottom of its tile.
    pub fn parse(rows: &[&str]) -> Self {
        let blocks = rows
            .iter()
            .enumerate()
            .flat_map(|(y, row)| {
                row.chars()
                    .enumerate()
                    .filter(|&(_, c)| c == 'B')
                    .map(move |(x, _)| Block::at((x as i32, y as i32)))
            })
            .collect();
        Self {
            level: Level::parse(rows),
            blocks,
            lifts: Vec::new(),
            grip: Grip::default(),
        }
    }

    /// The demo level, its two boxes, and a lift between the chimneys that goes from just
    /// over the floor up to the narrow chimney's lid.
    pub fn demo() -> Self {
        let mut s = Self::parse(&DEMO);
        s.lifts.push(Lift::new(
            (19 * TILE, 22 * TILE),
            (19 * TILE, 8 * TILE),
            (3 * TILE, 6),
            32.0,
            1.0,
        ));
        s
    }

    /// Whether `r` overlaps the level's solid tiles, a lift, or a box other than `skip`.
    fn solid_except(&self, r: Rect, skip: Option<usize>) -> bool {
        self.level.solid(r)
            || self.lifts.iter().any(|l| l.rect.intersects(r))
            || (self.blocks.iter().enumerate())
                .any(|(i, b)| Some(i) != skip && b.rect().intersects(r))
    }

    /// The first box at rest that overlaps `r`, by index.
    pub fn pushable(&self, r: Rect) -> Option<usize> {
        self.blocks
            .iter()
            .position(|b| b.grounded && b.rect().intersects(r))
    }

    /// Slide box `id` sideways by `dx` units, stopping at anything solid and at the
    /// hero's body `hero`. Returns the whole units it moved.
    pub fn shove(&mut self, id: usize, dx: f32, hero: Rect) -> i32 {
        // A copy moves while the stage still holds the old one, which it skips.
        let mut b = self.blocks[id];
        let moved = b.shove(dx, |r| self.solid_except(r, Some(id)) || r.intersects(hero));
        self.blocks[id] = b;
        moved
    }
}

impl Terrain for Stage {
    fn solid(&self, r: Rect) -> bool {
        self.solid_except(r, None)
    }

    // The level's refills come back by themselves: its own tick runs after this one.
    fn tick(&mut self, dt: f32, bodies: &[Rect]) {
        for l in &mut self.lifts {
            l.update(dt, bodies);
        }
        for i in 0..self.blocks.len() {
            let mut b = self.blocks[i];
            b.fall(dt, |r| {
                let under = r.shifted(0, 1);
                self.solid_except(under, Some(i))
                    || bodies.iter().any(|b| under.intersects(*b))
                    || self.level.platform(r)
            });
            self.blocks[i] = b;
        }
    }

    fn carry(&self, r: Rect) -> (f32, f32) {
        self.lifts
            .iter()
            .find_map(|l| l.carry(r))
            .unwrap_or((0.0, 0.0))
    }

    game_toolkit_platformer::terrain_layer!(level);
}

#[cfg(test)]
mod tests;
