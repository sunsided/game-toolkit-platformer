//! A small game on the public API only: its own tilemap of 16-pixel tiles as a `Grid`
//! (a borrowing `GridTerrain`) at `scale(2)`, spikes, a spring, a checkpoint and coins
//! checked per step, ice and one-way platforms, and a lunge of its own with its own
//! animation, on a key that can be rebound. `cargo run --example tilemap`; X lunges, F5 moves it to V.

use game_toolkit::prelude::*;
use game_toolkit_platformer::anim::{
    Clip, Cond, ParamId, ParamKind, Params, Playback, StateSpec, TransitionSpec,
};
use game_toolkit_platformer::prelude::*;
use game_toolkit_platformer::sim::{Ability, Mode, Step, World};

/// Pixels per tile.
const T: i32 = 16;
/// The map: `#` ground, `~` ice, `=` one-way platform, `^` spikes, `S` spring,
/// `C` checkpoint, `o` coin, `P` start.
const MAP: &[&str] = &[
    "#..........................#",
    "#..........................#",
    "#.................====.....#",
    "#..........................#",
    "#..........................#",
    "#.P...C..o..S.........o..^^#",
    "#######~~~~~################",
];
/// The custom button bit of the lunge.
const LUNGE_BUTTON: u8 = 0;
/// The lunge's mode and event.
const LUNGE: CustomId = CustomId::new("tilemap::lunge");

/// The game's own map: rows of characters, nothing of the library's.
struct Map(Vec<Vec<u8>>);

impl Map {
    /// The tile at `tx, ty`; outside is air.
    fn at(&self, tx: i32, ty: i32) -> u8 {
        let row = self.0.get(ty as usize);
        row.and_then(|r| r.get(tx as usize))
            .copied()
            .unwrap_or(b'.')
    }

    /// The tiles `r` overlaps, with their place.
    fn under(&self, r: Rect) -> impl Iterator<Item = (i32, i32, u8)> + '_ {
        let (x0, x1) = (r.x.div_euclid(T), (r.right() - 1).div_euclid(T));
        let (y0, y1) = (r.y.div_euclid(T), (r.bottom() - 1).div_euclid(T));
        (y0..=y1).flat_map(move |y| (x0..=x1).map(move |x| (x, y, self.at(x, y))))
    }

    /// The first of `hitboxes` on a `c` tile, and that tile.
    fn touches(&self, hitboxes: &[Rect], c: u8) -> Option<(Rect, i32, i32)> {
        hitboxes.iter().find_map(|&r| {
            let (x, y, _) = self.under(r).find(|t| t.2 == c)?;
            Some((r, x, y))
        })
    }
}

/// The map as the movement sees it, one tile at a time; `GridTerrain(&map)` derives the
/// rest (one-way platforms from above only, surfaces under the feet).
impl Grid for Map {
    fn tile_size(&self) -> i32 {
        T
    }
    fn tile(&self, tx: i32, ty: i32) -> CustomTile {
        match self.at(tx, ty) {
            b'#' => CustomTile::solid(Surface::DEFAULT),
            b'~' => CustomTile::solid(Surface {
                friction: 0.1,
                ..Surface::DEFAULT
            }),
            b'=' => CustomTile::platform(Surface::DEFAULT),
            _ => CustomTile::default(),
        }
    }
}

/// The lunge: a short dash along the ground on its button, a mode of its own.
#[derive(Clone, Debug, Default)]
struct Lunge {
    /// Lunging now (for the animation).
    on: bool,
}

impl Ability for Lunge {
    fn before(&mut self, s: &mut Step<'_>) {
        let p = &mut *s.player;
        match p.mode {
            Mode::Normal if s.intent.pressed(LUNGE_BUTTON) && p.grounded() => {
                p.mode = Mode::Custom { id: LUNGE, t: 0.0 };
                (p.vx, p.vy) = (200.0 * p.facing as f32, 0.0);
                s.events.push(Event::custom(LUNGE));
            }
            Mode::Custom { id: LUNGE, t } if t >= 0.2 => p.mode = Mode::Normal,
            _ => {}
        }
        self.on = matches!(p.mode, Mode::Custom { id: LUNGE, .. });
    }
}

impl Animate for Lunge {
    /// Checked against the graph when the character is built.
    const PARAMS: &'static [(&'static str, ParamKind)] = &[("lunging", ParamKind::Bool)];

    fn drive(&self, _world: &World, p: &mut Params, ids: &[ParamId]) {
        p.set_bool(ids[0], self.on);
    }
}

/// Idle, run, air and lunge: a graph of its own declaring only what it reads.
fn graph() -> GraphSpec {
    let clip = |s: u16| Clip::uniform(Playback::Loop, &[s], 0.1);
    let to = |from: &[&'static str], to: &'static str, when| {
        TransitionSpec::new(from.iter().copied(), to, when)
    };
    let is = |p: &'static str| Cond::Is(p.into());
    // On the ground and not lunging, by speed.
    let ground = |fast: bool| {
        let speed = Cond::Above("speed".into(), 10.0);
        let speed = if fast { speed } else { Cond::not(speed) };
        is("grounded").and(Cond::not(is("lunging"))).and(speed)
    };
    GraphSpec::new()
        .bool("grounded")
        .float("speed")
        .bool("lunging")
        .state(StateSpec::new("idle", clip(0)))
        .state(StateSpec::new("run", clip(1)))
        .state(StateSpec::new("air", clip(2)))
        .state(StateSpec::new("lunge", clip(3)))
        .transition(TransitionSpec::any("lunge", is("lunging")).except(["lunge"]))
        .transition(TransitionSpec::any("air", Cond::not(is("grounded"))).except(["air"]))
        .transition(to(&["idle", "air", "lunge"], "run", ground(true)))
        .transition(to(&["run", "air", "lunge"], "idle", ground(false)))
}

/// A game in progress, without a window (the test plays it headless).
struct Tilemap {
    /// The map; coins become air when taken.
    map: Map,
    /// The character.
    hero: Platformer,
    /// Coins taken.
    coins: u32,
}

impl Tilemap {
    /// The map at its start, the character on `P`.
    fn new() -> Self {
        let map = Map(MAP.iter().map(|r| r.as_bytes().to_vec()).collect());
        let (sx, sy) = (0..MAP.len() as i32)
            .find_map(|y| Some((MAP[y as usize].find('P')? as i32, y)))
            .expect("a start");
        let hero = Platformer::builder()
            .preset(Preset::Snappy)
            .scale(2)
            .spawn(Spot::Feet(sx * T + T / 2, (sy + 1) * T))
            .partial_graph(graph())
            .animated(Lunge::default())
            .build()
            .expect("the graph fits");
        Self {
            map,
            hero,
            coins: 0,
        }
    }

    /// One frame: move, then everything the map does to the character, step by step.
    fn frame(&mut self, intent: Intent, dt: f32) {
        self.hero.advance(intent, &mut GridTerrain(&self.map), dt);
        if self.map.touches(self.hero.hitboxes(), b'^').is_some() {
            return self.hero.respawn();
        }
        if let Some((_, x, y)) = self.map.touches(self.hero.hitboxes(), b'C') {
            self.hero.set_spawn(Spot::Feet(x * T + T / 2, (y + 1) * T));
        }
        if self.map.touches(self.hero.hitboxes(), b'S').is_some() {
            self.hero.launch(0.0, -500.0); // pixels/s, like everything at the facade
        }
        while let Some((_, x, y)) = self.map.touches(self.hero.hitboxes(), b'o') {
            self.map.0[y as usize][x as usize] = b'.';
            self.coins += 1;
            self.hero.refill();
        }
    }
}

impl Game for Tilemap {
    fn init(_ctx: &mut Context) -> Result<Self> {
        let mut game = Tilemap::new();
        let lunge = Custom::new(LUNGE_BUTTON, &[Key::KeyX], &[Button::West]);
        game.hero.controls.scheme.custom = vec![lunge].into();
        Ok(game)
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) {
        if ctx.input.key_pressed(Key::Escape) {
            ctx.quit();
        }
        if ctx.input.key_pressed(Key::F5) {
            // A rebinding menu's answer: the lunge on V from now on.
            let scheme = &mut self.hero.controls.scheme;
            scheme.bind(LUNGE_BUTTON, vec![Key::KeyV], vec![Button::West]);
        }
        let intent = self.hero.controls.read(&ctx.input);
        self.frame(intent, dt);
        self.hero.drain_events().for_each(drop);
    }

    fn render(&mut self, ctx: &mut Context, frame: &mut Frame) {
        let mut p = frame.painter(&mut ctx.gfx);
        p.clear([0.05, 0.04, 0.12, 1.0]);
        let s = 1.5; // window pixels per map pixel
        for (y, row) in self.map.0.iter().enumerate() {
            for (x, &c) in row.iter().enumerate() {
                let color = match c {
                    b'#' => Color::rgb8(74, 64, 100),
                    b'~' => Color::rgb8(150, 200, 230),
                    b'=' => Color::rgb8(140, 120, 90),
                    b'^' => Color::rgb8(220, 60, 60),
                    b'S' => Color::rgb8(90, 200, 90),
                    b'C' => Color::rgb8(240, 220, 90),
                    b'o' => Color::rgb8(250, 190, 40),
                    _ => continue,
                };
                let at = [(x as i32 * T) as f32 * s, (y as i32 * T) as f32 * s];
                p.rect(at, [T as f32 * s, T as f32 * s], color);
            }
        }
        let (x, y) = self.hero.position();
        let r = self.hero.hitbox();
        p.rect(
            [x * s, y * s],
            [r.w as f32 * s, r.h as f32 * s],
            Color::rgb8(240, 240, 255),
        );
        let text = format!("{}  coins {}", self.hero.state(), self.coins);
        p.text([12.0, 8.0], &text, 18.0, [0.8, 0.8, 0.9, 1.0]);
    }
}

/// Opens the window.
fn main() -> Result<()> {
    let (w, h) = (MAP[0].len() as u32 * 24, MAP.len() as u32 * 24 + 40);
    run::<Tilemap>(AppConfig {
        title: "tilemap".into(),
        width: w,
        height: h,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scripted_run_through_the_map() {
        let mut game = Tilemap::new();
        let right = Intent {
            move_x: 1.0,
            ..Intent::default()
        };
        let (start, mut launched, mut respawned) = (game.hero.spawn(), false, false);
        for _ in 0..600 {
            game.frame(right, 1.0 / 30.0);
            launched |= game.hero.velocity().1 < -300.0;
            if game.hero.drain_events().any(|e| e == Event::Respawned) {
                respawned = true;
                break;
            }
        }
        assert!(launched, "the spring threw it up");
        assert_eq!(game.coins, 2, "both coins");
        assert!(respawned, "ran into the spikes");
        assert_ne!(game.hero.spawn(), start, "the checkpoint moved the spawn");
        assert_eq!(
            game.hero.hitbox().bottom(),
            6 * T,
            "back on the checkpoint's floor"
        );

        // The one-way platform holds from above only; the ice is slippery ground.
        let view = GridTerrain(&game.map);
        let on = |y| Rect {
            x: 18 * T,
            y,
            w: 12,
            h: 24,
        };
        assert!(view.platform(on(2 * T - 24)) && !view.platform(on(2 * T - 20)));
        assert!(
            !view.solid(on(2 * T - 10)),
            "one-way: jumped through from below"
        );
        assert_eq!(
            view.surface(Rect {
                x: 8 * T,
                y: 5 * T,
                w: 12,
                h: 17
            })
            .friction,
            0.1
        );

        // The lunge, on its button, with its own animation state.
        for _ in 0..30 {
            game.frame(Intent::default(), 1.0 / 60.0);
        }
        let lunge = Intent {
            custom: 1,
            custom_pressed: 1,
            ..Intent::default()
        };
        game.frame(lunge, 1.0 / 60.0);
        assert!(game.hero.drain_events().any(|e| e == Event::custom(LUNGE)));
        assert_eq!(game.hero.state(), "lunge");
        assert!(game.hero.velocity().0 > 300.0, "{:?}", game.hero.velocity());
    }
}
