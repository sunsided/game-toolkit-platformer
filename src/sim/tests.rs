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
    assert!(!w.level.crystals[0].ready());
    run(&mut w, Intent::default(), 400);
    assert!(w.level.crystals[0].ready());
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

/// A room with a block on the floor at tile x 6, the hero standing left of it.
fn with_block() -> Sim {
    let mut w = world(room(&[(14, 6, "B")]));
    w.player.x = 6 * TILE - 2 - player::W - 4;
    run(&mut w, Intent::default(), 3);
    w
}

#[test]
fn walking_into_a_block_pushes_it_slowly() {
    let mut w = with_block();
    let start = w.level.blocks[0].x;
    run(
        &mut w,
        Intent {
            move_x: 1.0,
            ..Intent::default()
        },
        120,
    );
    let moved = w.level.blocks[0].x - start;
    assert!(moved > 20, "pushed {moved}");
    assert_eq!(w.player.shove, Shove::Push { side: 1 });
    assert!(w.player.vx <= Tuning::default().push_speed + 0.5);
    assert!(moved < 60, "slower than running: {moved}");
}

#[test]
fn grabbing_a_block_holds_it_and_walking_away_pulls_it() {
    let mut w = with_block();
    run(
        &mut w,
        Intent {
            move_x: 1.0,
            ..Intent::default()
        },
        20,
    );
    let start = w.level.blocks[0].x;
    let grab = Intent {
        grab: true,
        ..Intent::default()
    };
    run(&mut w, grab, 5);
    assert_eq!(w.player.shove, Shove::Hold { side: 1 });
    assert_eq!(
        w.player.mode,
        Mode::Normal,
        "grab at a block doesn't climb it"
    );
    run(
        &mut w,
        Intent {
            move_x: -1.0,
            ..grab
        },
        120,
    );
    assert_eq!(w.player.shove, Shove::Pull { side: 1 });
    assert_eq!(w.player.facing, 1, "still facing the block");
    assert!(
        w.level.blocks[0].x < start - 12,
        "pulled from {start} to {}",
        w.level.blocks[0].x
    );
    assert!(w.player.wall(&w.level, 1), "the block stays at the hand");
}

#[test]
fn blocks_fall_off_ledges_and_can_be_stood_on() {
    // A block on a shelf edge; push it off, then jump onto it.
    let mut w = world(room(&[(10, 1, "######"), (9, 5, "B")]));
    w.player.x = 2 * TILE;
    w.player.y = 10 * TILE - player::H;
    run(
        &mut w,
        Intent {
            move_x: 1.0,
            ..Intent::default()
        },
        200,
    );
    assert!(w.level.blocks[0].grounded);
    assert_eq!(w.level.blocks[0].y, 15 * TILE - block::SIZE, "on the floor");
    let mut w = world(room(&[(14, 4, "B")]));
    jump(&mut w, 30, 0);
    run(
        &mut w,
        Intent {
            move_x: 1.0,
            ..Intent::default()
        },
        18,
    );
    run(&mut w, Intent::default(), 90);
    assert_eq!(
        w.player.y + player::H,
        w.level.blocks[0].y,
        "standing on the block"
    );
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
fn blocks_stick_before_they_budge() {
    let mut w = with_block();
    // Walk up to it, until the first step of pushing.
    while !matches!(w.player.shove, Shove::Push { .. }) {
        w.step(&Intent {
            move_x: 1.0,
            ..Intent::default()
        });
    }
    let start = w.level.blocks[0].x;
    let mut pushed = 0;
    for _ in 0..24 {
        // Under the 0.25 s breakaway: strained, not moved.
        w.step(&Intent {
            move_x: 1.0,
            ..Intent::default()
        });
        pushed = w.level.blocks[0].x - start;
    }
    let t = Tuning::default();
    assert!(w.effort > 0.0 && w.effort < t.push_breakaway + 0.02);
    assert!(pushed <= 1, "budged early: {pushed}");
    run(
        &mut w,
        Intent {
            move_x: 1.0,
            ..Intent::default()
        },
        90,
    );
    assert!(w.level.blocks[0].x - start > 15, "then it slides");
    // Letting go resets the effort: it sticks again.
    run(&mut w, Intent::default(), 2);
    assert_eq!(w.effort, 0.0);
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
fn a_block_rumbles_while_it_sticks() {
    let mut w = with_block();
    while !matches!(w.player.shove, Shove::Push { .. }) {
        w.step(&Intent {
            move_x: 1.0,
            ..Intent::default()
        });
    }
    assert_eq!(w.straining(), Some(0));
    run(
        &mut w,
        Intent {
            move_x: 1.0,
            ..Intent::default()
        },
        40,
    );
    assert_eq!(w.straining(), None, "it budged");
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
