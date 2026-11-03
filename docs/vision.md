# Vision

The game is the test bed; the animation controller is the product.

A character controller that feels as good as Celeste's, Hollow Knight's or Super Meat
Boy's is two systems: movement that is precise and forgiving, and animation that reads
every one of its decisions without ever delaying them. The second is usually a pile of
`if`s in the player script. Here it is a state graph that is data: named parameters, clips,
conditions and priorities, compiled and validated, run by a generic animator, and fed by a
driver that is the only code knowing both sides.

## Principles

- **Movement decides, animation follows.** Input never waits for an animation. A jump
  happens the step it is pressed (or the step it becomes possible); the animation catches
  up in the same step. Locks in the animation never lock the body.
- **Deterministic all the way.** Simulation and animator advance in the same fixed step,
  so a replay replays the frames too.
- **Configurable, not coded.** Feel is [`Tuning`](movement.md#tuning) and
  [presets](movement.md#presets); animation is a [`GraphSpec`](animation.md).
- **Pluggable.** The controller knows nothing of this game; the hero is one binding.
- **Visible.** The debug overlay shows state, frame, locks, history, every parameter and
  pending trigger, the body's buffers and timers; slow motion down to 1/16 and single
  steps; headless filmstrips for every move.

## References

- *Celeste*: jump buffer, coyote time, variable jump, half gravity at the apex, 8-way
  dash with recharge, climbing with stamina, corner correction.
- *Hollow Knight*: hard jump cut, double jump, sideways dash, wall cling that slips,
  dive, hard landings.
- *Super Meat Boy*: momentum running, strong wall jumps, wall stick.

## Documents

- [Movement](movement.md): every ability, the step order, tuning and presets.
- [Animation](animation.md): the controller and the hero's graph.
- [Roadmap](roadmap.md).
