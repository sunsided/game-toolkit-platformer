# Agent guide

**game-toolkit-platformer**: a pluggable platforming controller for
[game-toolkit](https://github.com/sunsided/game-toolkit) 0.10 (fluid, snappy movement with
presets tuned toward the feel of Celeste, Hollow Knight, Silksong and Super Meat Boy, plus an animation state
graph that follows it). The library is the product; `demo/` is its test bench. Keep using it
plug and play for game-toolkit users: `Platformer::on(&level, preset)`, `update(&input, &mut level, dt)`,
draw `sprite()` at `position()`. Design docs live in `docs/`; keep them in sync when behavior
changes (`docs/roadmap.md` tracks what's next). Keep the README short; details go in `docs/`.

## Layout

A Cargo workspace: the library at the root, the demo in `demo/`.

Library (`src/`, depends on nothing but std unless the `toolkit` feature is on, which is the
default):

- `sim/`: deterministic movement, no toolkit types. `World::step` at fixed `DT` (1/120 s);
  a cloned world plays out identically (`sim::tests` enforce it). Integer positions plus a
  sub-pixel remainder, moved one unit at a time (Celeste style). `sim/player.rs` runs the
  abilities in a fixed order; each ability is its own module in `sim/player/`. Every feel
  number and ability switch lives in `sim/tuning.rs` (`Tuning`, `Preset`). Movement emits
  `Event`s; nothing in `sim` knows about animation. `sim/level.rs` holds the demo level
  (keep its stairs within a plain held jump; a test checks). `sim/terrain.rs`: the `Terrain`
  trait the world steps through (only `solid` required; closures count; `Level` implements
  all of it, crystals and boxes included).
- `anim/`: the animation controller, independent of any game. `Params` (bools, floats,
  triggers with TTL), `Clip`, `Cond`, `GraphSpec` (by name, `Cow<'static, str>`) compiled
  into a validated `Graph`, `Animator`, `Driver`. Keep it free of `sim`/`hero` imports.
- `hero/`: the built-in character: sprite numbers, the graph spec, `HeroDriver` (sim facts
  and events to params; `try_new` reports a custom graph's missing parameter).
- `platformer.rs`: the `Platformer` facade and its `Builder` (preset, `tune`, tuning,
  controls, graph, spawn): fixed-step clock, press latching, event queues, interpolated
  position.
- `controls.rs` (feature `toolkit`): game-toolkit input to an `Intent` through a `Scheme`
  (`Scheme::SNAPPY`, `PRECISE`, `MOMENTUM`, or `Scheme::of(preset)`).
- `debug_draw.rs` (feature `toolkit`): `Platformer::draw_debug` with game-toolkit's painter.
- `examples/minimal.rs`: the smallest game (`task minimal`); keep it short.

Demo (`demo/src/`, re-exports the library's `sim`, `anim`, `hero` so its paths stay
`crate::sim::...`):

- `session.rs`: a game in progress (world + animator + clock + fx + wind + foliage + rig);
  toolkit-free, headless-testable. Sim, driver, animator run in that order per step.
- `hero/pose.rs` (what each sprite number means for the body) and `hero/rig.rs` (body
  frame; simulated cape, tail, mane and headband chains).
- `gfx/`: the 384x216 grid; draw through `Raster` only (`Canvas` CPU, `DrawList` GPU spans).
- `render/`: the moonlit backdrop and the plain diagnostic one (F3), the scene (scaled by
  `K`, pixels per unit, 1), the heroine (`mouse.rs`, a cut-out puppet after the Cheesed
  character sheet, which stays in pCloud), the debug blocks, the overlay.
- `app.rs` + `app/`: the demo's toolkit glue (window, debug keys, presenting, screenshots);
  movement input comes from the library's `Controls`.
- `demo/examples/snapshot.rs` (filmstrips, `task snapshot`) and `poses.rs` (`task poses`).

## Conventions

- One type per module; modules use `name.rs` + `name/`, never `mod.rs`.
- Other games (Celeste, Hollow Knight, Silksong, Super Meat Boy) appear only as comparisons
  in docs and comments ("tuned toward the feel of ..."), never in identifiers, preset or
  scheme names, or user-facing labels; the project is not affiliated with them.
- Doc comments on everything, private items included; `task check` must pass (it also
  lints the library without game-toolkit).
- Add a test for every movement rule change (`src/sim/tests.rs`), every controller rule
  change (`src/anim/*`), and every facade change (`src/platformer.rs`). README and lib docs
  code samples are doc tests; keep them compiling.
- Check animation with `task snapshot` before claiming a move looks right; check the real
  window with `task screenshot`.
