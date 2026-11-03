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

## Next

- [ ] Specs from data: serde + RON for `GraphSpec` and `Tuning`, hot reload while playing
- [ ] Real art: Aseprite sheets (toolkit `aseprite` feature), sprite numbers to cells,
  per-frame pivots and hitbox-independent offsets
- [ ] Layers: an upper-body / overlay layer (attacks over running), additive squash and
  stretch driven by landing impact
- [ ] Transition durations / cross-frames (hold-frame blends for pixel art)
- [ ] Markers to sound: footsteps, dash, landing thuds (synthesized like Starscorcher)
- [ ] An in-game graph inspector (current state, live transitions, why one didn't fire)
- [ ] Lab: record intents, rewind and replay (determinism already holds)
- [ ] More moves: ledge grab, crouch and slide, super/hyper dashes, wall-jump coyote time,
  swimming and underwater diving
- [ ] Camera with look-ahead and larger levels (looking up/down pans it, as in Hollow
  Knight)
- [ ] Carrying blocks, throwing them
- [ ] Super dash (Crystal Heart: charge on the ground or a wall, fly straight until
  stopped); Silksong's controls
- [ ] WASM build
