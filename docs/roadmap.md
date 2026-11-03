# Roadmap

## Done (prototype)

- [x] Technical setup after Starscorcher: toolkit-free core, `Raster` grid, Taskfile, lints
- [x] Deterministic movement with every ability in [movement](movement.md)
- [x] Feel presets (hybrid, Celeste-, Hollow-Knight-, Meat-Boy-like)
- [x] Animation controller: params with trigger TTL, clips with markers, condition trees,
  named spec compiled and validated, locks, priorities, driver trait
- [x] Hero graph and driver; placeholder poses; debug overlay; slow motion and stepping
- [x] Headless filmstrips per move
- [x] Demo level around the overlay: reachable stairs, chimneys for wall bouncing
- [x] Hollow Knight controller mapping; look up/down; idle fidget; Monarch-style wings
- [x] Blocks: push, pull, hold, fall, stand on
- [x] Stick-aimed wall jumps; scaling a single wall
- [x] Compact overlay (F1: compact, full, off)
- [x] Walls refill air jumps (and dashes per preset); blocks with weight and rumble
- [x] Control schemes per preset; Silksong-like preset with glide; run-and-roll
- [x] Sprint button (Super Meat Boy's run)
- [x] Super dash (Crystal Heart); gamepad triggers read as axes too
- [x] The Cheesed heroine as a cut-out puppet with simulated cape, tail and mane; cheese
  wedges for crystals; `task poses`
- [x] Her bearing from the paintings (stance, wild mane, sword-raise fidget), a readable
  super dash charge, the moonlit scene; F3 for the diagnostic look
- [x] Wind for mane, headband and cape: height, facing, gusts
- [x] Twinkling stars; moss and grass that sway in the wind and react to her (footstep
  markers, landings, dashes)

- [x] Library and demo split: `game-toolkit-platformer` with a `Platformer` facade, a
  builder, control schemes per preset, a debug draw, a minimal example
- [x] Plug-and-play gaps: launch/teleport/respawn, spawning on any terrain, custom buttons,
  boxes on any terrain, custom tiles, a hazards recipe, custom drivers and partial graphs,
  a configurable hitbox, moving platforms (with squish), ability hooks, several characters
  (`Party`)
- [x] Leaner core: terrains build on a `base` (only what changes is written), abilities
  change the terrain and rule out single built-in moves, boxes moved into the demo as an
  ability, a lift in the demo and a moving-platforms recipe, `Builder::scale` for bigger
  tiles, facade accessors, `Platformer::standing`

## Next

- [ ] Specs from data: serde + RON for `GraphSpec` and `Tuning`, hot reload while playing
- [ ] A sprite sheet adapter: sprite numbers to Aseprite tags, drawn with game-toolkit
- [ ] Hand-drawn frames for the heroine: Aseprite sheets (toolkit `aseprite` feature),
  sprite numbers to cells; her expressions as a portrait; an attack for the sword
- [ ] Layers: an upper-body / overlay layer (attacks over running), additive squash and
  stretch driven by landing impact
- [ ] Transition durations / cross-frames (hold-frame blends for pixel art)
- [ ] Markers to sound: footsteps, dash, landing thuds (synthesized like Starscorcher)
- [ ] An in-game graph inspector (current state, live transitions, why one didn't fire)
- [ ] Lab: record intents, rewind and replay (determinism already holds)
- [ ] The demo's boxes in a `Party`: one grip per stage, and a shoved box is stopped by
  the terrain and the shover only
- [ ] More moves: ledge grab, crouch and slide (`Player::h` can change; check room to stand), super/hyper dashes, wall-jump coyote time,
  swimming and underwater diving
- [ ] Camera with look-ahead and larger levels (looking up/down pans it, as in Hollow
  Knight)
- [ ] Carrying blocks, throwing them
- [ ] Silksong's own controls
- [ ] WASM build
