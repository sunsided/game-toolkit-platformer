# Hazards, triggers and checkpoints

The controller moves the character; what hurts it, what it picks up and where it comes
back is your game's. The pattern: after each `update` (or `advance`), test the
character's `hitboxes()` against your things, then act through the facade.

| You want | Test | Then |
|---|---|---|
| Spikes, lava, a kill plane | a hitbox overlaps it | `respawn()` (a fresh body at the spawn point, an `Event::Respawned`) |
| Knockback | a hitbox overlaps an enemy | `launch(vx, vy)`: away from it and up; it isn't cut by releasing jump |
| A spring, a bounce pad | a hitbox overlaps it | `launch(0.0, -speed)`, or make it bouncy ground ([surfaces](surfaces.md)) |
| A checkpoint | a hitbox overlaps it | `set_spawn(Spot::Feet(x, floor_y))` |
| A pickup | a hitbox overlaps it | remove it; `refill()` for dashes, air jumps and stamina |
| A door, a portal | the hitbox overlaps it and a custom button was pressed | `teleport(spot)` |
| A crushing platform | `Event::Squished` (something solid moved into the character) | `respawn()` |
| A pit | `Terrain::fell_out` says so | nothing: the character respawns by itself, or with `respawn(false)` on the builder you get `Event::FellOut` and decide |

On a `Level`, mark these with tiles of your own: `Level::parse_with` reads
`Tile::Custom(n)` from characters you choose, the movement treats them as air, and
`level.touches(hero.hitboxes(), tile)` gives the first hitbox of the last frame that
touches one (`Level::tiles_touching(hitbox)` lists them all). On a terrain of your own,
keep them however you like and test rects with `hero.touched(rect)`. Both return a copy,
so you can call `respawn()` or `launch()` right away.

Dash refill crystals are the one pickup the terrain does handle (`Terrain::refill`):
the movement decides when one is used (only when a dash has been spent), so it asks.
Everything else is yours, as above.

## Every step, not every frame

The movement runs in fixed steps of 1/120 s, and a frame takes as many as its time
covers (up to 8 at 15 frames per second). `hitboxes()` has the hitbox after each step of
the last frame, `hitbox()` only the last one: test every one, or a spike touched in one
step and left by the next goes unseen, and how often depends on the frame rate.

Test after the update, not before: the events of those steps (`drain_events()`) are
still waiting, so a landing on a spring and its `Event::Landed` arrive together.

For a hazard that must react within the step (stop the character in the step it touches
lava, before the next step moves it on), write an `Ability`: its `after` hook runs every
step and reaches your terrain through `find_mut`.

## Example

Spikes (`^`), a checkpoint (`C`) and a spring (`S`) on a `Level`:

```rust
use game_toolkit_platformer::prelude::*;

const SPIKES: u8 = 0;
const CHECKPOINT: u8 = 1;
const SPRING: u8 = 2;

let mut level = Level::parse_with(
    &[
        "..........................",
        "..........................",
        "..........................",
        "..........................",
        ".P......C.....^^......S...",
        "##########################",
    ],
    |c| match c {
        '^' => Some(Tile::Custom(SPIKES)),
        'C' => Some(Tile::Custom(CHECKPOINT)),
        'S' => Some(Tile::Custom(SPRING)),
        _ => None,
    },
);
let mut hero = Platformer::on(&level, Preset::Hybrid);
let start = hero.hitbox();

let mut respawned = false;
for _ in 0..120 {
    hero.advance(Intent { move_x: 1.0, ..Intent::default() }, &mut level, 1.0 / 60.0);
    if level.touches(hero.hitboxes(), Tile::Custom(SPIKES)).is_some() {
        hero.respawn();
        respawned = true;
        break;
    }
    if let Some(r) = level.touches(hero.hitboxes(), Tile::Custom(CHECKPOINT)) {
        hero.set_spawn(Spot::Feet(r.x + r.w / 2, r.bottom()));
    }
    if level.touches(hero.hitboxes(), Tile::Custom(SPRING)).is_some() {
        hero.launch(0.0, -320.0);
    }
}
assert!(respawned, "ran into the spikes");
assert!(hero.drain_events().any(|e| e == Event::Respawned));
assert!(hero.hitbox().x > start.x, "back at the checkpoint, not the start");
```

A thin trigger crossed at speed, at 15 frames per second: the last hitbox of each frame
never touches it, one of the steps' does.

```rust
use game_toolkit_platformer::prelude::*;

let mut floor = |r: Rect| r.bottom() > 100;
let trigger = Rect { x: 44, y: 0, w: 1, h: 100 };
let mut hero = Platformer::at(Spot::Feet(20, 100), Preset::Hybrid);
let dash = Intent { move_x: 1.0, dash_pressed: true, ..Intent::default() };
let (mut by_frame, mut by_step) = (false, false);
for k in 0..8 {
    let intent = if k == 0 { dash } else { Intent { move_x: 1.0, ..Intent::default() } };
    hero.advance(intent, &mut floor, 1.0 / 15.0);
    by_frame |= hero.hitbox().intersects(trigger);
    by_step |= hero.touched(trigger).is_some();
}
assert!(by_step, "caught in a step between frames");
assert!(!by_frame, "the frames alone miss it");
```
