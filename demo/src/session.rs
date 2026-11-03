//! [`Session`]: a game in progress: the world, the hero's animator, the clock, effects.
//!
//! Toolkit-free: [`Session::update`] takes frame time, an [`Intent`] and the [`Keys`] for
//! the clock and debug view. Each fixed step runs the simulation, the [`HeroDriver`] and
//! the [`Animator`] in that order, so the animation is as deterministic as the movement.

pub mod clock;
pub mod foliage;
pub mod fx;
pub mod wind;

use std::collections::VecDeque;

use crate::anim::{AnimEvent, Animator, Driver};
use crate::hero::rig::{Air, Rig};
use crate::hero::{HeroDriver, graph};
use crate::sim::{DT, Event, Intent, Level, Mode, Preset, Tuning, World, player};
pub use clock::Clock;
pub use foliage::Foliage;
pub use fx::{Fx, FxColor};
pub use wind::Wind;

/// Lines kept in the event log (the overlay's bottom-left corner).
const LOG: usize = 6;

/// How the game is drawn (F3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Skin {
    /// The art: the heroine of Cheesed in the moonlit scene.
    Mouse,
    /// Diagnosis: the hero as a block colored by kind of state, on a plain backdrop.
    Blocks,
}

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
    /// Switch between the art and the diagnostic look.
    pub skin: bool,
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
    /// The hero's simulated cape, tail, mane and headband ends.
    pub rig: Rig,
    /// How the game is drawn.
    pub skin: Skin,
    /// Moss and grass, swaying and brushed aside.
    pub foliage: Foliage,
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
        let foliage = Foliage::grow(&world.level);
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
            rig: Rig::default(),
            foliage,
            skin: Skin::Mouse,
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
                skin: self.skin,
                ..Self::with(level, tuning)
            };
        }
        if keys.skin {
            self.skin = match self.skin {
                Skin::Mouse => Skin::Blocks,
                Skin::Blocks => Skin::Mouse,
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
        let p = &self.world.player;
        let (x, y) = p.exact_pos();
        let sprite = self.animator.sprite();
        let wind = Wind::at(self.world.tick, y);
        let air = Air {
            breeze: wind.breeze,
            gust: wind.push,
        };
        self.rig.step(x, y, sprite, p.facing, (p.vx, p.vy), air, DT);
        let p = &self.world.player;
        self.foliage
            .step(self.world.tick, p.rect(), (p.vx, p.vy), DT);
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
            // What she does pushes the plants around her.
            let shove = match *e {
                Event::Landed { drop, .. } => Some((10.0, 1.0 + drop as f32 * 0.06)),
                Event::RollStarted => Some((8.0, 3.0)),
                Event::Dashed { .. } => Some((10.0, 4.0)),
                Event::DiveImpact { .. } => Some((20.0, 7.0)),
                Event::SuperDashStarted { .. } | Event::SuperDashEnded { bonk: true } => {
                    Some((14.0, 5.0))
                }
                _ => None,
            };
            if let Some((radius, strength)) = shove {
                self.foliage.shove(cx, feet - 1.0, radius, strength);
            }
            match *e {
                Event::Recharged { crystal: true } => {
                    self.fx.push(ring(cx, mid, 14.0, 0.3, FxColor::Recharge));
                    self.fx.push(Fx::Heart {
                        x: cx,
                        y: mid - 10.0,
                        t: 0.0,
                        dur: 0.8,
                    });
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
                Event::SuperCharged => self.fx.push(ring(cx, mid, 10.0, 0.25, FxColor::Impact)),
                Event::SuperDashStarted { .. } => {
                    self.fx.push(ring(cx, mid, 18.0, 0.3, FxColor::Impact));
                    self.fx.push(dust(cx, feet, 14.0));
                }
                Event::SuperDashEnded { bonk: true } => {
                    self.fx.push(ring(cx, mid, 12.0, 0.25, FxColor::Air));
                }
                _ => {}
            }
            self.log.push_back((tick, format!("{e:?}")));
        }
        let streak = match p.mode {
            Mode::Dash { .. } => 3,
            Mode::SuperDash { .. } => 2,
            _ => 0,
        };
        if streak > 0 && tick.is_multiple_of(streak) {
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
            match e {
                AnimEvent::Entered(s) => {
                    let name = &self.animator.graph().state(s).name;
                    self.log.push_back((tick, format!("-> {name}")));
                }
                // The animation controller's footstep markers rustle the grass underfoot.
                AnimEvent::Marker(m) if &*m == "step" => {
                    self.foliage.shove(cx, feet - 1.0, 4.0, 1.5);
                }
                _ => {}
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
