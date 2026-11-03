# animation

A pluggable controller for modern platformers that feel fluid and snappy: movement and animation as two cooperating systems, with a small game around them as the test bench.

![Scaling a single wall with stick-aimed wall jumps, Hollow Knight-like preset](docs/art/wall-scale.png)

A character that feels as good as the ones in Hollow Knight, Celeste or Super Meat Boy needs movement that is precise and forgiving, and animation that shows every one of its decisions without ever holding one up. The first part is well documented by now; the second usually ends up as a pile of conditionals in the player script. Here the animation is a state graph of named parameters, clips, conditions and priorities, compiled, validated, and run by an animator that doesn't know what it animates, and the movement is a deterministic simulation that only reports what happened.

It is written in Rust on [game-toolkit](https://github.com/sunsided/game-toolkit), but the two systems use neither the toolkit nor any other dependency, so they carry over to other engines.

## What's in it

| Layer | Where | What it does |
|---|---|---|
| Movement | `src/sim/` | A fixed-step (120 Hz), deterministic body: an `Intent` in per step, `Event`s out (jumped, landed, dashed, ...). All feel numbers and ability switches live in `Tuning`; `Preset`s set them toward one game each. |
| Animation controller | `src/anim/` | Generic: parameters, clips, conditions, a graph spec compiled into a validated graph, an `Animator`, and a `Driver` trait that feeds it. |
| Binding | `src/hero/` | The glue for one character: its graph (36 states), its sprite numbers and poses, and the driver that turns movement facts and events into graph parameters. |
| Test bench | `src/render/`, `src/app/` | The game: drawing, input, the debug overlay. Replace this part with your own. |

## Using it in your platformer

The crate isn't published; use it as a git dependency or copy `src/anim/` (and `src/sim/` if you want the movement too). Both compile on `std` alone.

### The animation controller

A graph is written by name and compiled; every name is checked, and a parameter used as the wrong kind fails here rather than on the first frame that needs it:

```rust
use animation::anim::{Animator, Clip, Cond, GraphSpec, Playback, StateSpec, TransitionSpec};

let graph = GraphSpec::new()
    .float("speed")
    .trigger("jump", 0.05)
    .state(StateSpec::new("idle", Clip::uniform(Playback::PingPong, &[0, 1], 0.5)))
    .state(
        StateSpec::new("run", Clip::uniform(Playback::Loop, &[2, 3, 4, 5], 0.08).marker(0, "step"))
            .rate_by("speed", 90.0, 0.4, 1.6),
    )
    .state(StateSpec::new("takeoff", Clip::uniform(Playback::Once, &[7], 0.06)).then("idle"))
    .transition(TransitionSpec::new(["idle"], "run", Cond::Above("speed".into(), 10.0)))
    .transition(TransitionSpec::new(["run"], "idle", Cond::Below("speed".into(), 10.0)))
    .transition(TransitionSpec::any("takeoff", Cond::Trigger("jump".into())).priority(70).breaks_lock())
    .compile()?;
let mut animator = Animator::new(Arc::new(graph));
```

Each step, set the parameters (or let a `Driver` do it), call `animator.update(dt)`, draw `animator.sprite()`, and take `animator.drain_events()` for markers like `"step"`. What it adds over a plain state machine:

- Triggers with a time to live: an event fired while a state is locked isn't lost; it waits until a transition consumes it or expires. Input queueing, on the animation side.
- Locks and priorities: a state can refuse all but lock-breaking transitions for a while, and the highest-priority transition wins, so an impact beats a landing.
- Clip rates and markers: a run cycle keeps pace with the speed, a climb stops when the body does, and markers drive footstep sounds, dust or anything else.
- Names as `Cow<'static, str>`: free for graphs written in code, owned for graphs built from data.

### The movement

```rust
use animation::sim::{Intent, Level, Preset, World};

let mut world = World::new(Level::demo(), Preset::Celeste.tuning());
world.step(&Intent { move_x: 1.0, jump_pressed: true, jump_held: true, ..Intent::default() });
for event in &world.events { /* Jumped, Landed, Dashed, RollStarted, ... */ }
```

Step it at a fixed `DT` and latch button presses between steps, so a press in a frame without a step isn't lost (`src/session.rs` does both). Your own level goes in as ASCII rows (`Level::parse`), your own feel as a `Tuning`.

### Connecting the two

The pattern that keeps the graph readable: events enter states through triggers, continuous facts keep or leave them through bools and floats. `src/hero/driver.rs` is a complete example, and the per-step loop is three lines:

```rust
world.step(&intent);
driver.drive(&world, &mut animator.params);
animator.update(DT);
```

To add a move: give the simulation a mode or rule and an `Event` (with a test), add a trigger or bool and a state with its transitions to the graph, map the event in the driver, and give the new sprite numbers a pose. `docs/animation.md` has the details.

### Your art

The graph only names sprite numbers. Draw them however you like: this repository draws a cut-out puppet (`src/render/mouse.rs`, with `src/hero/pose.rs` saying what each number means for the body); a sprite sheet would map each number to a cell instead.

## The moves

Everything the movement does, each tunable and each animated:

- jumping: a jump buffer, coyote time, variable height, a jump cut on release, hangtime at the apex, and a heavier fall,
- in the air: a double jump, a glide after it, 8-way dashes with recharge from the ground, walls or pickups, and a dive,
- on walls: sliding, sticking, stick-aimed wall jumps that can scale a single wall, and climbing with stamina and a firm or slipping grip,
- on the ground: running and sprinting, rolls out of high landings or a sprint, hard landings, a charged super dash, drop-through platforms, and looking up and down,
- with boxes: pushing and pulling, with a little static friction,
- details: corner correction and a skid when turning around.

| | |
|---|---|
| ![Super dash across the floor, ending in a bonk](docs/art/super-dash.png) | ![Glide after the double jump, Silksong-like preset](docs/art/glide.png) |
| ![Bouncing up the narrow chimney](docs/art/chimney-bounce.png) | ![Push, hold and pull a box](docs/art/push-pull.png) |

The pictures are filmstrips (`task snapshot`): a scripted move played headless, with the animation states it went through as the caption.

### Feel presets

Starting points for your own `Tuning`. They lean toward each game by feel; they are not measured copies.

| Preset | What it's like |
|---|---|
| hybrid (default) | everything on, Celeste-like numbers with Hollow Knight's heavier fall and jump cut |
| celeste-like | precise and floaty: 8-way dash, climbing with stamina, no air jump, no jump cut |
| hollow-knight-like | snappy: instant run, hard jump cut, double jump, sideways dash, super dash, slipping wall cling, dive |
| silksong-like | Hollow Knight's feel with a glide after the double jump |
| meat-boy-like | walk or hold to run, momentum, strong wall jumps, wall stick |

## Trying it

```sh
task run
```

Arrows, left stick or d-pad move. The buttons follow the preset (F2 switches both; the overlay's corner shows the keys):

| Action | Hollow Knight-like, Silksong-like, hybrid | Celeste-like | Meat Boy-like |
|---|---|---|---|
| jump (hold for height) | Z, Space / A | C, Space / A, Y | Space, Z / A |
| dash | C / RT | X / X, B | |
| super dash (hold, release) | S / LT | | |
| grab (hold) | D / LB | Z / RT, LT, RB, LB | |
| dive | down + A / down + B, down + RB | | |
| sprint / run (hold) | Shift / X | | Shift, X / X, RT |

The less obvious ones: roll by landing from 6 tiles or more while holding a direction (or by tapping down while sprinting); scale a wall by wall-jumping with the stick toward it; charge the super dash until she flashes, then let go; drop through a thin platform with down + jump; pull a box with grab held while walking away.

![The compact debug overlay](docs/art/overlay.png)

For inspecting the animation: F1 cycles the overlay (state, frame, lock, timers, active parameters, events), P pauses, `.` steps one 1/120 s frame, `[` and `]` change speed down to 1/16, and F3 switches to a diagnostic look (state-colored blocks on a plain backdrop).

## Building

```sh
task test         # movement rules, determinism, controller, hero graph, doc examples
task snapshot     # filmstrips of every scripted move into snapshots/
task poses        # the character in every sprite pose, one sheet
task check        # fmt + clippy, warnings as errors
```

You'll need Rust 1.95 or newer and [go-task](https://taskfile.dev); plain `cargo run` and `cargo test` work too.

## The test bench

The test character is the mouse heroine of my game Cheesed, a cut-out puppet with a simulated cape, tail and mane, in a moonlit scene with moss and grass that react to her.

## Current limitations

- Graphs and tunings are written in code; loading them from files (and hot-reloading) is next.
- No hand-drawn frames, no sound, no camera (a single screen), and no attack move yet.

The design notes are in [docs/](docs/vision.md): the vision, the movement rules, the animation graph, and the roadmap.

## License

EUPL-1.2, as declared in `Cargo.toml`.
