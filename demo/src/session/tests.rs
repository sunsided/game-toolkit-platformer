//! Session tests: the hero graph compiles, and the animation follows the movement.

use super::*;

/// The states entered while stepping `n` times with `i`.
fn states(s: &mut Session, i: Intent, n: usize) -> Vec<String> {
    let mut out = Vec::new();
    for _ in 0..n {
        s.step(&i);
        let name = s.animator.state_name().to_string();
        if out.last() != Some(&name) {
            out.push(name);
        }
    }
    out
}

#[test]
fn a_jump_animates_takeoff_to_landing() {
    let mut s = Session::new();
    states(&mut s, Intent::default(), 10);
    let mut seen = states(
        &mut s,
        Intent {
            jump_pressed: true,
            jump_held: true,
            ..Intent::default()
        },
        1,
    );
    seen.extend(states(
        &mut s,
        Intent {
            jump_held: true,
            ..Intent::default()
        },
        20,
    ));
    seen.extend(states(&mut s, Intent::default(), 120));
    let want = ["takeoff", "rise", "apex", "fall", "land", "idle"];
    let mut it = seen.iter();
    for w in want {
        assert!(it.any(|s| s == w), "missing {w} in order in {seen:?}");
    }
}

#[test]
fn running_turns_skid_and_dashes_animate_by_direction() {
    let mut s = Session::new();
    states(
        &mut s,
        Intent {
            move_x: 1.0,
            ..Intent::default()
        },
        60,
    );
    assert_eq!(s.animator.state_name(), "run");
    let seen = states(
        &mut s,
        Intent {
            move_x: -1.0,
            ..Intent::default()
        },
        20,
    );
    assert!(seen.iter().any(|x| x == "skid"), "{seen:?}");
    let seen = states(
        &mut s,
        Intent {
            move_y: -1.0,
            dash_pressed: true,
            ..Intent::default()
        },
        1,
    );
    assert_eq!(seen, ["dash_up"]);
}

#[test]
fn slow_motion_does_not_drop_presses() {
    let mut s = Session::new();
    s.update(0.1, Intent::default(), &Keys::default());
    for _ in 0..4 {
        s.clock.change(true, false, false);
    }
    // A press in a frame too short for a step at 1/16 speed.
    s.update(
        0.001,
        Intent {
            jump_pressed: true,
            jump_held: true,
            ..Intent::default()
        },
        &Keys::default(),
    );
    let mut jumped = false;
    for _ in 0..200 {
        s.update(
            1.0 / 60.0,
            Intent {
                jump_held: true,
                ..Intent::default()
            },
            &Keys::default(),
        );
        jumped |= !s.world.player.grounded();
    }
    assert!(jumped);
}

#[test]
fn every_hero_sprite_has_a_state_that_shows_it() {
    let g = crate::hero::graph();
    let shown: std::collections::HashSet<u16> = g
        .states
        .iter()
        .flat_map(|st| st.clip.frames.iter().map(|f| f.sprite))
        .collect();
    for sprite in 0..crate::hero::sprite::COUNT {
        assert!(shown.contains(&sprite), "sprite {sprite} is never shown");
    }
}

#[test]
fn running_into_a_wall_settles_instead_of_restarting_the_run() {
    let mut s = Session::new();
    let left = Intent {
        move_x: -1.0,
        ..Intent::default()
    };
    // From the spawn to the level's left wall, and on into it.
    states(&mut s, left, 240);
    assert!(s.world.player.grounded() && s.world.player.wall(&s.stage, -1));
    // Long enough for an idle fidget: she keeps pushing against it instead.
    let seen = states(&mut s, left, 1200);
    assert_eq!(seen, ["push"], "one state against the wall");
    // Letting go: standing again.
    let seen = states(&mut s, Intent::default(), 30);
    assert_eq!(seen.last().map(String::as_str), Some("idle"), "{seen:?}");
}
