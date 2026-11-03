//! [`Fx`]: short-lived visual effects spawned from simulation events. Drawing only.

/// One effect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fx {
    /// An expanding ring: recharges, dive impacts, double jumps.
    Ring {
        /// Center x, grid pixels.
        x: f32,
        /// Center y.
        y: f32,
        /// Final radius.
        r: f32,
        /// Seconds alive.
        t: f32,
        /// Lifetime.
        dur: f32,
        /// Which palette color (see `render`).
        kind: FxColor,
    },
    /// An afterimage of the hero, fading: dashes.
    Ghost {
        /// Hitbox left.
        x: f32,
        /// Hitbox top.
        y: f32,
        /// Sprite shown.
        sprite: u16,
        /// Facing.
        facing: i32,
        /// Seconds alive.
        t: f32,
        /// Lifetime.
        dur: f32,
    },
    /// A little heart floating up: cheese! (a dash refill from a wedge).
    Heart {
        /// Center x, grid pixels.
        x: f32,
        /// Start y.
        y: f32,
        /// Seconds alive.
        t: f32,
        /// Lifetime.
        dur: f32,
    },
    /// Dust kicked up sideways from the feet: landings, rolls, skids.
    Dust {
        /// Feet center x.
        x: f32,
        /// Feet y.
        y: f32,
        /// Seconds alive.
        t: f32,
        /// Lifetime.
        dur: f32,
        /// How wide the puff gets.
        spread: f32,
    },
}

/// Effect colors, mapped to the palette by the renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FxColor {
    /// Dash recharge.
    Recharge,
    /// Air jump.
    Air,
    /// Dive impact.
    Impact,
}

impl Fx {
    /// Age by `dt`; false once it is gone.
    pub fn tick(&mut self, dt: f32) -> bool {
        let (t, dur) = match self {
            Fx::Ring { t, dur, .. }
            | Fx::Ghost { t, dur, .. }
            | Fx::Dust { t, dur, .. }
            | Fx::Heart { t, dur, .. } => (t, *dur),
        };
        *t += dt;
        *t < dur
    }

    /// How far through its life, 0..1.
    pub fn progress(&self) -> f32 {
        match *self {
            Fx::Ring { t, dur, .. }
            | Fx::Ghost { t, dur, .. }
            | Fx::Dust { t, dur, .. }
            | Fx::Heart { t, dur, .. } => (t / dur).clamp(0.0, 1.0),
        }
    }
}
