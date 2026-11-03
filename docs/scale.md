# Pixels and units

The movement works in units, made for 8-unit tiles and a 6x12 body. If your terrain
speaks pixels (16-pixel tiles, say), give the builder a scale: `Builder::scale(2)`. The
`Platformer` then converts at its edges, and the rule is simple: what the facade hands
in or out is in your pixels, what runs inside the movement is in units.

| API | Space |
|---|---|
| Your `Terrain` (every method, `tick`'s bodies, `carry`'s answer, `Surface::belt`) | pixels |
| `position`, `hitbox`, `hitboxes`, `touched`, `velocity` | pixels |
| `Spot`s: `Builder::spawn`, `teleport`, `set_spawn`, `spawn`, `respawn_at` | pixels |
| `launch(vx, vy)` | pixels/s |
| `drain_events`: `Landed { drop, impact }`, `DiveImpact { drop }`, `CornerCorrected { dx }` | pixels, pixels/s |
| `Event::Custom` data | yours (units unless you say otherwise) |
| `Tuning` (speeds, heights, `size`), `tuning()`, `tune` | units |
| `world()`, `world_mut()`: `Player` fields, `World::spawn`, `World::events` | units |
| Abilities (`Step`: `player`, `terrain`, `events`), `Player::contacts` | units |
| A terrain of yours reached through `find_mut` / `find` (in an ability: `s.terrain.find_mut::<MyMap>()`) | pixels: ask it with `rect.scaled(s.scale)` (`Step::scale`, `World::scale()`) |
| Drivers and `Animate::drive` (they read the `World`) | units |
| `Level` | units (keep the scale at 1 and draw it scaled) |

Directions (`facing`, `on_wall`, `Contacts::wall`, `Dashed { dx, dy }`) and counts
(`dashes`, `air_jumps`, `stamina`) have no space.

So `Event::Landed { drop, .. } if drop > 48` means 48 of your pixels at any scale, and an
`Ability` that wants a 24-pixel hop at `scale(2)` asks for 12 units. `Event::in_pixels(k)`
converts an event from the world (`world().events`) the way the facade does.
