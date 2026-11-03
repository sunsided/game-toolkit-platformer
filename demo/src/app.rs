//! The game-toolkit glue: window, input, presenting. The only module that touches the
//! toolkit.
//!
//! [`Game::update`] reads the keyboard and pad into an [`Intent`](crate::sim::Intent) and
//! advances the [`Session`]. [`Game::render`] draws the cached backdrop as one
//! integer-scaled sprite and the session's moving parts, rasterized into a [`DrawList`],
//! as one rect per span on the layer above.

pub mod controls;
pub mod screenshots;
pub mod viewport;

use game_toolkit::prelude::*;

use crate::gfx::{DrawList, H, W};
use crate::render;
use crate::session::{Session, Skin};
use controls::read_keys;
use game_toolkit_platformer::controls::{Controls, Scheme};
use screenshots::Screenshots;
use viewport::Viewport;

/// The screenshot asked for on the command line: path and seconds after start.
static SCREENSHOT: std::sync::OnceLock<(std::path::PathBuf, f32)> = std::sync::OnceLock::new();

/// Save a screenshot of the window to `path` once the app has run `secs` seconds, then
/// quit (call before `run`).
pub fn screenshot_after(path: impl Into<std::path::PathBuf>, secs: f32) {
    let _ = SCREENSHOT.set((path.into(), secs));
}

/// The `id` of the HTML canvas on the web.
pub const CANVAS_ID: &str = "animation";
/// The layer the moving parts draw on, above the backdrop sprite on layer 0.
const DYNAMIC: Layer = Layer(1);

/// The game.
pub struct App {
    /// The game in progress.
    session: Session,
    /// The backdrop texture.
    tex: TextureId,
    /// The moving parts of the current frame.
    draw: DrawList,
    /// Seconds since start.
    clock: f32,
    /// F12 and command-line screenshots.
    shots: Screenshots,
    /// Movement input, through the current preset's control scheme.
    controls: Controls,
    /// The look the backdrop texture was drawn for; F3 switches it.
    drawn_for: Skin,
    /// The backdrop's visible stars, which twinkle.
    stars: Vec<render::Twinkle>,
}

impl App {
    /// F12 saves the next frame to `screenshots/`; a command-line screenshot is taken when
    /// due, and the app quits once it is saved.
    fn screenshot(&mut self, ctx: &mut Context) {
        if self.shots.quit && self.shots.auto.is_none() {
            ctx.quit();
            return;
        }
        let path = if let Some(p) = self.shots.due(self.clock) {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            Some(p)
        } else if ctx.input.key_pressed(Key::F12) {
            Some(Screenshots::next_path("animation"))
        } else {
            None
        };
        if let Some(p) = path {
            log::info!("screenshot {}", p.display());
            ctx.gfx.request_screenshot(p);
        }
    }
}

impl Game for App {
    /// Start on the demo level and upload its backdrop.
    fn init(ctx: &mut Context) -> Result<Self> {
        let session = Session::new();
        let backdrop = render::backdrop(&session.world.level);
        let stars = render::twinkles(&backdrop);
        let mut rgba = vec![0; (W * H * 4) as usize];
        backdrop.to_rgba(&mut rgba);
        let tex = ctx
            .gfx
            .create_texture_rgba(W as u32, H as u32, &rgba, Some("backdrop"));
        Ok(Self {
            session,
            tex,
            draw: DrawList::default(),
            clock: 0.0,
            shots: Screenshots {
                auto: SCREENSHOT.get().cloned(),
                ..Screenshots::default()
            },
            controls: Controls::default(),
            drawn_for: Skin::Mouse,
            stars,
        })
    }

    /// Read input, advance the session.
    fn update(&mut self, ctx: &mut Context, dt: f32) {
        self.clock += dt;
        self.screenshot(ctx);
        if ctx.input.key_pressed(Key::Escape) {
            ctx.quit();
        }
        // F2 switches the preset, and with it the controls.
        self.controls.scheme = Scheme::of(self.session.preset);
        let intent = self.controls.read(&ctx.input);
        self.session.update(dt, intent, &read_keys(&ctx.input));
    }

    /// Draw the backdrop and the moving parts, both scaled by the same whole number.
    fn render(&mut self, ctx: &mut Context, frame: &mut Frame) {
        if self.session.skin != self.drawn_for {
            // F3: the art's night scene, or the plain diagnostic backdrop.
            let level = &self.session.world.level;
            let canvas = match self.session.skin {
                Skin::Mouse => render::backdrop(level),
                Skin::Blocks => render::plain(level),
            };
            let mut rgba = vec![0; (W * H * 4) as usize];
            canvas.to_rgba(&mut rgba);
            ctx.gfx.update_texture_rgba(self.tex, &rgba);
            self.stars = render::twinkles(&canvas);
            self.drawn_for = self.session.skin;
        }
        self.draw.clear();
        render::scene(&mut self.draw, &self.session, &self.stars, self.clock);
        let (ww, wh) = ctx.gfx.size();
        let vp = Viewport::fit(ww, wh);
        let mut p = frame.painter(&mut ctx.gfx);
        p.clear([0.0, 0.0, 0.0, 1.0]);
        p.sprite(
            self.tex,
            [vp.x, vp.y],
            [W as f32 * vp.scale, H as f32 * vp.scale],
        );
        for s in &self.draw.spans {
            let c = Color::rgb8(s.c.0, s.c.1, s.c.2).with_alpha(s.a);
            p.rect_ex(
                [vp.x + s.x0 as f32 * vp.scale, vp.y + s.y as f32 * vp.scale],
                [(s.x1 - s.x0) as f32 * vp.scale, vp.scale],
                c,
                DYNAMIC,
                BlendMode::Alpha,
            );
        }
    }
}

/// The window and runtime configuration.
pub fn config() -> AppConfig {
    AppConfig {
        title: "animation".into(),
        width: (W * 4) as u32,
        height: (H * 4) as u32,
        // Everything is drawn in code: no asset files.
        asset_root: if cfg!(target_arch = "wasm32") {
            asset_root!()
        } else {
            ".".into()
        },
        canvas_id: Some(CANVAS_ID.into()),
        web_preload: Some(Vec::new()),
        ..Default::default()
    }
}
