# Agent guide

**animation**: a small retro platformer whose real subject is a pluggable, configurable
animation controller (Hollow Knight meets Celeste meets Super Meat Boy). Rust on
[game-toolkit](https://github.com/sunsided/game-toolkit) 0.10; technical setup follows
Starscorcher (toolkit-free core, one glue module, pixel grid through `Raster`, Taskfile,
strict lints). Design docs live in `docs/`; keep them in sync when behavior changes
(`docs/roadmap.md` tracks what's next).

## Layout

- `src/sim/`: deterministic simulation, no toolkit types. `World::step` at fixed `DT`
  (1/120 s); a cloned world plays out identically (`sim::tests` enforce it). Integer pixel
  positions + sub-pixel remainder, moved one pixel at a time (Celeste style).
  `sim/player.rs` runs the abilities in a fixed order; each ability is its own module in
  `sim/player/` (`jump`, `dash`, `climb`, `dive`, `land`, `collide`). Every feel number
  and ability switch lives in `sim/tuning.rs` (`Tuning`, `Preset`). Movement emits `Event`s;
  nothing in `sim` knows about animation.
- `src/anim/`: the animation controller, independent of this game. `Params` (bools,
  floats, triggers with TTL), `Clip` (frames, playback, markers), `Cond` (condition tree),
  `GraphSpec` (everything by name, `Cow<'static, str>`) compiled and validated into a
  `Graph` (everything by id), `Animator` (runs a graph), `Driver` (subject to params).
  Keep it free of `sim`/`hero` imports.
- `src/hero/`: the hero's binding: sprite numbers, its graph spec, `HeroDriver`
  (sim facts and events to params), the pose table (`pose.rs`: what each sprite number
  means for the body) and the rig (`rig.rs`: body frame, simulated cape, tail, mane and
  headband chains, stepped in the session). A new move = sim event/mode, a driver line, states
  and transitions here, a pose in `render/hero.rs`.
- `src/session.rs`: a game in progress (world + animator + clock + fx). Toolkit-free: takes
  an `Intent` + `dt`. Latches presses between fixed steps so slow motion never drops one.
  Sim, driver, animator run in that order per step.
- `src/gfx/`: the 384x216 grid (8 px tiles, 5x = 1080p). Draw through `Raster` only.
  `Canvas` = CPU image (backdrop, snapshots), `DrawList` = spans the app turns into GPU
  rects. Copied from Starscorcher.
- `src/render/`: backdrop, scene, the heroine (`mouse.rs`: a cut-out puppet after the
  Cheesed character sheet, kept outside the repo in pCloud), the debug-block look
  (`hero.rs`, F3), the debug overlay (`debug.rs`). `task poses` renders every sprite.
- `src/app.rs` + `src/app/`: the only toolkit glue: input (keyboard + gamepad, Hollow
  Knight's mapping; `app/controls.rs`), presenting, screenshots.
- `src/sim/level.rs`: the demo level sits around the overlay's corners; keep stairs within
  a plain held jump (a test checks the left stairs).
- `examples/snapshot.rs`: headless filmstrips of scripted moves (`task snapshot`). To look
  at a move, add a script there.

## Conventions

- One type per module; modules use `name.rs` + `name/`, never `mod.rs`.
- Doc comments on everything, private items included (`missing_docs` and
  `clippy::missing_docs_in_private_items` are on); `task check` must pass.
- Add a test for every movement rule change (`src/sim/tests.rs`) and every controller
  rule change (`src/anim/*` tests). `session::tests` checks the hero graph against the sim.
- Check animation with `task snapshot` (filmstrips with the state sequence as caption)
  before claiming a move looks right; check the real window with `task screenshot`.
