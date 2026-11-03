//! [`Party`]: several characters on one terrain, on one clock.
//!
//! Separate [`Platformer`]s on the same terrain would each update it (so refills come back
//! and platforms move too fast), keep their own clocks, and pass through each other. A party
//! steps all of them together: each moves, then the terrain updates once with every body,
//! then moving ground (and the characters under them) carry them along.
//!
//! With [`Party::solid`] on (the default), the members block each other and stand on each
//! other's heads, and a character standing on another rides along when it moves. The
//! terrain's update gets every body, so what falls or moves can stop at any of them, and
//! abilities see the other members' bodies in [`Step::others`](crate::sim::Step::others).
//!
//! ```
//! use game_toolkit_platformer::prelude::*;
//!
//! let mut level = Level::demo();
//! let a = Platformer::on(&level, Preset::Hybrid);
//! let b = Platformer::builder().spawn(level.start()).build().unwrap();
//! let mut party = Party::new(vec![a, b]);
//! // One intent per member, in order (from your own input: one pad each, say).
//! let intents = [Intent { move_x: 1.0, ..Intent::default() }, Intent::default()];
//! party.advance(&intents, &mut level, 1.0 / 60.0);
//! for hero in &mut party.members {
//!     let _ = (hero.position(), hero.sprite(), hero.drain_events());
//! }
//! ```

use crate::Platformer;
use crate::platformer::{Scaled, steps};
use crate::sim::{DT, Intent, Rect, Terrain};

/// Several characters on one terrain, stepped together.
pub struct Party {
    /// The characters, in the order their intents come.
    pub members: Vec<Platformer>,
    /// The members block each other, stand on each other and ride each other.
    pub solid: bool,
    /// Frame time not yet stepped.
    acc: f32,
}

impl Party {
    /// A party of `members`, solid to each other.
    pub fn new(members: Vec<Platformer>) -> Self {
        Self {
            members,
            solid: true,
            acc: 0.0,
        }
    }

    /// Advance by `dt` seconds of frame time in `terrain`, member `k` with `intents[k]`
    /// (members without one stand still), in fixed steps of [`DT`]. Presses are kept
    /// until the next step, as with [`Platformer::advance`]. Returns the steps taken.
    pub fn advance(&mut self, intents: &[Intent], terrain: &mut dyn Terrain, dt: f32) -> u32 {
        let intent = |k: usize| intents.get(k).copied().unwrap_or_default();
        for (k, m) in self.members.iter_mut().enumerate() {
            m.begin_frame();
            m.hold_presses(&intent(k));
        }
        let n = steps(&mut self.acc, dt);
        for _ in 0..n {
            let step: Vec<Intent> = (self.members.iter_mut().enumerate())
                .map(|(k, m)| m.with_presses(intent(k)))
                .collect();
            self.step_once(&step, terrain);
        }
        for m in &mut self.members {
            m.set_clock(self.acc);
        }
        n
    }

    /// Exactly one fixed step of [`DT`] for everyone (for your own clock, replays, tests).
    /// The members share the first one's [`Platformer::scale`].
    pub fn step(&mut self, intents: &[Intent], terrain: &mut dyn Terrain) {
        for m in &mut self.members {
            m.begin_frame();
        }
        self.step_once(intents, terrain);
    }

    /// One fixed step for everyone, within a frame.
    fn step_once(&mut self, intents: &[Intent], terrain: &mut dyn Terrain) {
        let k = self.members.first().map_or(1, Platformer::scale);
        debug_assert!(
            self.members.iter().all(|m| m.scale() == k),
            "party members on one terrain share a scale"
        );
        if k == 1 {
            self.step_in(intents, terrain);
        } else {
            self.step_in(intents, &mut Scaled { inner: terrain, k });
        }
    }

    /// [`Party::step`] in a terrain that speaks units.
    fn step_in(&mut self, intents: &[Intent], terrain: &mut dyn Terrain) {
        let n = self.members.len();
        let before: Vec<Rect> = self.members.iter().map(|m| m.world.player.rect()).collect();
        // Who stands on whom, as the step starts.
        let carrier: Vec<Option<usize>> = (0..n)
            .map(|i| {
                let feet = self.members[i].world.player.riding()?;
                let grounded = self.members[i].world.player.grounded;
                (self.solid && grounded)
                    .then(|| (0..n).find(|&j| j != i && feet.intersects(before[j])))
                    .flatten()
            })
            .collect();
        // A carrier isn't held back by its riders; they ride along after.
        let others = |i: usize, now: &[Rect]| -> Vec<Rect> {
            if !self.solid {
                return Vec::new();
            }
            (0..n)
                .filter(|&j| j != i && carrier[j] != Some(i))
                .map(|j| now[j])
                .collect()
        };
        // Riders move before their carriers: top to bottom.
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by_key(|&i| before[i].y);
        let mut now = before.clone();
        for &i in &order {
            let mut among = Among {
                terrain: &mut *terrain,
                others: others(i, &now),
            };
            let intent = intents.get(i).copied().unwrap_or_default();
            let everyone_else: Vec<Rect> = (0..n).filter(|&j| j != i).map(|j| now[j]).collect();
            self.members[i].begin_step();
            self.members[i]
                .world
                .act_among(&mut among, &intent, &everyone_else);
            now[i] = self.members[i].world.player.rect();
        }
        terrain.update(DT, &now);
        // Carriers settle first, so their riders follow where they ended up.
        for &i in order.iter().rev() {
            let extra = carrier[i].map(|j| {
                let (b, a) = (before[j], now[j]);
                ((a.x - b.x) as f32, (a.y - b.y) as f32)
            });
            let mut among = Among {
                terrain: &mut *terrain,
                others: others(i, &now),
            };
            self.members[i].world.settle(&mut among, extra);
            now[i] = self.members[i].world.player.rect();
        }
        for m in &mut self.members {
            m.end_step();
        }
    }
}

/// A terrain with the other characters in it, as solids.
struct Among<'a> {
    /// The shared terrain.
    terrain: &'a mut dyn Terrain,
    /// The other characters' bodies that count as solid.
    others: Vec<Rect>,
}

impl Among<'_> {
    /// Whether `r` overlaps another character.
    fn hits_other(&self, r: Rect) -> bool {
        self.others.iter().any(|o| o.intersects(r))
    }
}

impl Terrain for Among<'_> {
    fn solid(&self, r: Rect) -> bool {
        self.terrain.solid(r) || self.hits_other(r)
    }

    fn base(&self) -> Option<&dyn Terrain> {
        Some(&*self.terrain)
    }

    fn base_mut(&mut self) -> Option<&mut dyn Terrain> {
        Some(&mut *self.terrain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{Event, Level, Preset, Spot};

    /// A flat floor at y = 100 that counts its updates and the bodies it saw.
    #[derive(Default)]
    struct Floor {
        /// Updates so far.
        updates: u32,
        /// Bodies in the last update.
        bodies: usize,
    }

    impl Terrain for Floor {
        fn solid(&self, r: Rect) -> bool {
            r.bottom() > 100
        }
        fn tick(&mut self, _dt: f32, bodies: &[Rect]) {
            self.updates += 1;
            self.bodies = bodies.len();
        }
        crate::terrain_layer!();
    }

    /// A character with its hitbox's top-left at `x, y`.
    fn at(x: i32, y: i32) -> Platformer {
        Platformer::at(Spot::TopLeft(x, y), Preset::Hybrid)
    }

    /// `n` frames of 1/60 s with `intents`.
    fn frames(p: &mut Party, intents: &[Intent], floor: &mut Floor, n: usize) {
        for _ in 0..n {
            p.advance(intents, floor, 1.0 / 60.0);
        }
    }

    #[test]
    fn the_terrain_updates_once_per_step_with_every_body() {
        let mut floor = Floor::default();
        let mut p = Party::new(vec![at(20, 88), at(60, 88), at(100, 88)]);
        let steps = p.advance(&[], &mut floor, 1.0 / 60.0);
        assert_eq!(steps, 2);
        assert_eq!(floor.updates, 2);
        assert_eq!(floor.bodies, 3);
    }

    #[test]
    fn members_stand_on_and_ride_each_other() {
        let mut floor = Floor::default();
        // B drops onto A's head.
        let mut p = Party::new(vec![at(20, 88), at(20, 40)]);
        frames(&mut p, &[], &mut floor, 60);
        let (a, b) = (p.members[0].hitbox(), p.members[1].hitbox());
        assert_eq!(b.bottom(), a.y, "B stands on A");
        assert!(p.members[1].world.player.grounded);

        // A walks right; B rides along without moving its own legs.
        let walk = [Intent {
            move_x: 1.0,
            ..Intent::default()
        }];
        frames(&mut p, &walk, &mut floor, 30);
        let (a2, b2) = (p.members[0].hitbox(), p.members[1].hitbox());
        assert!(a2.x > a.x + 20, "A walked: {} -> {}", a.x, a2.x);
        assert_eq!(b2.x - a2.x, b.x - a.x, "B kept its place on A's head");
        assert_eq!(b2.bottom(), a2.y);

        // A jumps; B goes up with it and isn't squished.
        let jump = [Intent {
            jump_pressed: true,
            jump_held: true,
            ..Intent::default()
        }];
        let mut events = Vec::new();
        p.advance(&jump, &mut floor, 1.0 / 60.0);
        for _ in 0..10 {
            p.advance(
                &[Intent {
                    jump_held: true,
                    ..Intent::default()
                }],
                &mut floor,
                1.0 / 60.0,
            );
            events.extend(p.members[1].drain_events());
        }
        assert!(p.members[1].hitbox().y < b2.y - 10, "carried up");
        assert!(!events.contains(&Event::Squished), "{events:?}");
    }

    #[test]
    fn members_block_each_other_unless_not_solid() {
        let mut floor = Floor::default();
        let right = [Intent {
            move_x: 1.0,
            ..Intent::default()
        }];
        let mut p = Party::new(vec![at(20, 88), at(60, 88)]);
        frames(&mut p, &right, &mut floor, 60);
        assert_eq!(p.members[0].hitbox().right(), 60, "stopped by B");

        let mut p = Party::new(vec![at(20, 88), at(60, 88)]);
        p.solid = false;
        frames(&mut p, &right, &mut floor, 60);
        assert!(p.members[0].hitbox().x > 60, "walked through B");
    }

    #[test]
    fn abilities_reach_the_shared_terrain_through_the_others() {
        use crate::sim::{Ability, Step, Tile};
        /// Custom button 0 digs out the tile under the feet.
        #[derive(Clone, Debug)]
        struct Dig;
        impl Ability for Dig {
            fn before(&mut self, s: &mut Step<'_>) {
                if s.intent.pressed(0) {
                    let r = s.player.rect();
                    let level = s.terrain.find_mut::<Level>().expect("behind the party");
                    level.set((r.x + r.w / 2) / 8, r.bottom() / 8, Tile::Empty);
                }
            }
        }
        let mut level = Level::parse(&["........", "........", "........", "########"]);
        let a = Platformer::builder()
            .ability(Dig)
            .spawn(Spot::Feet(12, 24))
            .build()
            .unwrap();
        let b = Platformer::builder()
            .spawn(Spot::Feet(44, 24))
            .build()
            .unwrap();
        let mut p = Party::new(vec![a, b]);
        let dig = Intent {
            custom_pressed: 1,
            ..Intent::default()
        };
        p.step(&[dig], &mut level);
        assert_eq!(level.tile(1, 3), Tile::Empty, "dug out");
        assert_eq!(level.tile(5, 3), Tile::Solid);
    }

    #[test]
    fn abilities_see_the_other_members() {
        use crate::sim::{Ability, CustomId, Step};
        /// Reports how many others it saw and how far the nearest is.
        #[derive(Clone, Debug)]
        struct Look;
        /// Look's event.
        const SAW: CustomId = CustomId::new("tests::saw");
        impl Ability for Look {
            fn after(&mut self, s: &mut Step<'_>) {
                let me = s.player.rect();
                let near = s.others.iter().map(|o| (o.x - me.x).abs()).min();
                let n = s.others.len() as f32;
                let d = near.map_or(-1.0, |d| d as f32);
                s.events.push(Event::custom(SAW).with([n, d, 0.0, 0.0]));
            }
        }
        let a = Platformer::builder()
            .ability(Look)
            .spawn(Spot::TopLeft(20, 88))
            .build()
            .unwrap();
        let mut p = Party::new(vec![a, at(60, 88), at(100, 88)]);
        let mut floor = Floor::default();
        p.step(&[], &mut floor);
        let saw: Vec<_> = p.members[0]
            .drain_events()
            .filter_map(|e| e.custom_data(SAW))
            .collect();
        assert_eq!(saw, [[2.0, 40.0, 0.0, 0.0]]);
        assert_eq!(p.members[0].hitboxes().len(), 1);

        // Alone, nobody.
        let mut solo = Platformer::builder().ability(Look).build().unwrap();
        solo.step(&Intent::default(), &mut floor);
        let saw: Vec<_> = solo
            .drain_events()
            .filter_map(|e| e.custom_data(SAW))
            .collect();
        assert_eq!(saw, [[0.0, -1.0, 0.0, 0.0]]);
    }
}
