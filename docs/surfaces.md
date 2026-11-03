# Surfaces: ice, conveyors, walls you can't climb, bouncy pads

`Terrain::solid` says whether something is in the way; `Terrain::surface` says what kind
of ground or wall it is. The movement asks for the row under the feet while standing and
the column beside the body at a wall, and reads a `Surface`:

| Field | Default | What it does |
|---|---|---|
| `friction` | `1.0` | scales running's acceleration, braking and turning on the ground: below 1 for ice, above for sticky ground |
| `climbable` | `true` | whether the wall can be grabbed and climbed (wall slides and wall jumps still work) |
| `belt` | `0.0` | a conveyor: carries what stands on it sideways, units/s; a jump off keeps the speed |
| `bounce` | `0.0` | a landing faster than `Surface::BOUNCE_MIN` bounces back up at this share of its speed |

A terrain that says nothing is `Surface::DEFAULT` everywhere, and the `Tuning` alone sets
the feel. On a terrain of your own, write `surface` like `solid` (it forwards to the
`base` by default). On a `Level`, give your custom tiles solidity and a surface with
`Level::define`: `CustomTile::solid(surface)`, or `CustomTile::platform(surface)` for a
one-way platform of that surface (a bouncy or icy ledge you can jump up through).

## Example

Ice (`~`), a conveyor running right (`>`), a wall too smooth to climb (`|`) and a bouncy
pad (`o`) on a `Level`:

```rust
use game_toolkit_platformer::prelude::*;

const ICE: u8 = 0;
const BELT: u8 = 1;
const GLASS: u8 = 2;
const PAD: u8 = 3;

let rows = [
    "..............................|",
    "..............................|",
    "..............................|",
    "..............................|",
    "..............................|",
    "..............................|",
    ".P............................|",
    "~~~~~~~~~~>>>>>>>>>>oooo######|",
];
let mut level = Level::parse_with(&rows, |c| match c {
    '~' => Some(Tile::Custom(ICE)),
    '>' => Some(Tile::Custom(BELT)),
    '|' => Some(Tile::Custom(GLASS)),
    'o' => Some(Tile::Custom(PAD)),
    _ => None,
});
level.define(ICE, CustomTile::solid(Surface { friction: 0.1, ..Surface::DEFAULT }));
level.define(BELT, CustomTile::solid(Surface { belt: 40.0, ..Surface::DEFAULT }));
level.define(GLASS, CustomTile::solid(Surface { climbable: false, ..Surface::DEFAULT }));
level.define(PAD, CustomTile::solid(Surface { bounce: 0.9, ..Surface::DEFAULT }));

let frame = 1.0 / 60.0;
let idle = Intent::default();
let run = Intent { move_x: 1.0, ..idle };

// On ice, letting go of the stick doesn't stop the run at once.
let mut hero = Platformer::on(&level, Preset::Hybrid);
for _ in 0..40 {
    hero.advance(run, &mut level, frame);
}
for _ in 0..10 {
    hero.advance(idle, &mut level, frame);
}
assert!(hero.velocity().0 > 30.0, "still sliding: {:?}", hero.velocity());

// Standing still on the conveyor, it carries the character along.
hero.teleport(Spot::Feet(12 * 8, 8 * 8 - 8));
hero.launch(0.0, 0.0);
for _ in 0..30 {
    hero.advance(idle, &mut level, frame);
}
let x0 = hero.hitbox().x;
for _ in 0..60 {
    hero.advance(idle, &mut level, frame);
}
assert!(hero.hitbox().x - x0 >= 35, "carried {} units", hero.hitbox().x - x0);

// Dropped onto the pad, it bounces back up.
hero.teleport(Spot::Feet(21 * 8, 2 * 8));
let mut bounced = false;
for _ in 0..60 {
    hero.advance(idle, &mut level, frame);
    bounced |= hero.velocity().1 < -50.0;
}
assert!(bounced, "bounced off the pad");

// Against the glass wall, grabbing doesn't hold on.
hero.teleport(Spot::Feet(29 * 8, 7 * 8));
let grab = Intent { move_x: 1.0, grab: true, ..idle };
for _ in 0..30 {
    hero.advance(grab, &mut level, frame);
}
assert_ne!(hero.mode(), game_toolkit_platformer::sim::Mode::Climb);
```
