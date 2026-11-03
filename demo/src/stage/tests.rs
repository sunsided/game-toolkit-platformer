//! Stage tests: boxes pushed, pulled and fallen, through the library's ability hooks; the
//! lift carrying the hero.

use super::*;
use crate::sim::{Event, Intent, Mode, Tuning, World, player};

/// A character on a stage, stepped together, pushing and pulling with [`PushPull`].
#[derive(Clone)]
struct Sim {
    /// The character.
    world: World,
    /// Its terrain.
    stage: Stage,
}

impl Sim {
    /// A character on `stage`'s spawn tile.
    fn new(stage: Stage) -> Self {
        let mut world = World::on(&stage.level, Tuning::default());
        world.abilities.push(Box::new(PushPull::default()));
        Self { world, stage }
    }

    /// Step `n` times with `i`, collecting every event.
    fn run(&mut self, i: Intent, n: usize) -> Vec<Event> {
        let mut all = Vec::new();
        for _ in 0..n {
            self.world.step(&mut self.stage, &i);
            all.extend(self.world.events.iter().copied());
        }
        all
    }

    /// The hero's hold on a box.
    fn shove(&self) -> Option<Shove> {
        self.stage.grip.shove.map(|(s, _)| s)
    }
}

/// A closed room 24 tiles wide and 16 high, the spawn on the floor at the left; `extra`
/// rows (tile row, column, string) overwrite parts of it.
fn room(extra: &[(usize, usize, &str)]) -> Stage {
    let mut rows: Vec<Vec<u8>> = (0..16)
        .map(|y| {
            (0..24)
                .map(|x| {
                    if x == 0 || x == 23 || y == 15 {
                        b'#'
                    } else {
                        b'.'
                    }
                })
                .collect()
        })
        .collect();
    rows[14][2] = b'P';
    for &(y, x, s) in extra {
        rows[y][x..x + s.len()].copy_from_slice(s.as_bytes());
    }
    let strs: Vec<String> = rows
        .into_iter()
        .map(|r| String::from_utf8(r).unwrap())
        .collect();
    let refs: Vec<&str> = strs.iter().map(String::as_str).collect();
    Stage::parse(&refs)
}

/// Walking right.
const RIGHT: Intent = Intent {
    move_x: 1.0,
    move_y: 0.0,
    jump_pressed: false,
    jump_held: false,
    dash_pressed: false,
    grab: false,
    dive_pressed: false,
    sprint: false,
    super_dash: false,
    custom: 0,
    custom_pressed: 0,
};

/// A room with a block on the floor at tile x 6, the hero standing left of it.
fn with_block() -> Sim {
    let mut w = Sim::new(room(&[(14, 6, "B")]));
    w.world.player.x = 6 * TILE - 2 - player::W - 4;
    w.run(Intent::default(), 3);
    w
}

#[test]
fn walking_into_a_block_pushes_it_slowly() {
    let mut w = with_block();
    let start = w.stage.blocks[0].x;
    w.run(RIGHT, 120);
    let moved = w.stage.blocks[0].x - start;
    assert!(moved > 20, "pushed {moved}");
    assert_eq!(w.shove(), Some(Shove::Push { side: 1 }));
    assert!(w.world.player.vx <= PushPull::default().push_speed + 0.5);
    assert!(moved < 60, "slower than running: {moved}");
}

#[test]
fn grabbing_a_block_holds_it_and_walking_away_pulls_it() {
    let mut w = with_block();
    w.run(RIGHT, 20);
    let start = w.stage.blocks[0].x;
    let grab = Intent {
        grab: true,
        ..Intent::default()
    };
    w.run(grab, 5);
    assert_eq!(w.shove(), Some(Shove::Hold { side: 1 }));
    assert_eq!(
        w.world.player.mode,
        Mode::Normal,
        "grab at a block doesn't climb it"
    );
    w.run(
        Intent {
            move_x: -1.0,
            ..grab
        },
        120,
    );
    assert_eq!(w.shove(), Some(Shove::Pull { side: 1 }));
    assert_eq!(w.world.player.facing, 1, "still facing the block");
    assert!(
        w.stage.blocks[0].x < start - 12,
        "pulled from {start} to {}",
        w.stage.blocks[0].x
    );
    assert!(
        w.world.player.wall(&w.stage, 1),
        "the block stays at the hand"
    );
}

#[test]
fn blocks_fall_off_ledges_and_can_be_stood_on() {
    // A block on a shelf edge; push it off, then jump onto it.
    let mut w = Sim::new(room(&[(10, 1, "######"), (9, 5, "B")]));
    w.world.player.x = 2 * TILE;
    w.world.player.y = 10 * TILE - player::H;
    w.run(RIGHT, 200);
    assert!(w.stage.blocks[0].grounded);
    assert_eq!(w.stage.blocks[0].y, 15 * TILE - block::SIZE, "on the floor");
    let mut w = Sim::new(room(&[(14, 4, "B")]));
    w.run(
        Intent {
            jump_pressed: true,
            jump_held: true,
            ..Intent::default()
        },
        1,
    );
    w.run(
        Intent {
            jump_held: true,
            ..Intent::default()
        },
        30,
    );
    w.run(RIGHT, 18);
    w.run(Intent::default(), 90);
    assert_eq!(
        w.world.player.y + player::H,
        w.stage.blocks[0].y,
        "standing on the block"
    );
}

#[test]
fn blocks_stick_before_they_budge_and_rumble_meanwhile() {
    let mut w = with_block();
    // Walk up to it, until the first step of pushing.
    while !matches!(w.shove(), Some(Shove::Push { .. })) {
        w.run(RIGHT, 1);
    }
    let pp = PushPull::default();
    assert_eq!(w.stage.grip.straining(pp.breakaway), Some(0));
    let start = w.stage.blocks[0].x;
    let mut pushed = 0;
    for _ in 0..24 {
        // Under the 0.25 s breakaway: strained, not moved.
        w.run(RIGHT, 1);
        pushed = w.stage.blocks[0].x - start;
    }
    let g = w.stage.grip;
    assert!(g.effort > 0.0 && g.effort < pp.breakaway + 0.02);
    assert!(pushed <= 1, "budged early: {pushed}");
    w.run(RIGHT, 90);
    assert!(w.stage.blocks[0].x - start > 15, "then it slides");
    assert_eq!(w.stage.grip.straining(pp.breakaway), None, "it budged");
    // Letting go resets the effort: it sticks again.
    w.run(Intent::default(), 2);
    assert_eq!(w.stage.grip.effort, 0.0);
}

#[test]
fn a_cloned_stage_with_boxes_plays_out_identically() {
    let mut a = Sim::new(Stage::demo());
    // Up to the box between the chimneys, pushing it, then pulling it back.
    let script = |k: usize| Intent {
        move_x: if k % 500 < 350 { 1.0 } else { -1.0 },
        grab: k % 500 >= 350,
        ..Intent::default()
    };
    for k in 0..100 {
        a.run(script(k), 1);
    }
    let mut b = a.clone();
    for k in 100..1200 {
        let ea = a.run(script(k), 1);
        let eb = b.run(script(k), 1);
        assert_eq!(ea, eb, "step {k}");
    }
    assert_eq!(a.world.player, b.world.player);
    assert_eq!(a.stage, b.stage);
    assert_ne!(a.stage.blocks, Stage::demo().blocks, "a box moved");
}

/// A room with a lift 3 tiles wide from `a` to `b` (tiles), the hero standing on it.
fn on_lift(extra: &[(usize, usize, &str)], a: (i32, i32), b: (i32, i32)) -> Sim {
    let mut stage = room(extra);
    let t = |(x, y): (i32, i32)| (x * TILE, y * TILE);
    stage
        .lifts
        .push(Lift::new(t(a), t(b), (3 * TILE, 6), 40.0, 0.0));
    let mut w = Sim::new(stage);
    let l = w.stage.lifts[0].rect;
    w.world.player.teleport(l.x + 6, l.y - player::H);
    w
}

#[test]
fn the_lift_carries_the_hero_up_and_across() {
    let mut w = on_lift(&[], (4, 13), (10, 6));
    let ev = w.run(Intent::default(), 120);
    let (l, r) = (w.stage.lifts[0].rect, w.world.player.rect());
    assert!(l.y < 13 * TILE - 20, "the lift rose: {}", l.y);
    assert_eq!(r.bottom(), l.y, "on top of it");
    assert_eq!(r.x - l.x, 6, "where she stood on it");
    assert!(
        !ev.iter()
            .any(|e| matches!(e, Event::LeftGround | Event::Landed { .. })),
        "{ev:?}"
    );
}

#[test]
fn jumping_off_the_rising_lift_gets_its_lift() {
    let peak = |ride: bool| {
        let (a, b) = if ride {
            ((4, 13), (4, 2))
        } else {
            ((4, 13), (4, 13))
        };
        let mut w = on_lift(&[], a, b);
        w.run(Intent::default(), 60);
        let start = w.world.player.y;
        w.run(
            Intent {
                jump_pressed: true,
                jump_held: true,
                ..Intent::default()
            },
            1,
        );
        let mut top = start;
        for _ in 0..60 {
            w.run(
                Intent {
                    jump_held: true,
                    ..Intent::default()
                },
                1,
            );
            top = top.min(w.world.player.y);
        }
        start - top
    };
    let (still, rising) = (peak(false), peak(true));
    assert!(rising > still + 4, "{rising} over {still}");
}

#[test]
fn a_lift_closing_on_a_ceiling_squishes() {
    // A ceiling 4 tiles over the lift's top end.
    let mut w = on_lift(&[(1, 1, "######################")], (4, 13), (4, 3));
    let ev = w.run(Intent::default(), 600);
    assert!(ev.contains(&Event::Squished), "{ev:?}");
}

#[test]
fn the_lift_stops_short_of_the_hero_under_it() {
    let mut stage = room(&[]);
    // Coming down onto her: from 4 tiles over the floor to the floor.
    stage.lifts.push(Lift::new(
        (2 * TILE, 10 * TILE),
        (2 * TILE, 15 * TILE),
        (24, 6),
        40.0,
        0.0,
    ));
    let mut w = Sim::new(stage);
    let ev = w.run(Intent::default(), 240);
    let (l, r) = (w.stage.lifts[0].rect, w.world.player.rect());
    assert_eq!(l.bottom(), r.y, "resting on her head, not in her");
    assert!(!ev.contains(&Event::Squished), "{ev:?}");
}
