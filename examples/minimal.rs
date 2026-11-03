//! The smallest platformer: the demo level, a character with the precise preset's feel and
//! controls, and the debug look. `cargo run --example minimal`; F2 cycles the presets.

use game_toolkit::prelude::*;
use game_toolkit_platformer::prelude::*;

/// The game: a level and one character.
struct Minimal {
    /// The level: any `Terrain` would do.
    level: Level,
    /// The character.
    hero: Platformer,
}

impl Game for Minimal {
    fn init(_ctx: &mut Context) -> Result<Self> {
        let level = Level::demo();
        let hero = Platformer::on(&level, Preset::Precise);
        Ok(Self { level, hero })
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) {
        if ctx.input.key_pressed(Key::Escape) {
            ctx.quit();
        }
        if ctx.input.key_pressed(Key::F2) {
            let next = self.hero.preset().unwrap_or(Preset::Hybrid).next();
            self.hero.set_preset(next);
        }
        self.hero.update(&ctx.input, &mut self.level, dt);
        for event in self.hero.drain_events() {
            if let Event::Landed { drop, .. } = event
                && drop > 48
            {
                // A place for a thud and some dust.
                eprintln!("a big landing: {drop} units");
            }
        }
    }

    fn render(&mut self, ctx: &mut Context, frame: &mut Frame) {
        let mut p = frame.painter(&mut ctx.gfx);
        p.clear([0.05, 0.04, 0.12, 1.0]);
        self.level.draw_debug(&mut p, [0.0, 0.0], 3.0);
        self.hero.draw_debug(&mut p, [0.0, 0.0], 3.0);
        let preset = self.hero.preset().map_or("custom", Preset::name);
        p.text(
            [12.0, 8.0],
            &format!("{preset} (F2)"),
            18.0,
            [0.8, 0.8, 0.9, 1.0],
        );
    }
}

/// Opens the window.
fn main() -> Result<()> {
    run::<Minimal>(AppConfig {
        title: "minimal platformer".into(),
        width: 384 * 3,
        height: 216 * 3,
        ..Default::default()
    })
}
