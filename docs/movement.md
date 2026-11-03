# Movement

All in `src/sim/`. Fixed step of 1/120 s, integer pixels with a sub-pixel remainder,
moved one pixel at a time against 8 px tiles (`#` solid, `=` one-way platform,
`*` dash crystal, `B` block). The hitbox is 6x12 by default (`Tuning::size`, read into
`Player::w`/`h`; the other numbers stay in units and don't scale with it), blocks 12x12.

## The demo level

One screen, laid out around the debug overlay's corners: stairs of 3-tile steps (each
within a plain held jump; `every_demo_stair_is_within_a_plain_jump` keeps it so) up to a
tower, two chimneys in the middle for wall-jump bouncing (24 and 40 px inside), a low
ceiling, a step to a ledge over the pit, a pillar to climb, crystals and two blocks.

## Step order

`Player::step` runs, in order: timers (buffers, coyote, refills on the ground, mode
clocks), dash, dive, climb, running, jump, falling, moving, landing. The order is part of
the feel: a buffered jump is checked before gravity, the landing after moving.

## Your own abilities

A game adds moves (an attack lunge, a ledge grab, a grapple, a pogo) as `sim::Ability`s
in `World::abilities` (or `Builder::ability`), not by forking `sim/player`:

- **When**: `before` runs ahead of the built-in moves each step (after the boxes are
  decided), `after` once they moved, collided and landed. Several abilities run in the
  order added.
- **What they touch**: the `Player`'s public state (speed, facing, refills, `launch`,
  `teleport`), the `Intent` (custom buttons in `custom`/`custom_pressed`, latched by the
  facade), the terrain and the tuning, read-only. They report `Event::Custom(n)`.
- **Modes**: `Mode::Custom { id, t }` takes the body over: no built-in move starts or runs
  (no running, gravity, jumps, dashes, climbing), but it still moves by its speed, collides,
  lands and rides moving ground, and `t` counts up. Setting `Mode::Normal` hands it back.
  Outside a custom mode, an ability works alongside the built-in moves (a pogo in
  `after`, say).
- **Determinism**: abilities are part of the world, cloned with it (`Clone` is required),
  so a cloned world plays out identically as long as they keep their state inside and
  read nothing from outside.

## Abilities

| Ability | Rule | Knobs |
|---|---|---|
| Run | accelerate toward the stick; faster turnaround; over top speed only bleed slowly (momentum after roll jumps and dashes) | `run_*`, `turn_accel`, `overspeed_decel`, `air_control` |
| Jump buffer | a press waits up to `jump_buffer` for a jump to become possible | `jump_buffer` |
| Coyote time | a ground jump still works shortly after leaving a ledge | `coyote_time` |
| Variable jump | holding jump keeps the full upward speed for `var_jump_time` | `var_jump_time` |
| Jump cut | releasing while rising multiplies the upward speed | `jump_cut` |
| Hangtime | with jump held near the apex, gravity is reduced | `apex_threshold`, `apex_gravity` |
| Heavy fall | more gravity falling; faster terminal speed with down | `fall_gravity`, `max_fall`, `fast_fall` |
| Double jump | air jumps, refilled on landing; falling just above ground, a press waits for the landing instead of spending one | `air_jumps`, `double_*`, `land_jump_lookahead` |
| Drop-through | down + jump on a one-way platform | `drop_through_time` |
| Corner correction | a head clipping a ceiling corner is nudged past it | `corner_correction` |
| Moving ground | after the terrain's update, standing on (or holding) something that moved carries the body along (`Terrain::carry`), stopping at walls and ceilings; a jump off keeps its smoothed speed for coyote time; a body left inside something solid reports `Squished` | `lift_boost` |
| Land-and-roll | a drop of `roll_drop`+ px while moving rolls; the roll keeps speed and cancels into a jump that keeps it too | `roll_*` |
| Hard landing | a drop of `hard_land_drop`+ px standing still costs control | `hard_land_*` |
| Queued jump vs. landing | a jump pressed before touching down wins over roll and hard landing | |
| Dash | 8-way (or sideways only), gravity off, hands over part of its speed; down-dashes end on the ground; refilled by the ground (after a moment) or crystals | `dash*`, `crystal_respawn` |
| Wall slide | pressing into a wall while falling caps the fall | `wall_slide_speed` |
| Wall refill | sliding down or holding on to a wall gives the air jumps back, and the dashes where the tuning says so (Hollow Knight; not Celeste) | `wall_refills_*` |
| Wall stick | pressing away from the wall slid down keeps the hero on it briefly (Super Meat Boy), so away + jump is a wall jump | `wall_stick_time` |
| Wall jump | near a wall (or climbing and pushing away). The stick aims it: toward the wall a small kick with a short lock that arcs back onto the same wall (chained, it scales the wall, as in Hollow Knight and Super Meat Boy), away the full leap, neutral in between; up and down tilt it ±`wall_jump_aim` | `wall_jump_*`, `wall_climb_*` |
| Climb | grab a wall: climb up/down at a stamina cost, hop over the top; climb jump straight up | `grab`, `climb_*`, `stamina` |
| Grip | holding still: a firm grip holds, a slipping one slides; a tired grip (low stamina) slips and can't climb | `slip_speed`, `tired_*` |
| Dive | in the air: a short hang, then straight down; the impact costs a moment of control | `dive`, `dive_*` |
| Super dash | Crystal Heart: hold the button on the ground or on a wall (sliding or climbing) to charge; released once charged, the hero flies straight and level, gravity off, away from a wall it charged on, until a wall (a bonk: knocked back, falling) or a cancel (jump or the button; keeps `super_cancel_speed`). Released early or jumped out of, the charge does nothing | `super_dash`, `super_*` |
| Glide | a jump pressed in the air (not off a wall) and held opens a cloak once falling: the fall is capped at a slow drift. The press spends the air jump first if one is left, as in Silksong (Faydown Cloak, then Drifter's Cloak) | `glide`, `glide_fall_speed` |
| Sprint | holding sprint raises the top speed; the extra speed carries into jumps (it only bleeds off slowly) | `sprint`, `sprint_speed` |
| Run-and-roll | a fresh tap of down while faster than walking (sprinting) on the ground rolls, like a landing roll | `run_roll`, `run_roll_speed` |
| Blocks | walking into a resting block pushes it (slowly); grab facing it holds it, walking away pulls it; a block sticks until the hero has strained for `push_breakaway`, then speeds up over `push_ramp` (weight); blocks fall, stack, and can be stood on | `push`, `push_*`, `pull_speed` |
| Look | holding up or down while standing still (no movement) | |

"Diving" is the Hollow Knight downward plunge here; swimming is a different feature (see
the [roadmap](roadmap.md)).

## Tuning

`Tuning` holds every number and ability switch; px and seconds. One field is
presentation, not movement: `wings` tells the hero's animation driver to show air jumps as
wings rather than a flip. Abilities are off with a
zero count (`air_jumps`, `dashes`), a false flag (`grab`, `dive`, `dash_eight_way`) or an
unreachable drop (`roll_drop`, `hard_land_drop`).

## Presets

`Preset::tuning()`; F2 in game cycles them, together with the matching control scheme
(`app/controls.rs`, the README lists them).

| Preset | Character |
|---|---|
| hybrid | everything on (default) |
| precise (toward Celeste) | no air jump, no cut, no dive, no roll; climb, 8-way dash |
| snappy (toward Hollow Knight) | instant run, hard cut, double jump shown as wings, sideways dash, super dash, cling-and-slide (no grab), dive, hard landings |
| drifter (toward Silksong) | Hollow Knight's feel with a glide after the double jump (flip, not wings); no dive |
| momentum (toward Super Meat Boy) | walk, hold to run (fast, momentum), strong wall jumps, wall stick; no dash, no double, no climb |

These lean toward each game's feel; they are not measured copies of it.
