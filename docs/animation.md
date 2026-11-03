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

- Standing: `idle` (ping-pong breathing), `idle_fidget` (after 5 s still, once, back to
  idle), `look_up` / `look_down` (after holding up or down for 0.15 s).
- Ground: `run` (rate by speed, `step` markers), `sprint` (above 110 px/s, back under 100),
  `skid`, `land` (once, into idle or run),
  `hard_land` (locked), `roll`.
- Blocks: `push`, `pull` (rate by speed, `step` markers), `box_hold`.
- Air: `takeoff` → `rise` → `apex` → `fall`; one-shots `double` (locked flip) or `wings`
  (chosen by the `wings` bool: the same trigger, two looks), `wall_jump`, `hop`, `drop`;
  `glide` (any air state while `gliding`).
- Super dash: `super_charge` → `super_ready` (once `charged`) → `super_dash` (on the
  `super_launch` trigger) → `super_bonk` (on a wall) → `fall`.
- Dashes: `dash_side`, `dash_up`, `dash_down`, chosen by `dash_y`.
- Walls: `climb` (rate by vertical speed) ↔ `grip_slip`, `wall_slide`.
- Dive: `dive_windup` → `dive` → `dive_land` (locked).

Priorities, highest first: dive impact, dive, roll / hard landing, dashes, air jump and
wall jump, grab, jump / hop / drop, landing, leaving continuous states, walls, ground, the
air arc, and a low-priority net that settles air states on the ground.

## Drawing

The graph only names sprite numbers. `hero/pose.rs` says what each means for the body
(size, lean, lift); `render/mouse.rs` draws the heroine as a cut-out puppet in that body
frame (at `render::K` pixels per unit, 1 by default), and `hero/rig.rs` simulates her cape, tail, mane and headband ends as short Verlet
chains with gravity, air drag from moving, a rest shape (the tail curls) and the
wind (`session/wind.rs`: a breeze, stronger with height, that streams them out behind
her either way, and gusts from three slow waves that blow in their real direction);
they stay behind her body and above the floor. Hand-drawn
frames would replace `mouse.rs` and keep everything else.

## Markers driving the world

The run, sprint, push and pull clips carry `step` markers; the session turns each one
into a small shove on the grass under her feet (`session/foliage.rs`). Events from the
simulation (landings, rolls, dashes, dives, the super dash) shove the plants harder.
Sounds would hang off the same markers.

## Adding a move

1. Sim: a mode or rule, an `Event`, a test.
2. Spec: a trigger or bool, a state with its clip, transitions.
3. Driver: map the event or fact to the parameter.
4. Sprites: new numbers in `hero/sprite.rs`, a pose in `hero/pose.rs`
   (`session::tests` fails if a sprite is never shown).
5. A script in `examples/snapshot.rs`, then `task snapshot`.
