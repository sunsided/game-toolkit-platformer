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

Movement is always the arrows, the left stick or the d-pad. The buttons follow the feel
preset (F2 cycles them, the overlay's bottom-right corner shows the keys):

| Action | Hollow Knight-like, hybrid | Celeste-like | Meat Boy-like |
|---|---|---|---|
| jump (hold for height) | Z, Space / A | C, Space / A, Y | Space, Z / A |
| dash | C / RT | X / X, B | |
| grab (hold) | S / LT, LB | Z / RT, LT, RB, LB | |
| dive | down + A / down + B, down + RB | | |

Keyboard / gamepad (Xbox names). Hollow Knight has no grab, so it takes the super-dash
slot; its dive is down + cast, as Desolate Dive is.

### How to

- **Variable jump, hangtime**: hold jump to go higher; keep holding over the top to float
  a little longer. Let go early for a short hop.
- **Queued jump**: press jump a moment before landing; the hero jumps the instant it lands.
  **Coyote time**: you can still jump a moment after running off a ledge.
- **Double jump**: jump again in the air (wings in the Hollow Knight preset). A wall gives
  it back.
- **Roll**: drop from 6 tiles (48 px) or more and keep holding left or right as you land.
  Jump during the roll to keep its speed. The tower on the left or the chimney lids are
  high enough.
- **Hard landing**: drop from 12 tiles (96 px) or more without holding a direction.
  Pressing jump just before landing skips both roll and hard landing.
- **Dash**: dash with a direction (eight ways, up and down too; with none, the way you
  face). The ground, a wall (not in Celeste) or a crystal gives it back.
- **Wall slide, wall jump**: hold toward a wall while falling to slide; jump to kick off.
  The stick aims it: toward the wall kicks back onto it higher up (repeat to climb a wall,
  the chimneys are for that), away leaps off, up and down tilt it.
- **Climb**: hold grab against a wall; up and down climb, stamina runs out; climbing past
  the top hops over. A tired grip slips.
- **Dive**: down + dive in the air: a short hang, then straight down.
- **Drop through**: down + jump on a thin platform.
- **Push and pull**: walk into a box to push it (it takes a moment to budge); hold grab
  facing it and walk away to pull it.
- **Look up and down**: hold up or down while standing still.

Debug: F1 overlay (compact, full, off), F2 next preset, P pause, `.` step, `[` `]`
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
