# animation

A small retro platformer built to grow a modern, pluggable and configurable animation
controller: jump queueing, coyote time, hangtime, jump cut, double jump (or Monarch-style
wings), land-and-roll, hard landings, 8-way dashes (up and down included) with recharge,
wall slide, wall stick, stick-aimed wall jumps that can scale a single wall, climbing with
stamina and a firm or slipping grip, dives, drop-through platforms, pushing and pulling
blocks, looking up and down, idle fidgets. Hollow Knight meets Celeste meets Super Meat
Boy.

Written in Rust on [game-toolkit](https://github.com/sunsided/game-toolkit). A 384x216
pixel grid, no art yet: the hero is drawn as placeholder poses, one per sprite number.

## Playing

Controls follow Hollow Knight's defaults; grab (not in Hollow Knight) takes the
super-dash slot.

| Action | Keyboard | Gamepad (Xbox names) |
|---|---|---|
| move, aim, climb, look | arrows | left stick, d-pad |
| jump (hold for height) | Z (also Space) | A |
| dash (8-way) | C | RT |
| grab walls, hold blocks | S (hold) | LT or LB (hold) |
| dive | down + A | down + B, down + RB |
| drop through a platform | down + jump | down + A |
| push a block | walk into it | |
| pull a block | grab + walk away | |

Wall jumps follow the stick: toward the wall kicks back onto it (chain them to scale a
wall), away leaps off, up and down tilt the jump.

Tuning: F2 cycles the feel presets (hybrid, celeste-like, hollow-knight-like,
meat-boy-like). Debug: F1 overlay (compact, full, off), P pause, `.` step, `[` `]`
slower/faster, `\` normal speed, R respawn, F12 screenshot, Esc quit.

## Building

```sh
task run          # play
task test         # movement rules, determinism, controller, hero graph
task snapshot     # filmstrips of every scripted move into snapshots/ (task snapshot -- wings)
task screenshot   # the real window after 2 s, into screenshots/
task check        # fmt + clippy, warnings as errors
```

## Design

See [docs/vision.md](docs/vision.md) and the documents it links.
