# Animation

All in `src/anim/`; the hero's binding in `src/hero/`.

## The controller

- **Parameters** (`Params`), declared by name: `Bool` and `Float` (the driver sets them
  every step) and `Trigger { ttl }` (fired on events). A fired trigger stays pending until
  a transition consumes it or `ttl` runs out; `ttl = 0` lives for exactly one update. That
  is input queueing on the animation side: an event during a locked animation is not lost.
- **Clips** (`Clip`): frames of `(sprite, seconds)`, `Loop`, `Once` (holds the last frame,
  reports finished) or `PingPong`; named markers (`"step"`) reported when their frame
  starts, for sounds and effects.
- **States**: a clip, a rate (fixed, or scaled by a float parameter: run cycles keep pace
  with speed, climbing stops when the hero does), a `lock` (seconds during which only
  lock-breaking transitions may leave), and `next` (where to go when the clip finishes).
- **Transitions**: from a list of states or from any state (with exceptions), a condition
  (`Cond`: `Is`, `Trigger`, `Above`, `Below`, `Finished`, `After`, `Not`, `All`, `Any`), a
  priority, `breaks_lock`, `restart`.
- **Spec and graph**: `GraphSpec` names everything (`Cow<'static, str>`: free for literals,
  owned when built from data) and `compile()`s into a `Graph` of ids, failing on unknown
  names, duplicates and parameters used as the wrong kind. Transitions are sorted by
  priority once.
- **Animator**: per update, advance the clip, take the first applicable transition whose
  condition holds (consuming the triggers it tested), otherwise follow `next` if the clip
  finished, then age the triggers. It reports `Entered`, `Marker` and `Finished` events and
  keeps a short history for the overlay.
- **Driver**: `Driver<S>` writes parameters from a subject; any `FnMut(&S, &mut Params)` is
  one.

## The hero's graph

Pattern: events enter states through triggers; continuous facts keep or leave them
through bools and floats.

- Ground: `idle`, `run` (rate by speed, `step` markers), `skid`, `land` (once, into idle
  or run), `hard_land` (locked), `roll`.
- Air: `takeoff` → `rise` → `apex` → `fall`; one-shots `double` (locked flip), `wall_jump`,
  `hop`, `drop`.
- Dashes: `dash_side`, `dash_up`, `dash_down`, chosen by `dash_y`.
- Walls: `climb` (rate by vertical speed) ↔ `grip_slip`, `wall_slide`.
- Dive: `dive_windup` → `dive` → `dive_land` (locked).

Priorities, highest first: dive impact, dive, roll / hard landing, dashes, air jump and
wall jump, grab, jump / hop / drop, landing, leaving continuous states, walls, ground, the
air arc, and a low-priority net that settles air states on the ground.

## Adding a move

1. Sim: a mode or rule, an `Event`, a test.
2. Spec: a trigger or bool, a state with its clip, transitions.
3. Driver: map the event or fact to the parameter.
4. Sprites: new numbers in `hero/sprite.rs`, a pose in `render/hero.rs`
   (`session::tests` fails if a sprite is never shown).
5. A script in `examples/snapshot.rs`, then `task snapshot`.
