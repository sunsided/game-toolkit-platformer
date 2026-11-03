# Hazards, triggers and checkpoints

The controller moves the character; what hurts it, what it picks up and where it comes
back is your game's. The pattern: after each `update` (or `advance`), test the
character's `hitbox()` against your things, then act through the facade.

| You want | Test | Then |
|---|---|---|
| Spikes, lava, a kill plane | the hitbox overlaps it | `respawn()` (a fresh body at the spawn point, an `Event::Respawned`) |
| Knockback | the hitbox overlaps an enemy | `launch(vx, vy)`: away from it and up; it isn't cut by releasing jump |
| A spring, a bounce pad | the feet are on it (`hitbox().bottom() == pad.top`) | `launch(0.0, -speed)` |
| A checkpoint | the hitbox overlaps it | `set_spawn(x, y)` (hitbox top-left; `standing_at(x, floor_y)` works it out from the feet) |
| A pickup | the hitbox overlaps it | remove it; `refill()` for dashes, air jumps and stamina |
| A door, a portal | the hitbox overlaps it and a custom button was pressed | `teleport(x, y)` |
| A pit | `Terrain::fell_out` says so | nothing: the character respawns by itself, or with `respawn(false)` on the builder you get `Event::FellOut` and decide |

On a `Level`, mark these with tiles of your own: `Level::parse_with` reads
`Tile::Custom(n)` from characters you choose, the movement treats them as air, and
`Level::tiles_touching(hitbox)` lists what the character overlaps. On a terrain of your
own, keep them however you like and test rects.

Test after the update, not before: `hitbox()` is where the last fixed step left the
character, and the events of that step (`drain_events()`) are still waiting, so a landing
on a spring and its `Event::Landed` arrive together.

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
    let touching: Vec<Tile> = level.tiles_touching(hero.hitbox()).map(|(_, _, t)| t).collect();
    if touching.contains(&Tile::Custom(SPIKES)) {
        hero.respawn();
        respawned = true;
        break;
    }
    if touching.contains(&Tile::Custom(CHECKPOINT)) {
        let r = hero.hitbox();
        let (x, y) = hero.standing_at(r.x + r.w / 2, r.bottom());
        hero.set_spawn(x, y);
    }
    if touching.contains(&Tile::Custom(SPRING)) && hero.world.player.grounded {
        hero.launch(0.0, -320.0);
    }
}
assert!(respawned, "ran into the spikes");
assert!(hero.drain_events().contains(&Event::Respawned));
assert!(hero.hitbox().x > start.x, "back at the checkpoint, not the start");
```
