//! [`Session`]: a game in progress: the world, the hero's animator, the clock, effects.
//!
//! Toolkit-free: [`Session::update`] takes frame time, an [`Intent`] and the [`Keys`] for
//! the clock and debug view. Each fixed step runs the simulation, the [`HeroDriver`] and
//! the [`Animator`] in that order, so the animation is as deterministic as the movement.

pub mod clock;
pub mod fx;

use std::collections::VecDeque;

use crate::anim::{AnimEvent, Animator, Driver};
use crate::hero::{HeroDriver, graph};
use crate::sim::{DT, Event, Intent, Level, Mode, Preset, Tuning, World, player};
pub use clock::Clock;
pub use fx::{Fx, FxColor};

/// Lines kept in the event log (the overlay's bottom-left corner).
const LOG: usize = 6;

/// How much of the debug overlay to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overlay {
    /// The essentials in the corners: state, body, active parameters, a short log.
    Compact,
    /// Everything: history, every parameter, timers as bars, a longer log.
    Full,
    /// Nothing.
    Off,
}

impl Overlay {
    /// The next mode, wrapping.
    pub fn next(self) -> Self {
        match self {
            Overlay::Compact => Overlay::Full,
            Overlay::Full => Overlay::Off,
            Overlay::Off => Overlay::Compact,
        }
    }
}

/// Clock and view controls for one frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct Keys {
    /// Pause or go on.
    pub pause: bool,
    /// One step (pauses).
    pub step: bool,
    /// Slower.
    pub slower: bool,
    /// Faster.
    pub faster: bool,
    /// Normal speed.
    pub normal: bool,
    /// Next overlay mode (compact, full, off).
    pub overlay: bool,
    /// Back to the spawn.
    pub reset: bool,
    /// Switch to the next feel preset.
    pub preset: bool,
}

/// A game in progress.
pub struct Session {
    /// The simulation.
    pub world: World,
    /// The hero's animation.
    pub animator: Animator,
    /// Sim facts to animation parameters.
    driver: HeroDriver,
    /// Fixed steps, slow motion, pause.
    pub clock: Clock,
    /// Presses seen since the last step, so slow motion doesn't drop them.
    latch: Intent,
    /// The hero's exact position before the last step, for drawing between steps.
    pub prev: (f32, f32),
    /// Live effects.
    pub fx: Vec<Fx>,
    /// How much of the debug overlay to show.
    pub overlay: Overlay,
    /// Recent events, newest last, with the step they happened in.
    pub log: VecDeque<(u64, String)>,
    /// The intent of the last step, for the overlay.
    pub last_intent: Intent,
    /// The feel preset in use (when the tuning came from one).
    pub preset: Option<Preset>,
}

impl Session {
    /// A session on the demo level with default tuning.
    pub fn new() -> Self {
        let mut s = Self::with(Level::demo(), Preset::Hybrid.tuning());
        s.preset = Some(Preset::Hybrid);
        s
    }

    /// A session on `level` with `tuning`.
    pub fn with(level: Level, tuning: Tuning) -> Self {
        let world = World::new(level, tuning);
        let animator = Animator::new(graph());
        let driver = HeroDriver::new(&animator.params);
        Self {
            prev: world.player.exact_pos(),
            world,
            animator,
            driver,
            clock: Clock::default(),
            latch: Intent::default(),
            fx: Vec::new(),
            overlay: Overlay::Compact,
            log: VecDeque::new(),
            last_intent: Intent::default(),
            preset: None,
        }
    }

    /// Advance by `dt` seconds of frame time.
    pub fn update(&mut self, dt: f32, intent: Intent, keys: &Keys) {
        if keys.pause {
            self.clock.toggle_pause();
        }
        if keys.step {
            self.clock.step();
        }
        self.clock.change(keys.slower, keys.faster, keys.normal);
        if keys.overlay {
            self.overlay = self.overlay.next();
        }
        if keys.reset {
            let (level, tuning) = (self.world.level.clone(), self.world.tuning);
            *self = Self {
                overlay: self.overlay,
                clock: self.clock.clone(),
                preset: self.preset,
                ..Self::with(level, tuning)
            };
        }
        if keys.preset {
            // Swap the feel in place; resources refill on the next touch of the ground.
            let next = self.preset.map_or(Preset::Hybrid, Preset::next);
            self.preset = Some(next);
            self.world.tuning = next.tuning();
            self.log
                .push_back((self.world.tick, format!("preset {}", next.name())));
        }
        self.latch.jump_pressed |= intent.jump_pressed;
        self.latch.dash_pressed |= intent.dash_pressed;
        self.latch.dive_pressed |= intent.dive_pressed;
        for _ in 0..self.clock.advance(dt) {
            let i = Intent {
                jump_pressed: self.latch.jump_pressed,
                dash_pressed: self.latch.dash_pressed,
                dive_pressed: self.latch.dive_pressed,
                ..intent
            };
            self.latch = Intent::default();
            self.step(&i);
        }
    }

    /// One fixed step: simulation, driver, animator, effects.
    pub fn step(&mut self, i: &Intent) {
        self.prev = self.world.player.exact_pos();
        self.last_intent = *i;
        self.world.step(i);
        self.driver.drive(&self.world, &mut self.animator.params);
        self.animator.update(DT);
        self.fx.retain_mut(|f| f.tick(DT));
        self.effects();
    }

    /// Spawn effects for this step's events, and log them.
    fn effects(&mut self) {
        let p = &self.world.player;
        let (cx, feet) = ((p.x + player::W / 2) as f32, (p.y + player::H) as f32);
        let mid = feet - player::H as f32 / 2.0;
        let tick = self.world.tick;
        for e in &self.world.events {
            match *e {
                Event::Recharged { crystal: true } => {
                    self.fx.push(ring(cx, mid, 14.0, 0.3, FxColor::Recharge));
                }
                Event::Jumped {
                    kind: crate::sim::JumpKind::Double,
                    ..
                } => {
                    self.fx.push(ring(cx, feet, 8.0, 0.2, FxColor::Air));
                }
                Event::DiveImpact { .. } => {
                    self.fx.push(ring(cx, feet, 28.0, 0.35, FxColor::Impact));
                    self.fx.push(dust(cx, feet, 20.0));
                }
                Event::Landed { drop, .. } if drop >= 24 => {
                    self.fx.push(dust(cx, feet, 6.0 + drop as f32 / 8.0));
                }
                Event::RollStarted | Event::HardLanded => self.fx.push(dust(cx, feet, 12.0)),
                _ => {}
            }
            self.log.push_back((tick, format!("{e:?}")));
        }
        if matches!(p.mode, Mode::Dash { .. }) && tick.is_multiple_of(3) {
            let (x, y) = p.exact_pos();
            self.fx.push(Fx::Ghost {
                x,
                y,
                sprite: self.animator.sprite(),
                facing: p.facing,
                t: 0.0,
                dur: 0.25,
            });
        }
        for e in self.animator.drain_events() {
            if let AnimEvent::Entered(s) = e {
                let name = &self.animator.graph().state(s).name;
                self.log.push_back((tick, format!("-> {name}")));
            }
        }
        while self.log.len() > LOG {
            self.log.pop_front();
        }
    }

    /// Where to draw the hero: between the last two steps by the clock's progress.
    pub fn hero_pos(&self) -> (f32, f32) {
        let (x, y) = self.world.player.exact_pos();
        let a = self.clock.alpha();
        let (px, py) = self.prev;
        if (x - px).abs() > 16.0 || (y - py).abs() > 16.0 {
            return (x, y);
        }
        (px + (x - px) * a, py + (y - py) * a)
    }
}

#[cfg(test)]
mod tests;

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// A ring effect.
fn ring(x: f32, y: f32, r: f32, dur: f32, kind: FxColor) -> Fx {
    Fx::Ring {
        x,
        y,
        r,
        t: 0.0,
        dur,
        kind,
    }
}

/// A dust puff.
fn dust(x: f32, y: f32, spread: f32) -> Fx {
    Fx::Dust {
        x,
        y,
        t: 0.0,
        dur: 0.3,
        spread,
    }
}
