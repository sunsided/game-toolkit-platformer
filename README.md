# animation

A small retro platformer built to grow a modern, pluggable and configurable animation
controller: jump queueing, coyote time, hangtime, jump cut, double jump, land-and-roll,
hard landings, 8-way dashes (up and down included) with recharge, wall slide, wall stick,
wall jumps, climbing with stamina and a firm or slipping grip, dives, drop-through
platforms. Hollow Knight meets Celeste meets Super Meat Boy.

Written in Rust on [game-toolkit](https://github.com/sunsided/game-toolkit). A 384x216
pixel grid, no art yet: the hero is drawn as placeholder poses, one per sprite number.

## Playing

| Action | Keyboard | Gamepad |
|---|---|---|
| move, aim, climb | arrows / WASD | left stick, d-pad |
| jump (hold for height) | Space, C, K | A |
| dash (8-way) | X, J | X |
| grab (hold) | Z, L, Shift | right shoulder / trigger |
| dive (in the air) | V, I | B |
| drop through a platform | down + jump | down + A |

Tuning: F2 cycles the feel presets (hybrid, celeste-like, hollow-knight-like,
meat-boy-like). Debug: F1 overlay, P pause, `.` step, `[` `]` slower/faster, `\` normal
speed, R respawn, F12 screenshot, Esc quit.

## Building

```sh
task run          # play
task test         # movement rules, determinism, controller, hero graph
task snapshot     # filmstrips of every scripted move into snapshots/
task screenshot   # the real window after 2 s, into screenshots/
task check        # fmt + clippy, warnings as errors
```

## Design

See [docs/vision.md](docs/vision.md) and the documents it links.
