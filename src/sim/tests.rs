//! Simulation tests: determinism and movement rules, driven by scripted intents.

use super::*;

/// A character in a level, stepped together: the tests' shorthand for a `World` plus its
/// `Level` as the terrain.
#[derive(Clone)]
struct Sim {
    /// The character.
    world: World,
    /// Its terrain.
    level: Level,
}

impl Sim {
    /// A character on `level`'s spawn tile with `tuning`.
    fn new(level: Level, tuning: Tuning) -> Self {
        Self {
            world: World::on(&level, tuning),
            level,
        }
    }

    /// One step with `i`.
    fn step(&mut self, i: &Intent) {
        self.world.step(&mut self.level, i);
    }
}

impl std::ops::Deref for Sim {
    type Target = World;
    fn deref(&self) -> &World {
        &self.world
    }
}

impl std::ops::DerefMut for Sim {
    fn deref_mut(&mut self) -> &mut World {
        &mut self.world
    }
}

/// A closed room 24 tiles wide and 16 high, the spawn on the floor at the left; `extra`
/// rows (tile row, string) overwrite parts of it.
fn room(extra: &[(usize, usize, &str)]) -> Level {
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
    Level::parse(&refs)
}

/// A world in `level` with default tuning.
fn world(level: Level) -> Sim {
    Sim::new(level, Tuning::default())
}

/// Step `n` times with `i`, collecting every event.
fn run(w: &mut Sim, i: Intent, n: usize) -> Vec<Event> {
    let mut all = Vec::new();
    for _ in 0..n {
        w.step(&i);
        all.extend(w.events.iter().copied());
    }
    all
}

/// Press jump once (holding), then hold it for `hold` steps, then release for `rest`.
fn jump(w: &mut Sim, hold: usize, rest: usize) -> Vec<Event> {
    let press = Intent {
        jump_pressed: true,
        jump_held: true,
        ..Intent::default()
    };
    let mut ev = run(w, press, 1);
    ev.extend(run(
        w,
        Intent {
            jump_held: true,
            ..Intent::default()
        },
        hold,
    ));
    ev.extend(run(w, Intent::default(), rest));
    ev
}

/// The jumps among `ev`.
fn jumps(ev: &[Event]) -> Vec<(JumpKind, bool)> {
    ev.iter()
        .filter_map(|e| match *e {
            Event::Jumped { kind, buffered } => Some((kind, buffered)),
            _ => None,
        })
        .collect()
}

/// The hero's highest point (smallest y) over the events of a run, by stepping it.
fn peak(w: &mut Sim, hold: usize) -> i32 {
    let start = w.player.y;
    let mut top = start;
    let press = Intent {
        jump_pressed: true,
        jump_held: true,
        ..Intent::default()
    };
    w.step(&press);
    for k in 0..240 {
        let held = k < hold;
        w.step(&Intent {
            jump_held: held,
            ..Intent::default()
        });
        top = top.min(w.player.y);
    }
    start - top
}

#[test]
fn a_cloned_world_plays_out_identically() {
    let mut a = world(Level::demo());
    let script: Vec<Intent> = (0..600)
        .map(|k| Intent {
            move_x: if k % 200 < 120 { 1.0 } else { -1.0 },
            jump_pressed: k % 70 == 0,
            jump_held: k % 70 < 30,
            dash_pressed: k % 150 == 40,
            move_y: if k % 150 == 40 { -1.0 } else { 0.0 },
            ..Intent::default()
        })
        .collect();
    run(&mut a, Intent::default(), 10);
    let mut b = a.clone();
    for i in &script {
        a.step(i);
        b.step(i);
        assert_eq!(a.world.events, b.events);
    }
    assert_eq!(a.player, b.player);
}

#[test]
fn holding_jump_goes_higher_than_tapping() {
    let tap = peak(&mut world(room(&[])), 1);
    let hold = peak(&mut world(room(&[])), 60);
    assert!(hold > tap + 8, "tap {tap}, hold {hold}");
}

#[test]
fn holding_through_the_apex_hangs_longer() {
    /// Steps spent with |vy| under the apex threshold.
    fn hang(hold: bool) -> usize {
        let mut w = world(room(&[]));
        let mut n = 0;
        w.step(&Intent {
            jump_pressed: true,
            jump_held: true,
            ..Intent::default()
        });
        for k in 0..240 {
            // Both release the same early; one presses again over the apex.
            let held = k < 24 || (hold && w.player.vy.abs() < 40.0);
            w.step(&Intent {
                jump_held: held,
                ..Intent::default()
            });
            if !w.player.grounded && w.player.vy.abs() < 40.0 {
                n += 1;
            }
        }
        n
    }
    assert!(hang(true) > hang(false) + 5);
}

#[test]
fn a_press_just_before_landing_is_queued_into_a_ground_jump() {
    let mut w = world(room(&[]));
    jump(&mut w, 30, 0);
    // Fall until just above the floor, then press.
    while !(w.player.vy > 0.0 && w.player.y + player::H >= 15 * TILE - 5) {
        w.step(&Intent::default());
    }
    let ev = run(
        &mut w,
        Intent {
            jump_pressed: true,
            jump_held: true,
            ..Intent::default()
        },
        1,
    );
    let mut ev = ev;
    ev.extend(run(
        &mut w,
        Intent {
            jump_held: true,
            ..Intent::default()
        },
        12,
    ));
    assert_eq!(jumps(&ev), [(JumpKind::Ground, true)], "{ev:?}");
    assert_eq!(w.player.air_jumps, 1, "the air jump was not spent");
}

#[test]
fn coyote_time_allows_a_late_jump_off_a_ledge() {
    // A ledge at tile row 10 from x 1..8; walk right off it.
    let mut w = world(room(&[(10, 1, "########")]));
    w.player.x = 6 * TILE;
    w.player.y = 10 * TILE - player::H;
    run(&mut w, Intent::default(), 2);
    let right = Intent {
        move_x: 1.0,
        ..Intent::default()
    };
    let mut ev = Vec::new();
    while !ev.contains(&Event::LeftGround) {
        ev.extend(run(&mut w, right, 1));
    }
    run(&mut w, right, 4);
    let ev = run(
        &mut w,
        Intent {
            jump_pressed: true,
            jump_held: true,
            move_x: 1.0,
            ..Intent::default()
        },
        1,
    );
    assert_eq!(jumps(&ev), [(JumpKind::Coyote, false)]);
}

#[test]
fn one_air_jump_until_landing() {
    let mut w = world(room(&[]));
    let mut ev = jump(&mut w, 20, 0);
    ev.extend(jump(&mut w, 10, 0));
    ev.extend(jump(&mut w, 10, 0));
    let kinds: Vec<_> = jumps(&ev).into_iter().map(|(k, _)| k).collect();
    assert_eq!(kinds, [JumpKind::Ground, JumpKind::Double]);
}

/// A world with the hero standing on a shelf `rows` tiles above the floor.
fn on_shelf(rows: i32) -> Sim {
    let y = 15 - rows;
    let mut w = world(room(&[(y as usize, 1, "######")]));
    w.player.x = 4 * TILE;
    w.player.y = y * TILE - player::H;
    run(&mut w, Intent::default(), 2);
    w
}

/// Walk right off the shelf with `move_x` held after leaving it, until landed.
fn drop_off(w: &mut Sim, move_x: f32, extra: Intent) -> Vec<Event> {
    let mut ev = Vec::new();
    while !ev.contains(&Event::LeftGround) {
        ev.extend(run(
            w,
            Intent {
                move_x: 1.0,
                ..Intent::default()
            },
            1,
        ));
    }
    while !ev.iter().any(|e| matches!(e, Event::Landed { .. })) {
        ev.extend(run(w, Intent { move_x, ..extra }, 1));
    }
    ev.extend(run(
        w,
        Intent {
            move_x,
            ..Intent::default()
        },
        3,
    ));
    ev
}

#[test]
fn a_high_landing_while_moving_rolls() {
    let ev = drop_off(&mut on_shelf(8), 1.0, Intent::default());
    assert!(ev.contains(&Event::RollStarted), "{ev:?}");
}

#[test]
fn a_very_high_landing_standing_still_lands_hard() {
    let mut w = on_shelf(13);
    let ev = drop_off(&mut w, 0.0, Intent::default());
    assert!(ev.contains(&Event::HardLanded), "{ev:?}");
    // No control until it is over.
    let ev = run(
        &mut w,
        Intent {
            jump_pressed: true,
            jump_held: true,
            ..Intent::default()
        },
        1,
    );
    assert!(jumps(&ev).is_empty());
    assert!(run(&mut w, Intent::default(), 40).contains(&Event::Recovered));
}

#[test]
fn a_queued_jump_beats_the_roll() {
    let mut w = on_shelf(8);
    let mut ev = Vec::new();
    while !ev.contains(&Event::LeftGround) {
        ev.extend(run(
            &mut w,
            Intent {
                move_x: 1.0,
                ..Intent::default()
            },
            1,
        ));
    }
    while w.player.y + player::H < 15 * TILE - 5 {
        run(
            &mut w,
            Intent {
                move_x: 1.0,
                ..Intent::default()
            },
            1,
        );
    }
    let mut ev = run(
        &mut w,
        Intent {
            move_x: 1.0,
            jump_pressed: true,
            jump_held: true,
            ..Intent::default()
        },
        1,
    );
    ev.extend(run(
        &mut w,
        Intent {
            move_x: 1.0,
            jump_held: true,
            ..Intent::default()
        },
        10,
    ));
    assert!(!ev.contains(&Event::RollStarted), "{ev:?}");
    assert_eq!(jumps(&ev), [(JumpKind::Ground, true)]);
}

#[test]
fn rolls_cancel_into_jumps_that_keep_the_speed() {
    let mut w = on_shelf(8);
    drop_off(&mut w, 1.0, Intent::default());
    assert!(matches!(w.player.mode, Mode::Roll { .. }));
    // Pressed too early in the roll: it waits in the buffer until the roll can be cut.
    let mut ev = run(
        &mut w,
        Intent {
            move_x: 1.0,
            jump_pressed: true,
            jump_held: true,
            ..Intent::default()
        },
        1,
    );
    ev.extend(run(
        &mut w,
        Intent {
            move_x: 1.0,
            jump_held: true,
            ..Intent::default()
        },
        8,
    ));
    assert_eq!(jumps(&ev), [(JumpKind::Roll, true)]);
    assert!(
        w.player.vx > Tuning::default().run_speed + 30.0,
        "{}",
        w.player.vx
    );
}

#[test]
fn down_and_jump_drops_through_a_platform() {
    let mut w = world(room(&[(11, 1, "========")]));
    w.player.y = 11 * TILE - player::H;
    run(&mut w, Intent::default(), 2);
    assert!(w.player.grounded, "platforms hold");
    let ev = run(
        &mut w,
        Intent {
            move_y: 1.0,
            jump_pressed: true,
            jump_held: true,
            ..Intent::default()
        },
        1,
    );
    assert!(ev.contains(&Event::DroppedThrough));
    run(&mut w, Intent::default(), 60);
    assert!(w.player.grounded);
    assert_eq!(w.player.y, 15 * TILE - player::H, "on the floor below");
}

#[test]
fn jumping_up_through_a_platform_lands_on_it() {
    let mut w = world(room(&[(12, 1, "========")]));
    jump(&mut w, 30, 80);
    assert!(w.player.grounded);
    assert_eq!(w.player.y, 12 * TILE - player::H);
}

#[test]
fn dashes_go_eight_ways_and_recharge_on_the_ground() {
    let mut w = world(room(&[]));
    jump(&mut w, 10, 0);
    let ev = run(
        &mut w,
        Intent {
            move_x: 1.0,
            move_y: -1.0,
            dash_pressed: true,
            ..Intent::default()
        },
        1,
    );
    assert!(
        ev.iter()
            .any(|e| matches!(e, Event::Dashed { dx, dy } if *dx > 0.5 && *dy < -0.5))
    );
    assert!(w.player.vx > 150.0 && w.player.vy < -150.0);
    assert_eq!(w.player.dashes, 0);
    // No second dash in the air.
    let ev = run(
        &mut w,
        Intent {
            dash_pressed: true,
            ..Intent::default()
        },
        30,
    );
    assert!(!ev.iter().any(|e| matches!(e, Event::Dashed { .. })));
    let ev = run(&mut w, Intent::default(), 120);
    assert!(ev.contains(&Event::Recharged { crystal: false }));
    assert_eq!(w.player.dashes, 1);
}

#[test]
fn a_downward_dash_ends_on_the_ground() {
    let mut w = world(room(&[]));
    jump(&mut w, 30, 0);
    let mut ev = run(
        &mut w,
        Intent {
            move_y: 1.0,
            dash_pressed: true,
            ..Intent::default()
        },
        1,
    );
    ev.extend(run(
        &mut w,
        Intent {
            move_y: 1.0,
            ..Intent::default()
        },
        30,
    ));
    assert!(ev.iter().any(|e| matches!(e, Event::Landed { .. })));
    assert_eq!(w.player.mode, Mode::Normal);
}

#[test]
fn crystals_recharge_in_the_air_and_come_back() {
    let mut w = world(room(&[(10, 2, "*")]));
    jump(&mut w, 2, 0);
    run(
        &mut w,
        Intent {
            move_y: -1.0,
            dash_pressed: true,
            ..Intent::default()
        },
        1,
    );
    let ev = run(&mut w, Intent::default(), 30);
    assert!(ev.contains(&Event::Recharged { crystal: true }), "{ev:?}");
    assert!(!w.level.refills()[0].ready());
    run(&mut w, Intent::default(), 400);
    assert!(w.level.refills()[0].ready());
}

#[test]
fn climbing_costs_stamina_and_wall_jumps_push_away() {
    // A wall column at x 10, from row 4 down to the floor.
    let mut w = world(room(&(4..15).map(|y| (y, 10, "#")).collect::<Vec<_>>()));
    w.player.x = 10 * TILE - player::W;
    run(
        &mut w,
        Intent {
            move_x: 1.0,
            ..Intent::default()
        },
        2,
    );
    let grab = Intent {
        grab: true,
        ..Intent::default()
    };
    jump(&mut w, 10, 0);
    let ev = run(&mut w, grab, 2);
    assert!(ev.contains(&Event::Grabbed), "{ev:?}");
    let full = w.player.stamina;
    run(
        &mut w,
        Intent {
            move_y: -1.0,
            ..grab
        },
        30,
    );
    assert!(w.player.stamina < full);
    assert_eq!(w.player.mode, Mode::Climb);
    let ev = run(
        &mut w,
        Intent {
            move_x: -1.0,
            jump_pressed: true,
            jump_held: true,
            ..grab
        },
        1,
    );
    assert_eq!(jumps(&ev), [(JumpKind::Wall, false)]);
    assert!(w.player.vx < -100.0);
    assert_eq!(w.player.facing, -1);
}

#[test]
fn climbing_over_the_top_hops() {
    let mut w = world(room(&(11..15).map(|y| (y, 10, "#")).collect::<Vec<_>>()));
    w.player.x = 10 * TILE - player::W;
    run(
        &mut w,
        Intent {
            move_x: 1.0,
            ..Intent::default()
        },
        2,
    );
    jump(&mut w, 4, 0);
    let ev = run(
        &mut w,
        Intent {
            grab: true,
            move_y: -1.0,
            ..Intent::default()
        },
        120,
    );
    assert!(ev.contains(&Event::ClimbHop), "{ev:?}");
}

#[test]
fn running_out_of_stamina_lets_go() {
    // A grip that never tires into a slip, so it holds until stamina is gone.
    let firm = Tuning {
        tired_stamina: 0.0,
        ..Tuning::default()
    };
    let mut w = Sim::new(
        room(&(1..15).map(|y| (y, 10, "#")).collect::<Vec<_>>()),
        firm,
    );
    w.player.x = 10 * TILE - player::W;
    run(
        &mut w,
        Intent {
            move_x: 1.0,
            ..Intent::default()
        },
        2,
    );
    jump(&mut w, 4, 0);
    let ev = run(
        &mut w,
        Intent {
            grab: true,
            ..Intent::default()
        },
        1500,
    );
    assert!(ev.contains(&Event::StaminaOut));
}

#[test]
fn sliding_down_a_wall_caps_the_fall() {
    let mut w = world(room(&(1..13).map(|y| (y, 10, "#")).collect::<Vec<_>>()));
    w.player.x = 10 * TILE - player::W;
    w.player.y = 2 * TILE;
    w.player.grounded = false;
    run(
        &mut w,
        Intent {
            move_x: 1.0,
            ..Intent::default()
        },
        60,
    );
    assert_eq!(w.player.wall_slide, 1);
    assert!(w.player.vy <= Tuning::default().wall_slide_speed + 1.0);
}

#[test]
fn dives_plunge_and_impact() {
    let mut w = world(room(&[]));
    jump(&mut w, 30, 0);
    let ev = run(
        &mut w,
        Intent {
            dive_pressed: true,
            ..Intent::default()
        },
        1,
    );
    assert!(ev.contains(&Event::DiveStarted));
    let ev = run(&mut w, Intent::default(), 60);
    assert!(
        ev.iter().any(|e| matches!(e, Event::DiveImpact { .. })),
        "{ev:?}"
    );
    assert!(ev.contains(&Event::Recovered));
}

#[test]
fn the_head_slips_past_ceiling_corners() {
    // A ceiling over tiles 1..2 at row 12: the hero at x 22..28 clips its corner by 2 px.
    let mut w = world(room(&[(12, 1, "##")]));
    w.player.x = 3 * TILE - 2;
    let ev = jump(&mut w, 20, 0);
    assert!(
        ev.iter()
            .any(|e| matches!(e, Event::CornerCorrected { .. })),
        "{ev:?}"
    );
}

/// A world in a room with a wall column at tile x 10 over `rows`, the hero against it
/// and in the air at `y` px, with `tuning`.
fn at_wall(rows: std::ops::Range<usize>, y: i32, tuning: Tuning) -> Sim {
    let mut w = Sim::new(
        room(&rows.map(|r| (r, 10, "#")).collect::<Vec<_>>()),
        tuning,
    );
    w.player.x = 10 * TILE - player::W;
    w.player.y = y;
    w.player.grounded = false;
    w
}

#[test]
fn a_firm_grip_holds_and_a_slipping_grip_slides() {
    let firm = Tuning::default();
    let mut w = at_wall(1..15, 4 * TILE, firm);
    run(
        &mut w,
        Intent {
            grab: true,
            ..Intent::default()
        },
        30,
    );
    let y = w.player.y;
    run(
        &mut w,
        Intent {
            grab: true,
            ..Intent::default()
        },
        60,
    );
    assert_eq!(w.player.y, y, "a firm grip holds");

    let slip = Tuning {
        slip_speed: 25.0,
        ..firm
    };
    let mut w = at_wall(1..15, 4 * TILE, slip);
    run(
        &mut w,
        Intent {
            grab: true,
            ..Intent::default()
        },
        30,
    );
    let y = w.player.y;
    run(
        &mut w,
        Intent {
            grab: true,
            ..Intent::default()
        },
        60,
    );
    assert!(
        w.player.y > y + 8,
        "a slipping grip slides: {} -> {}",
        y,
        w.player.y
    );
    assert!(w.player.slipping);
    assert_eq!(w.player.mode, Mode::Climb, "and still holds on");
}

#[test]
fn a_tired_grip_slips_and_cannot_climb() {
    let mut w = at_wall(1..15, 4 * TILE, Tuning::default());
    run(
        &mut w,
        Intent {
            grab: true,
            ..Intent::default()
        },
        2,
    );
    w.player.stamina = 10.0;
    let y = w.player.y;
    run(
        &mut w,
        Intent {
            grab: true,
            move_y: -1.0,
            ..Intent::default()
        },
        30,
    );
    assert!(w.player.y > y, "slipping down, not climbing up");
}

#[test]
fn the_wall_stick_turns_away_and_jump_into_a_wall_jump() {
    let t = Preset::Momentum.tuning();
    let mut w = at_wall(1..15, 3 * TILE, t);
    let into = Intent {
        move_x: 1.0,
        ..Intent::default()
    };
    run(&mut w, into, 30);
    assert_eq!(w.player.wall_slide, 1);
    // Pressing away: stuck for a moment, still beside the wall.
    let away = Intent {
        move_x: -1.0,
        ..Intent::default()
    };
    run(&mut w, away, 6);
    assert!(w.player.wall(&w.level, 1), "stuck to the wall");
    let ev = run(
        &mut w,
        Intent {
            jump_pressed: true,
            jump_held: true,
            ..away
        },
        1,
    );
    assert_eq!(jumps(&ev), [(JumpKind::Wall, false)]);
    // Without the stick, the same input falls away first.
    let mut w = at_wall(
        1..15,
        3 * TILE,
        Tuning {
            wall_stick_time: 0.0,
            wall_jump_reach: 1,
            ..t
        },
    );
    run(&mut w, into, 30);
    run(&mut w, away, 6);
    assert!(!w.player.wall(&w.level, 1));
}

#[test]
fn presets_switch_abilities_off() {
    let mut w = Sim::new(room(&[]), Preset::Momentum.tuning());
    jump(&mut w, 10, 0);
    let ev = run(
        &mut w,
        Intent {
            dash_pressed: true,
            dive_pressed: true,
            jump_pressed: true,
            ..Intent::default()
        },
        1,
    );
    assert!(
        ev.is_empty()
            || !ev.iter().any(|e| matches!(
                e,
                Event::Dashed { .. } | Event::DiveStarted | Event::Jumped { .. }
            )),
        "{ev:?}"
    );

    let mut w = Sim::new(room(&[]), Preset::Snappy.tuning());
    jump(&mut w, 10, 0);
    let ev = run(
        &mut w,
        Intent {
            move_x: 1.0,
            move_y: -1.0,
            dash_pressed: true,
            ..Intent::default()
        },
        1,
    );
    assert!(
        ev.contains(&Event::Dashed { dx: 1.0, dy: 0.0 }),
        "sideways only: {ev:?}"
    );
}

/// On the demo level, start standing at `x` with feet at `feet`, press jump (held) while
/// holding `move_x` for `steps` steps, then let go; whether the hero ever stands with
/// feet at `target`.
fn reaches(x: i32, feet: i32, move_x: f32, steps: usize, target: i32) -> bool {
    let mut w = world(Level::demo());
    w.player.x = x;
    w.player.y = feet - player::H;
    w.player.peak_y = w.player.y;
    run(&mut w, Intent::default(), 3);
    assert!(w.player.grounded, "start on the ground at {x},{feet}");
    let go = Intent {
        move_x,
        jump_held: true,
        ..Intent::default()
    };
    w.step(&Intent {
        jump_pressed: true,
        ..go
    });
    for k in 0..240 {
        let i = if k < steps { go } else { Intent::default() };
        w.step(&i);
        if w.player.grounded && w.player.y + player::H == target {
            return true;
        }
    }
    false
}

#[test]
fn every_demo_stair_is_within_a_plain_jump() {
    let (floor, s1, s2, s3, tower) = (25 * TILE, 22 * TILE, 19 * TILE, 16 * TILE, 13 * TILE);
    assert!(reaches(10 * TILE, floor, -1.0, 25, s1), "floor -> step 1");
    assert!(reaches(7 * TILE, s1, -1.0, 30, s2), "step 1 -> step 2");
    assert!(reaches(2 * TILE, s2, 1.0, 30, s3), "step 2 -> step 3");
    assert!(reaches(7 * TILE, s3, -1.0, 30, tower), "step 3 -> tower");
}

#[test]
fn a_tapped_jump_does_not_reach_the_first_step() {
    // The steps are tuned to need the held jump, not a tap.
    let mut w = world(Level::demo());
    w.player.x = 10 * TILE;
    run(&mut w, Intent::default(), 3);
    let left = Intent {
        move_x: -1.0,
        ..Intent::default()
    };
    w.step(&Intent {
        jump_pressed: true,
        jump_held: true,
        ..left
    });
    let mut highest = w.player.y;
    for _ in 0..120 {
        w.step(&left);
        highest = highest.min(w.player.y);
    }
    assert!(
        25 * TILE - (highest + player::H) < 24,
        "a tap rises less than a step"
    );
}

#[test]
fn holding_up_or_down_standing_still_looks() {
    let mut w = world(room(&[]));
    run(
        &mut w,
        Intent {
            move_y: -1.0,
            ..Intent::default()
        },
        30,
    );
    assert_eq!(w.player.look, -1);
    assert!(w.player.look_time > 0.2);
    run(
        &mut w,
        Intent {
            move_y: 1.0,
            ..Intent::default()
        },
        1,
    );
    assert_eq!((w.player.look, w.player.look_time), (1, 0.0));
    run(
        &mut w,
        Intent {
            move_x: 1.0,
            move_y: 1.0,
            ..Intent::default()
        },
        1,
    );
    assert_eq!(w.player.look, 0, "not while moving");
}

/// Wall-jump repeatedly holding toward a single wall at tile x 10 for `steps` steps;
/// the height gained in pixels.
fn scale_wall(t: Tuning, steps: usize) -> i32 {
    let mut w = at_wall(1..15, 15 * TILE - player::H, t);
    run(
        &mut w,
        Intent {
            move_x: 1.0,
            ..Intent::default()
        },
        5,
    );
    assert!(w.player.grounded);
    let start = w.player.y;
    let mut top = start;
    for _ in 0..steps {
        // Press jump whenever the wall is within reach and not still rising fast.
        let near = (1..=2).any(|d| w.level.overlaps(w.player.rect().shifted(d, 0)));
        let press = near && w.player.vy > -30.0;
        w.step(&Intent {
            move_x: 1.0,
            jump_pressed: press,
            jump_held: true,
            ..Intent::default()
        });
        top = top.min(w.player.y);
    }
    start - top
}

#[test]
fn wall_jumps_toward_the_wall_scale_it() {
    for p in [Preset::Hybrid, Preset::Snappy, Preset::Momentum] {
        let gained = scale_wall(p.tuning(), 300);
        assert!(gained > 60, "{}: gained only {gained} px", p.name());
    }
}

#[test]
fn the_stick_aims_wall_jumps() {
    /// Velocity right after a wall jump off a wall on the right, with this stick.
    fn kick(move_x: f32, move_y: f32) -> (f32, f32) {
        let mut w = at_wall(1..15, 6 * TILE, Tuning::default());
        run(
            &mut w,
            Intent {
                move_x: 1.0,
                ..Intent::default()
            },
            10,
        );
        let ev = run(
            &mut w,
            Intent {
                move_x,
                move_y,
                jump_pressed: true,
                jump_held: true,
                ..Intent::default()
            },
            1,
        );
        assert_eq!(jumps(&ev), [(JumpKind::Wall, false)]);
        (w.player.vx, w.player.vy)
    }
    let (toward, neutral, away) = (kick(1.0, 0.0).0, kick(0.0, 0.0).0, kick(-1.0, 0.0).0);
    assert!(
        toward > neutral && neutral > away,
        "{toward} {neutral} {away}"
    );
    assert!(
        kick(0.0, -1.0).1 < kick(0.0, 1.0).1,
        "up jumps higher than down"
    );
}

#[test]
fn a_wall_gives_the_air_jump_back() {
    // From the floor left of a wall: jump, spend the air jump, slide down the wall, jump
    // off it, and air-jump again without landing.
    let mut w = Sim::new(
        room(&(1..15).map(|r| (r, 10, "#")).collect::<Vec<_>>()),
        Tuning::default(),
    );
    w.player.x = 7 * TILE;
    run(&mut w, Intent::default(), 3);
    let right = Intent {
        move_x: 1.0,
        jump_held: true,
        ..Intent::default()
    };
    let mut ev = run(
        &mut w,
        Intent {
            jump_pressed: true,
            ..right
        },
        1,
    );
    ev.extend(run(&mut w, right, 10));
    ev.extend(run(
        &mut w,
        Intent {
            jump_pressed: true,
            ..right
        },
        1,
    ));
    let kinds: Vec<_> = jumps(&ev).into_iter().map(|(k, _)| k).collect();
    assert_eq!(kinds, [JumpKind::Ground, JumpKind::Double]);
    assert_eq!(w.player.air_jumps, 0);
    // Into the wall until the slide starts, still in the air.
    for _ in 0..60 {
        if w.player.wall_slide != 0 {
            break;
        }
        w.step(&Intent {
            move_x: 1.0,
            ..Intent::default()
        });
    }
    w.step(&Intent {
        move_x: 1.0,
        ..Intent::default()
    });
    assert!(!w.player.grounded, "still in the air");
    assert_eq!(w.player.wall_slide, 1);
    assert_eq!(w.player.air_jumps, 1, "the wall refilled it");
    let left = Intent {
        move_x: -1.0,
        jump_held: true,
        ..Intent::default()
    };
    let ev = run(
        &mut w,
        Intent {
            jump_pressed: true,
            ..left
        },
        1,
    );
    assert_eq!(jumps(&ev), [(JumpKind::Wall, false)]);
    run(&mut w, left, 15);
    let ev = run(
        &mut w,
        Intent {
            jump_pressed: true,
            ..left
        },
        1,
    );
    assert_eq!(jumps(&ev), [(JumpKind::Double, false)]);
}

#[test]
fn celeste_walls_do_not_give_the_dash_back() {
    let mut w = at_wall(1..15, 3 * TILE, Preset::Precise.tuning());
    w.player.dashes = 0;
    run(
        &mut w,
        Intent {
            move_x: 1.0,
            ..Intent::default()
        },
        40,
    );
    assert_eq!(w.player.wall_slide, 1);
    assert_eq!(w.player.dashes, 0);
    let mut w = at_wall(1..15, 3 * TILE, Preset::Snappy.tuning());
    w.player.dashes = 0;
    run(
        &mut w,
        Intent {
            move_x: 1.0,
            ..Intent::default()
        },
        40,
    );
    assert_eq!(w.player.dashes, 1);
}

#[test]
fn holding_a_jump_pressed_in_the_air_glides() {
    let t = Preset::Drifter.tuning();
    let mut w = Sim::new(room(&[]), t);
    // A ground jump held through its apex does not glide: that's hangtime.
    jump(&mut w, 25, 0);
    let ev = run(
        &mut w,
        Intent {
            jump_held: true,
            ..Intent::default()
        },
        20,
    );
    assert!(!ev.contains(&Event::GlideStarted));
    // Pressed again in the air: the air jump goes first, then the glide once falling.
    let press = Intent {
        jump_pressed: true,
        jump_held: true,
        ..Intent::default()
    };
    let mut ev = run(&mut w, press, 1);
    assert_eq!(jumps(&ev), [(JumpKind::Double, false)]);
    ev.extend(run(
        &mut w,
        Intent {
            jump_held: true,
            ..Intent::default()
        },
        60,
    ));
    assert!(ev.contains(&Event::GlideStarted), "{ev:?}");
    assert!(w.player.gliding);
    assert!(w.player.vy <= t.glide_fall_speed + 0.5);
    // Letting go closes it.
    let ev = run(&mut w, Intent::default(), 1);
    assert!(ev.contains(&Event::GlideEnded));
}

#[test]
fn tapping_down_while_sprinting_rolls() {
    let mut w = world(room(&[]));
    let walk = Intent {
        move_x: 1.0,
        ..Intent::default()
    };
    let sprint = Intent {
        sprint: true,
        ..walk
    };
    // Plain running is too slow for it.
    run(&mut w, walk, 40);
    let ev = run(
        &mut w,
        Intent {
            move_y: 1.0,
            ..walk
        },
        1,
    );
    assert!(!ev.contains(&Event::RollStarted));
    run(&mut w, sprint, 40);
    let ev = run(
        &mut w,
        Intent {
            move_y: 1.0,
            ..sprint
        },
        1,
    );
    assert!(ev.contains(&Event::RollStarted), "{ev:?}");
    // Holding down doesn't roll again.
    let ev = run(
        &mut w,
        Intent {
            move_y: 1.0,
            ..sprint
        },
        60,
    );
    assert!(!ev.contains(&Event::RollStarted));
}

#[test]
fn sprinting_raises_the_top_speed_where_the_preset_has_it() {
    /// Top speed after a second of running, sprinting or not, with `t`.
    fn top(t: Tuning, sprint: bool) -> f32 {
        let mut w = Sim::new(room(&[]), t);
        run(
            &mut w,
            Intent {
                move_x: 1.0,
                sprint,
                ..Intent::default()
            },
            120,
        );
        w.player.vx
    }
    let t = Tuning::default();
    assert!((top(t, false) - t.run_speed).abs() < 1.0);
    assert!((top(t, true) - t.sprint_speed).abs() < 1.0);
    let hk = Preset::Snappy.tuning();
    assert_eq!(
        top(hk, true),
        top(hk, false),
        "no sprint in the snappy preset"
    );
}

/// Hold super dash for `steps` steps, then let go; the events.
fn charge(w: &mut Sim, steps: usize) -> Vec<Event> {
    let mut ev = run(
        w,
        Intent {
            super_dash: true,
            ..Intent::default()
        },
        steps,
    );
    ev.extend(run(w, Intent::default(), 1));
    ev
}

#[test]
fn a_charged_super_dash_flies_level_until_a_wall() {
    let mut w = world(room(&[]));
    w.player.x = 3 * TILE;
    run(&mut w, Intent::default(), 3);
    let ev = charge(&mut w, 110);
    assert!(ev.contains(&Event::SuperCharged));
    assert!(ev.contains(&Event::SuperDashStarted { dir: 1 }), "{ev:?}");
    let y = w.player.y;
    let mut ev = Vec::new();
    for _ in 0..120 {
        w.step(&Intent::default());
        ev.extend(w.events.iter().copied());
        if matches!(w.player.mode, Mode::SuperDash { .. }) {
            assert_eq!(w.player.y, y, "level flight");
            assert!(w.player.vx > 250.0);
        }
    }
    assert!(ev.contains(&Event::SuperDashEnded { bonk: true }), "{ev:?}");
    assert!(w.player.x > 18 * TILE, "flew across the room");
}

#[test]
fn letting_go_early_does_nothing_and_jump_cancels_the_flight() {
    let mut w = world(room(&[]));
    run(&mut w, Intent::default(), 3);
    let ev = charge(&mut w, 40);
    assert!(
        !ev.iter()
            .any(|e| matches!(e, Event::SuperDashStarted { .. }))
    );
    assert_eq!(w.player.mode, Mode::Normal);
    charge(&mut w, 110);
    run(&mut w, Intent::default(), 10);
    let ev = run(
        &mut w,
        Intent {
            jump_pressed: true,
            jump_held: true,
            ..Intent::default()
        },
        1,
    );
    assert!(ev.contains(&Event::SuperDashEnded { bonk: false }));
    assert!(jumps(&ev).is_empty(), "the press cancels, it doesn't jump");
    assert_eq!(w.player.mode, Mode::Normal);
}

#[test]
fn a_super_dash_charged_on_a_wall_launches_away_from_it() {
    let mut w = at_wall(1..15, 5 * TILE, Tuning::default());
    run(
        &mut w,
        Intent {
            move_x: 1.0,
            ..Intent::default()
        },
        20,
    );
    assert_eq!(w.player.wall_slide, 1);
    let y = w.player.y;
    let ev = charge(&mut w, 110);
    assert!(ev.contains(&Event::SuperDashStarted { dir: -1 }), "{ev:?}");
    assert!(w.player.y - y <= 2, "held on the wall while charging");
    let mut w = Sim::new(room(&[]), Preset::Precise.tuning());
    run(&mut w, Intent::default(), 3);
    let ev = charge(&mut w, 110);
    assert!(
        !ev.contains(&Event::SuperCharged),
        "no super dash in the precise preset"
    );
}

#[test]
fn a_bigger_body_collides_by_its_own_size() {
    // A low passage: 2 tiles (16 units) of headroom from x = 6 tiles on.
    let low = room(&[(12, 6, "##################")]);
    let right = Intent {
        move_x: 1.0,
        ..Intent::default()
    };
    let mut small = world(low.clone());
    run(&mut small, right, 240);
    assert!(
        small.player.x > 10 * TILE,
        "6x12 fits under: {}",
        small.player.x
    );

    let tall = Tuning {
        size: (10, 20),
        ..Tuning::default()
    };
    let mut big = Sim::new(low, tall);
    assert_eq!(big.player.rect().bottom(), 15 * TILE, "stands on the floor");
    run(&mut big, right, 240);
    assert_eq!(
        big.player.rect().right(),
        6 * TILE,
        "10x20 stops at the passage"
    );
    assert!(big.player.grounded);
}

/// A floor at y = 200 and one moving platform, 32x8, at a steady speed.
#[derive(Clone)]
struct Elevator {
    /// The platform now.
    plat: Rect,
    /// The platform before the last update.
    prev: Rect,
    /// Its speed, units/s.
    v: (f32, f32),
    /// Sub-unit motion not yet moved.
    rem: (f32, f32),
}

impl Elevator {
    /// A platform with its top-left at `x, y` moving at `v`.
    fn new(x: i32, y: i32, v: (f32, f32)) -> Self {
        let plat = Rect { x, y, w: 32, h: 8 };
        Self {
            plat,
            prev: plat,
            v,
            rem: (0.0, 0.0),
        }
    }
}

impl Terrain for Elevator {
    fn solid(&self, r: Rect) -> bool {
        r.bottom() > 200 || r.intersects(self.plat)
    }

    fn update(&mut self, dt: f32, _bodies: &[Rect]) {
        self.prev = self.plat;
        self.rem.0 += self.v.0 * dt;
        self.rem.1 += self.v.1 * dt;
        let (nx, ny) = (self.rem.0.round(), self.rem.1.round());
        self.rem = (self.rem.0 - nx, self.rem.1 - ny);
        self.plat.x += nx as i32;
        self.plat.y += ny as i32;
    }

    fn carry(&self, r: Rect) -> (f32, f32) {
        if r.intersects(self.prev) {
            (
                (self.plat.x - self.prev.x) as f32,
                (self.plat.y - self.prev.y) as f32,
            )
        } else {
            (0.0, 0.0)
        }
    }

    crate::terrain_layer!();
}

/// A character standing on `e`'s platform, 10 units in from its left edge.
fn on_elevator(e: &Elevator, tuning: Tuning) -> World {
    World::new(tuning, Spot::TopLeft(e.plat.x + 10, e.plat.y - player::H))
}

#[test]
fn moving_ground_carries_the_character_along() {
    for v in [
        (30.0, 0.0),
        (-45.0, 0.0),
        (0.0, -40.0),
        (0.0, 50.0),
        (25.0, -25.0),
    ] {
        let mut e = Elevator::new(80, 120, v);
        let mut w = on_elevator(&e, Tuning::default());
        let mut events = Vec::new();
        for _ in 0..120 {
            w.step(&mut e, &Intent::default());
            events.extend(w.events.iter().copied());
        }
        let r = w.player.rect();
        assert!(w.player.grounded, "{v:?}: still standing");
        assert_eq!(r.bottom(), e.plat.y, "{v:?}: on top of it");
        assert_eq!(r.x - e.plat.x, 10, "{v:?}: where it stood on it");
        assert!(!e.solid(r), "{v:?}: not stuck in it");
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::LeftGround | Event::Landed { .. })),
            "{v:?}: {events:?}"
        );
    }
}

#[test]
fn moving_ground_stops_at_walls_and_ceilings() {
    // Rising toward a ceiling at y = 60 that the platform passes beside.
    struct Lid(Elevator);
    impl Terrain for Lid {
        fn solid(&self, r: Rect) -> bool {
            self.0.solid(r) || (r.y < 60 && r.x < 100)
        }
        // The elevator's update and carry, through a tuple field.
        crate::terrain_layer!(0);
    }
    let mut lid = Lid(Elevator::new(80, 120, (0.0, -40.0)));
    let mut w = on_elevator(&lid.0, Tuning::default());
    while lid.0.plat.y > 72 {
        w.step(&mut lid, &Intent::default());
        assert!(!w.events.contains(&Event::Squished));
    }
    assert_eq!(w.player.y, 60, "held under the ceiling");
    assert!(!lid.solid(w.player.rect()));
    // The platform keeps rising into the feet (a unit every three steps).
    let squished = (0..4).any(|_| {
        w.step(&mut lid, &Intent::default());
        w.events.contains(&Event::Squished)
    });
    assert!(squished);

    // Sideways into a wall: held there, the platform slides on under the feet.
    struct Wall(Elevator);
    impl Terrain for Wall {
        fn solid(&self, r: Rect) -> bool {
            self.0.solid(r) || r.right() > 120
        }
        crate::terrain_layer!(0);
    }
    let mut wall = Wall(Elevator::new(60, 120, (30.0, 0.0)));
    let mut w = on_elevator(&wall.0, Tuning::default());
    for _ in 0..200 {
        w.step(&mut wall, &Intent::default());
        assert!(!w.events.contains(&Event::Squished));
    }
    assert_eq!(w.player.rect().right(), 120, "against the wall");
}

#[test]
fn jumping_off_moving_ground_keeps_its_speed_per_tuning() {
    let press = Intent {
        jump_pressed: true,
        jump_held: true,
        ..Intent::default()
    };
    let launch = |boost: f32, v: (f32, f32)| {
        let t = Tuning {
            lift_boost: boost,
            ..Tuning::default()
        };
        let mut e = Elevator::new(80, 120, v);
        let mut w = on_elevator(&e, t);
        for _ in 0..60 {
            w.step(&mut e, &Intent::default());
        }
        w.step(&mut e, &press);
        (w.player.vx, w.player.vy, t.jump_speed)
    };
    let (vx, _, _) = launch(1.0, (60.0, 0.0));
    assert!((vx - 60.0).abs() < 6.0, "keeps the ride's speed: {vx}");
    let (vx, _, _) = launch(0.0, (60.0, 0.0));
    assert!(vx.abs() < 1.0, "lift boost off: {vx}");
    let (_, vy, jump) = launch(1.0, (0.0, -60.0));
    assert!(vy < -jump - 50.0, "rising adds to the jump: {vy}");
    assert!(vy > -jump - 70.0, "about its speed: {vy}");
    let (_, vy, jump) = launch(1.0, (0.0, 60.0));
    assert!(vy <= -jump + 10.0, "sinking takes nothing away: {vy}");
}

#[test]
fn a_still_level_carries_nothing() {
    let mut w = world(room(&[]));
    run(&mut w, Intent::default(), 60);
    assert_eq!(w.player.lift, (0.0, 0.0));
}

#[test]
fn a_moving_wall_carries_the_character_holding_it() {
    let climb = |v: (f32, f32)| {
        let mut e = Elevator::new(0, 0, v);
        e.plat = Rect {
            x: 100,
            y: 40,
            w: 8,
            h: 120,
        };
        e.prev = e.plat;
        let mut w = World::new(Tuning::default(), Spot::TopLeft(94, 100));
        let hold = Intent {
            move_x: 1.0,
            grab: true,
            ..Intent::default()
        };
        for _ in 0..60 {
            w.step(&mut e, &hold);
        }
        assert_eq!(w.player.mode, Mode::Climb, "{v:?}");
        w.player.y
    };
    let still = climb((0.0, 0.0));
    let rising = climb((0.0, -30.0));
    assert!(
        (still - rising - 15).abs() <= 1,
        "half a second at 30 units/s: {still} vs {rising}"
    );
}

/// The lunge's mode and event.
const LUNGE: CustomId = CustomId::new("tests::lunge");

/// The dig's event.
const DIG: CustomId = CustomId::new("tests::dig");

/// Custom button 0 lunges forward on the ground for 0.15 s; counts its lunges.
#[derive(Clone, Debug, Default)]
struct Lunge {
    /// Lunges so far.
    count: u32,
}

impl Ability for Lunge {
    fn before(&mut self, s: &mut Step<'_>) {
        let p = &mut *s.player;
        match p.mode {
            Mode::Normal if s.intent.pressed(0) && p.grounded => {
                self.count += 1;
                p.mode = Mode::Custom { id: LUNGE, t: 0.0 };
                p.vx = 240.0 * p.facing as f32;
                s.events
                    .push(Event::custom(LUNGE).with([self.count as f32, 0.0, 0.0, 0.0]));
            }
            Mode::Custom { id: LUNGE, t } if t >= 0.15 => p.mode = Mode::Normal,
            _ => {}
        }
    }

    fn after(&mut self, s: &mut Step<'_>) {
        // Against a wall, the lunge ends at once.
        if matches!(s.player.mode, Mode::Custom { id: LUNGE, .. }) && s.player.vx == 0.0 {
            s.player.mode = Mode::Normal;
        }
    }
}

#[test]
fn an_ability_takes_the_body_over_and_hands_it_back() {
    let mut w = world(room(&[]));
    w.abilities.push(Box::new(Lunge::default()));
    let x0 = w.player.x;
    let press = Intent {
        custom: 1,
        custom_pressed: 1,
        jump_pressed: true,
        jump_held: true,
        ..Intent::default()
    };
    w.step(&press);
    assert!(
        w.events
            .iter()
            .any(|e| e.custom_data(LUNGE) == Some([1.0, 0.0, 0.0, 0.0])),
        "the first lunge, with its count"
    );
    assert!(
        !w.events.iter().any(|e| matches!(e, Event::Jumped { .. })),
        "no built-in move during a custom mode: {:?}",
        w.events
    );
    let events = run(&mut w, Intent::default(), 17);
    assert!(
        matches!(w.player.mode, Mode::Custom { id: LUNGE, .. }),
        "still lunging"
    );
    assert!(
        w.player.x - x0 >= 30,
        "carried by its speed: {}",
        w.player.x - x0
    );
    assert!(w.player.grounded && events.is_empty(), "{events:?}");
    run(&mut w, Intent::default(), 2);
    assert_eq!(w.player.mode, Mode::Normal, "the clock ran out");

    // Into the room's right wall: the after hook ends it.
    w.player.x = 21 * TILE;
    w.step(&press);
    run(&mut w, Intent::default(), 6);
    assert_eq!(w.player.mode, Mode::Normal);
    assert_eq!(w.player.rect().right(), 23 * TILE);
}

#[test]
fn a_cloned_world_with_abilities_plays_out_identically() {
    let mut a = world(Level::demo());
    a.abilities.push(Box::new(Lunge::default()));
    let script = |k: u64| Intent {
        move_x: if k % 90 < 60 { 1.0 } else { -1.0 },
        custom_pressed: u32::from(k.is_multiple_of(37)),
        jump_pressed: k.is_multiple_of(53),
        jump_held: k % 53 < 20,
        ..Intent::default()
    };
    for k in 0..200 {
        a.step(&script(k));
    }
    let mut b = a.clone();
    let (mut ea, mut eb) = (Vec::new(), Vec::new());
    for k in 200..800 {
        a.step(&script(k));
        b.step(&script(k));
        ea.extend(a.events.iter().copied());
        eb.extend(b.events.iter().copied());
    }
    assert_eq!(a.player, b.player);
    assert_eq!(ea, eb);
    assert!(
        ea.iter()
            .any(|e| matches!(e, Event::Custom { id: LUNGE, .. }))
    );
}

/// While custom button 0 is held: no dashes, no wall grabs, walking at 20 units/s at most.
#[derive(Clone, Debug)]
struct Laden;

impl Ability for Laden {
    fn before(&mut self, s: &mut Step<'_>) {
        if s.intent.held(0) {
            s.moves.dash = false;
            s.moves.climb = false;
            s.moves.max_run = 20.0;
        }
    }
}

#[test]
fn an_ability_rules_out_single_built_in_moves() {
    let mut w = world(room(&[]));
    w.abilities.push(Box::new(Laden));
    let laden = Intent {
        move_x: 1.0,
        custom: 1,
        ..Intent::default()
    };
    let ev = run(&mut w, laden, 60);
    assert!(
        (w.player.vx - 20.0).abs() < 0.01,
        "walks slowly: {}",
        w.player.vx
    );
    assert!(!ev.iter().any(|e| matches!(e, Event::Jumped { .. })));
    let ev = run(
        &mut w,
        Intent {
            dash_pressed: true,
            ..laden
        },
        1,
    );
    assert!(
        !ev.iter().any(|e| matches!(e, Event::Dashed { .. })),
        "{ev:?}"
    );
    // Jumping is still allowed.
    let ev = run(
        &mut w,
        Intent {
            jump_pressed: true,
            jump_held: true,
            ..laden
        },
        1,
    );
    assert!(
        ev.iter().any(|e| matches!(e, Event::Jumped { .. })),
        "{ev:?}"
    );
    // Let go of the button: the dash press, still buffered, fires now.
    let ev = run(&mut w, Intent::default(), 1);
    assert!(
        ev.iter().any(|e| matches!(e, Event::Dashed { .. })),
        "{ev:?}"
    );
}

#[test]
fn a_ruled_out_grab_lets_go_of_the_wall() {
    let t = Preset::Precise.tuning();
    let mut w = at_wall(1..15, 10 * TILE, t);
    w.abilities.push(Box::new(Laden));
    let grab = Intent {
        move_x: 1.0,
        grab: true,
        ..Intent::default()
    };
    run(&mut w, grab, 2);
    assert_eq!(w.player.mode, Mode::Climb);
    let ev = run(&mut w, Intent { custom: 1, ..grab }, 1);
    assert_ne!(w.player.mode, Mode::Climb);
    assert!(ev.contains(&Event::LetGo), "{ev:?}");
}

/// Custom button 0 digs out the floor tile under the feet; the after hook reports
/// whether it is still gone.
#[derive(Clone, Debug, Default)]
struct Dig {
    /// Tiles dug so far.
    dug: u32,
}

impl Ability for Dig {
    fn before(&mut self, s: &mut Step<'_>) {
        if !s.intent.pressed(0) {
            return;
        }
        let r = s.player.rect();
        if let Some(level) = s.terrain.find_mut::<Level>() {
            let (tx, ty) = (
                (r.x + r.w / 2).div_euclid(TILE),
                r.bottom().div_euclid(TILE),
            );
            level.set(tx, ty, Tile::Empty);
            self.dug += 1;
            s.events
                .push(Event::custom(DIG).with([self.dug as f32, 0.0, 0.0, 0.0]));
        }
    }
}

#[test]
fn an_ability_edits_the_terrain_within_the_step() {
    let mut w = world(room(&[]));
    // Standing within one floor tile, so digging it out leaves nothing under the feet.
    w.player.x = 5 * TILE + 1;
    run(&mut w, Intent::default(), 3);
    assert!(w.player.grounded);
    w.abilities.push(Box::new(Dig::default()));
    w.step(&Intent {
        custom_pressed: 1,
        ..Intent::default()
    });
    assert!(w.events.iter().any(|e| e.custom_data(DIG).is_some()));
    assert_eq!(w.level.tile(5, 15), Tile::Empty);
    assert!(!w.player.grounded, "fell into the hole in the same step");
    assert!(w.events.contains(&Event::LeftGround), "{:?}", w.events);
}

#[test]
fn a_cloned_world_editing_its_terrain_plays_out_identically() {
    let mut a = world(Level::demo());
    a.abilities.push(Box::new(Dig::default()));
    let script = |k: u64| Intent {
        move_x: if k % 120 < 70 { 1.0 } else { -1.0 },
        custom_pressed: u32::from(k.is_multiple_of(41)),
        jump_pressed: k.is_multiple_of(67),
        jump_held: k % 67 < 25,
        ..Intent::default()
    };
    for k in 0..100 {
        a.step(&script(k));
    }
    let mut b = a.clone();
    for k in 100..700 {
        a.step(&script(k));
        b.step(&script(k));
        assert_eq!(a.world.events, b.events);
    }
    assert_eq!(a.player, b.player);
    assert_eq!(a.level, b.level);
    assert_ne!(a.level, Level::demo(), "it dug");
}

/// A terrain built on another, overriding nothing but `solid` (which it answers the same).
struct Wrapped<T> {
    /// The terrain it builds on.
    inner: T,
}

impl<T: Terrain> Terrain for Wrapped<T> {
    fn solid(&self, r: Rect) -> bool {
        self.inner.solid(r)
    }
    fn base(&self) -> Option<&dyn Terrain> {
        Some(&self.inner)
    }
    fn base_mut(&mut self) -> Option<&mut dyn Terrain> {
        Some(&mut self.inner)
    }
}

#[test]
fn a_wrapper_plays_out_like_its_base() {
    // The demo level: platforms, refills, falling out, the update.
    let script = |k: u64| Intent {
        move_x: if k % 300 < 200 { 1.0 } else { -0.5 },
        jump_pressed: k.is_multiple_of(45),
        jump_held: k % 45 < 30,
        dash_pressed: k % 90 == 20,
        move_y: if k % 200 == 150 { 1.0 } else { -0.4 },
        ..Intent::default()
    };
    let mut level = Level::demo();
    let mut wrapped = Wrapped {
        inner: Level::demo(),
    };
    let mut a = World::on(&level, Tuning::default());
    let mut b = a.clone();
    let mut seen = Vec::new();
    for k in 0..1500 {
        a.step(&mut level, &script(k));
        b.step(&mut wrapped, &script(k));
        assert_eq!(a.events, b.events, "step {k}");
        seen.extend(a.events.iter().copied());
    }
    assert_eq!(a.player, b.player);
    assert_eq!(level, wrapped.inner);
    assert!(
        seen.contains(&Event::Recharged { crystal: true }),
        "used a refill"
    );

    // Moving ground: carry is forwarded too.
    let e = Elevator::new(40, 150, (0.0, -30.0));
    let mut a = on_elevator(&e, Tuning::default());
    let mut b = a.clone();
    let mut bare = e.clone();
    let mut wrapped = Wrapped { inner: e.clone() };
    for _ in 0..60 {
        a.step(&mut bare, &Intent::default());
        b.step(&mut wrapped, &Intent::default());
    }
    assert_eq!(a.player, b.player);
    assert!(a.player.y <= e.plat.y - player::H - 10, "carried up");
}

/// The room with custom tile 0 defined as `tile` at the given `(tile row, tile column)`s.
fn custom_room(cells: &[(i32, i32)], tile: CustomTile) -> Sim {
    let mut level = room(&[]);
    level.define(0, tile);
    for &(ty, tx) in cells {
        level.set(tx, ty, Tile::Custom(0));
    }
    world(level)
}

/// A surface like the default but for `f`.
fn surface(f: impl FnOnce(&mut Surface)) -> CustomTile {
    let mut s = Surface::DEFAULT;
    f(&mut s);
    CustomTile::solid(s)
}

/// How far the character slides after running right for `run_steps`, then letting go.
fn slide(w: &mut Sim, run_steps: usize) -> i32 {
    let go = Intent {
        move_x: 1.0,
        ..Intent::default()
    };
    run(w, go, run_steps);
    let x0 = w.player.x;
    run(w, Intent::default(), 120);
    assert!(w.player.vx.abs() < 1.0, "came to rest: {}", w.player.vx);
    w.player.x - x0
}

#[test]
fn ice_lets_the_run_slide_on() {
    let floor: Vec<(i32, i32)> = (1..23).map(|x| (15, x)).collect();
    let mut plain = custom_room(&floor, CustomTile::solid(Surface::DEFAULT));
    let mut ice = custom_room(&floor, surface(|s| s.friction = 0.15));
    assert!(ice.level.solid_tile(5, 15), "custom tiles can be solid");
    let (p, i) = (slide(&mut plain, 30), slide(&mut ice, 90));
    assert!(i > p + 10, "ice slides further: {i} vs {p}");
}

#[test]
fn a_conveyor_carries_what_stands_on_it_and_a_jump_keeps_it() {
    let floor: Vec<(i32, i32)> = (1..23).map(|x| (15, x)).collect();
    let mut w = custom_room(&floor, surface(|s| s.belt = 30.0));
    run(&mut w, Intent::default(), 10);
    let x0 = w.player.x;
    run(&mut w, Intent::default(), 120);
    let carried = w.player.x - x0;
    assert!(
        (28..=32).contains(&carried),
        "a second at 30 units/s: {carried}"
    );
    assert!(w.player.grounded);
    assert!((w.player.lift.0 - 30.0).abs() < 1.0, "{:?}", w.player.lift);
    // Off the belt in a jump, it takes the belt's speed along.
    let ev = jump(&mut w, 0, 0);
    assert!(jumps(&ev).iter().any(|(k, _)| *k == JumpKind::Ground));
    assert!(w.player.vx > 20.0, "kept the belt's speed: {}", w.player.vx);
}

#[test]
fn a_wall_that_is_not_climbable_cannot_be_grabbed_but_slid() {
    let wall: Vec<(i32, i32)> = (1..15).map(|y| (y, 10)).collect();
    let mut w = custom_room(&wall, surface(|s| s.climbable = false));
    w.player.x = 10 * TILE - player::W;
    w.player.y = 4 * TILE;
    w.player.grounded = false;
    let grab = Intent {
        move_x: 1.0,
        grab: true,
        ..Intent::default()
    };
    let ev = run(&mut w, grab, 30);
    assert!(!ev.contains(&Event::Grabbed), "{ev:?}");
    assert_ne!(w.player.mode, Mode::Climb);
    assert_eq!(w.player.wall_slide, 1, "slides down it");

    // The same wall, climbable: grabbed at once.
    let mut w = custom_room(&wall, CustomTile::solid(Surface::DEFAULT));
    w.player.x = 10 * TILE - player::W;
    w.player.y = 4 * TILE;
    w.player.grounded = false;
    assert!(run(&mut w, grab, 2).contains(&Event::Grabbed));
}

#[test]
fn bouncy_ground_bounces_a_landing_back_up() {
    let floor: Vec<(i32, i32)> = (1..23).map(|x| (15, x)).collect();
    let mut w = custom_room(&floor, surface(|s| s.bounce = 0.8));
    w.player.y = 4 * TILE;
    w.player.grounded = false;
    let mut bounced = None;
    for _ in 0..120 {
        w.step(&Intent::default());
        if let Some(impact) = w.events.iter().find_map(|e| match e {
            Event::Landed { impact, .. } => Some(*impact),
            _ => None,
        }) {
            bounced = Some((impact, w.player.vy, w.player.grounded));
            break;
        }
    }
    let (impact, vy, grounded) = bounced.expect("landed");
    assert!((vy + impact * 0.8).abs() < 1.0, "{vy} after {impact}");
    assert!(!grounded, "in the air again");
    run(&mut w, Intent::default(), 10);
    assert!(w.player.y < 15 * TILE - player::H - 4, "on the way up");
}

#[test]
fn a_custom_bouncy_platform_bounces_from_above_and_lets_through_from_below() {
    let bouncy = Surface {
        bounce: 0.8,
        ..Surface::DEFAULT
    };
    // A ledge two tiles over the room's floor.
    let ledge: Vec<(i32, i32)> = (1..23).map(|x| (13, x)).collect();
    let mut w = custom_room(&ledge, CustomTile::platform(bouncy));
    w.player.y = 4 * TILE;
    w.player.grounded = false;
    let mut bounced = None;
    for _ in 0..120 {
        w.step(&Intent::default());
        if let Some(impact) = w.events.iter().find_map(|e| match e {
            Event::Landed { impact, .. } => Some(*impact),
            _ => None,
        }) {
            bounced = Some((impact, w.player.vy, w.player.rect().bottom()));
            break;
        }
    }
    let (impact, vy, feet) = bounced.expect("landed on the platform");
    assert_eq!(feet, 13 * TILE, "on top of the ledge");
    assert!((vy + impact * 0.8).abs() < 1.0, "{vy} after {impact}");

    // From below, a jump passes up through it.
    let mut w = custom_room(&ledge, CustomTile::platform(bouncy));
    w.player.y = 15 * TILE - player::H;
    run(&mut w, Intent::default(), 10);
    assert!(w.player.grounded, "on the room's floor");
    let jump = Intent {
        jump_pressed: true,
        jump_held: true,
        ..Intent::default()
    };
    run(&mut w, jump, 1);
    let held = Intent {
        jump_held: true,
        ..Intent::default()
    };
    let mut top = w.player.y;
    for _ in 0..60 {
        run(&mut w, held, 1);
        top = top.min(w.player.y);
    }
    assert!(top + player::H < 13 * TILE, "rose through the ledge: {top}");
}

#[test]
fn contacts_say_what_the_body_ran_into() {
    let mut w = world(room(&[(11, 1, "######")]));
    let right = Intent {
        move_x: 1.0,
        ..Intent::default()
    };
    // Running into the room's right wall.
    w.player.x = 20 * TILE;
    run(&mut w, right, 40);
    assert_eq!(w.player.contacts.wall, 1);
    run(&mut w, Intent::default(), 10);
    assert_eq!(w.player.contacts, Contacts::default());
    // A jump into the low ceiling at the left.
    w.player.x = 2 * TILE;
    run(&mut w, Intent::default(), 30);
    let mut ceiling = false;
    let press = Intent {
        jump_pressed: true,
        jump_held: true,
        ..Intent::default()
    };
    w.step(&press);
    for _ in 0..30 {
        w.step(&Intent {
            jump_held: true,
            ..Intent::default()
        });
        ceiling |= w.player.contacts.ceiling;
    }
    assert!(ceiling, "bonked");
    let mut floor = false;
    for _ in 0..60 {
        w.step(&Intent::default());
        floor |= w.player.contacts.floor;
    }
    assert!(floor, "landed");
}

#[test]
fn a_level_can_have_bigger_tiles() {
    let level = Level::parse(&["....", ".P*.", "####"]).with_tile_size(16);
    assert!(level.solid(Rect {
        x: 0,
        y: 32,
        w: 4,
        h: 4
    }));
    assert!(!level.solid(Rect {
        x: 0,
        y: 28,
        w: 4,
        h: 4
    }));
    assert_eq!(level.start(), Spot::Feet(24, 32));
    assert_eq!(
        level.refills()[0].area,
        Rect {
            x: 32,
            y: 16,
            w: 16,
            h: 16
        }
    );
    assert_eq!(level.floor_y(), 48);
    let mut w = Sim::new(level, Tuning::default());
    run(&mut w, Intent::default(), 10);
    assert!(w.player.grounded);
    assert_eq!(w.player.rect().bottom(), 32);
}

/// A knockback ability from one crate: on custom button 1, a hop back, with its speed.
#[derive(Clone, Debug)]
struct Recoil;

/// Recoil's event, named after its crate.
const RECOIL: CustomId = CustomId::new("recoil_crate::recoil");

impl Ability for Recoil {
    fn before(&mut self, s: &mut Step<'_>) {
        if s.intent.pressed(1) {
            let vx = -60.0 * s.player.facing as f32;
            s.player.launch(vx, -100.0);
            s.events
                .push(Event::custom(RECOIL).with([vx, -100.0, 0.0, 0.0]));
        }
    }
}

#[test]
fn abilities_from_different_crates_need_no_shared_ids_and_carry_data() {
    let mut w = world(room(&[]));
    w.abilities.push(Box::new(Lunge::default()));
    w.abilities.push(Box::new(Recoil));
    assert_ne!(LUNGE, RECOIL);
    run(&mut w, Intent::default(), 5);
    w.step(&Intent {
        custom_pressed: 0b11,
        ..Intent::default()
    });
    let data: Vec<_> = w
        .events
        .iter()
        .filter_map(|e| e.custom_data(RECOIL))
        .collect();
    assert_eq!(data, [[-60.0, -100.0, 0.0, 0.0]]);
    assert!(w.events.iter().any(|e| e.custom_data(LUNGE).is_some()));
    assert_eq!(
        Event::Apex.with([1.0; 4]),
        Event::Apex,
        "only custom events take data"
    );
    assert_eq!(
        w.ability::<Lunge>().map(|l| l.count),
        Some(1),
        "typed access"
    );
    w.ability_mut::<Lunge>().unwrap().count = 7;
    assert_eq!(w.ability::<Lunge>().unwrap().count, 7);
    assert!(w.ability::<Laden>().is_none());
}

/// A layer over a level that counts its own ticks and never chains to the level's.
struct Counting {
    /// The level beneath.
    level: Level,
    /// Ticks so far.
    ticks: u32,
}

impl Terrain for Counting {
    fn solid(&self, r: Rect) -> bool {
        self.level.solid(r)
    }
    fn tick(&mut self, _dt: f32, _bodies: &[Rect]) {
        self.ticks += 1;
    }
    crate::terrain_layer!(level);
}

#[test]
fn a_layer_that_ticks_without_chaining_keeps_the_base_refills_coming_back() {
    let mut t = Counting {
        level: room(&[(10, 2, "*")]),
        ticks: 0,
    };
    let mut w = World::on(&t.level, Tuning::default());
    t.level.set_refill_back(1.0);
    let area = t.level.refills()[0].area;
    assert!(t.level.refill(area), "used up");
    for _ in 0..130 {
        w.step(&mut t, &Intent::default());
    }
    assert_eq!(t.ticks, 130, "its own tick ran");
    assert!(t.level.refills()[0].ready(), "and the level's");
    let found: &mut dyn Terrain = &mut t;
    assert!(found.find_mut::<Counting>().is_some(), "found by find_mut");
    assert!(found.find_mut::<Level>().is_some());
}

/// A layer whose base and base_mut disagree: a mistake.
struct Crossed {
    /// One level.
    a: Level,
    /// Another.
    b: Level,
}

impl Terrain for Crossed {
    fn solid(&self, r: Rect) -> bool {
        self.a.solid(r)
    }
    fn base(&self) -> Option<&dyn Terrain> {
        Some(&self.a)
    }
    fn base_mut(&mut self) -> Option<&mut dyn Terrain> {
        Some(&mut self.b)
    }
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "must return the same terrain")]
fn a_layer_whose_bases_disagree_is_caught_in_debug_builds() {
    let mut t = Crossed {
        a: Level::demo(),
        b: Level::demo(),
    };
    let mut w = World::on(&t.a, Tuning::default());
    w.step(&mut t, &Intent::default());
}
