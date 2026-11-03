# Moving platforms

Moving ground is your terrain's, not the controller's: `Level` has none. A terrain with a
platform that moves answers three methods together, and the world does the rest (riding,
the lift boost on jumping off, `Event::Squished`):

| Method | Answers |
|---|---|
| `solid(r)` | the platform's box counts, where it is now |
| `update(dt, bodies)` | move it, once per step, after the characters moved; don't move into a body from below or the side (it doesn't push characters), but do move into one standing on top, which gets carried |
| `carry(r)` | how far it moved in that update, when `r` overlapped it before: `r` is the row under the feet, or the column beside a hand on its side |

Build it on a `Level` (or any terrain) by naming that as the `base`: platforms, refills
and falling out stay the level's, and the update has to call the base's too.

The demo's lift (`demo/src/stage/lift.rs`) does this with two ends and a pause at each.

## Example

A platform going back and forth over a pit:

```rust
use game_toolkit_platformer::prelude::*;

/// A level with a platform sliding between `left` and `right` (its x, units).
struct Ferry {
    level: Level,
    plat: Rect,
    before: Rect,
    left: i32,
    right: i32,
    dir: i32,
}

impl Terrain for Ferry {
    fn solid(&self, r: Rect) -> bool {
        self.level.solid(r) || r.intersects(self.plat)
    }

    fn update(&mut self, dt: f32, bodies: &[Rect]) {
        self.level.update(dt, bodies);
        self.before = self.plat;
        if self.plat.x <= self.left || self.plat.x >= self.right {
            self.dir = if self.plat.x <= self.left { 1 } else { -1 };
        }
        // One unit per step (120 units/s); stop short of anyone in the way.
        let next = self.plat.shifted(self.dir, 0);
        let riding = |b: &Rect| b.bottom() == self.plat.y;
        if !bodies.iter().any(|b| next.intersects(*b) && !riding(b)) {
            self.plat = next;
        }
    }

    fn carry(&self, r: Rect) -> (f32, f32) {
        if r.intersects(self.before) {
            ((self.plat.x - self.before.x) as f32, (self.plat.y - self.before.y) as f32)
        } else {
            (0.0, 0.0)
        }
    }

    fn base(&self) -> Option<&dyn Terrain> {
        Some(&self.level)
    }

    fn base_mut(&mut self) -> Option<&mut dyn Terrain> {
        Some(&mut self.level)
    }
}

let level = Level::parse(&[
    "..................",
    "..................",
    "..................",
    ".P................",
    "###............###",
]);
let mut ferry = Ferry {
    plat: Rect { x: 24, y: 32, w: 24, h: 4 },
    before: Rect { x: 24, y: 32, w: 24, h: 4 },
    left: 24,
    right: 96,
    dir: 1,
    level,
};
// Standing on the platform as it sets off.
let mut hero = Platformer::standing(36, 32, Preset::Hybrid);
for _ in 0..30 {
    hero.advance(Intent::default(), &mut ferry, 1.0 / 60.0);
}
assert!(hero.grounded());
assert_eq!(hero.hitbox().bottom(), ferry.plat.y, "riding it");
assert!(hero.hitbox().x > 40, "carried across the pit");
```
