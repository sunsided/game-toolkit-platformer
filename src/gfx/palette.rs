//! Colors: the [`Rgb`] type and the palette.
//!
//! The images are true color, so blends produce in-between colors; the palette is a style
//! constraint for drawn art (pick from here, don't invent), not a hardware limit.

/// An 8-bit-per-channel sRGB color: `Rgb(red, green, blue)`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// Build a color from a `0xRRGGBB` literal.
    pub const fn hex(v: u32) -> Self {
        Self((v >> 16) as u8, (v >> 8) as u8, v as u8)
    }

    /// Linear mix from `self` (at `t = 0`) to `o` (at `t = 1`), per channel in sRGB.
    /// `t` is clamped to 0..=1.
    pub fn lerp(self, o: Rgb, t: f32) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Rgb(m(self.0, o.0), m(self.1, o.1), m(self.2, o.2))
    }

    /// Multiply every channel by `f`, clamped to 0..=255.
    pub fn scale(self, f: f32) -> Rgb {
        let m = |a: u8| (a as f32 * f).clamp(0.0, 255.0) as u8;
        Rgb(m(self.0), m(self.1), m(self.2))
    }
}

/// Pure black: outlines, text shadows.
pub const BLACK: Rgb = Rgb::hex(0x000000);
/// The top of the sky.
pub const SKY_TOP: Rgb = Rgb::hex(0x10142e);
/// The bottom of the sky.
pub const SKY_BOTTOM: Rgb = Rgb::hex(0x2c3463);
/// Dark stone: the body of solid tiles.
pub const STONE_DK: Rgb = Rgb::hex(0x2a2a3a);
/// Mid stone: tile faces.
pub const STONE: Rgb = Rgb::hex(0x4a4a64);
/// Light stone: the lit top edge of tiles.
pub const STONE_LT: Rgb = Rgb::hex(0x8a8aa8);
/// Night sky, the top.
pub const SKY_0: Rgb = Rgb::hex(0x0d0820);
/// Night sky, upper middle.
pub const SKY_1: Rgb = Rgb::hex(0x22134a);
/// Night sky, lower middle.
pub const SKY_2: Rgb = Rgb::hex(0x43236a);
/// Night sky at the horizon, warmed by the torches below.
pub const SKY_3: Rgb = Rgb::hex(0x5e3478);
/// Starlight.
pub const STAR: Rgb = Rgb::hex(0xe6dcff);
/// The moon.
pub const MOON: Rgb = Rgb::hex(0xeee8fa);
/// The moon's shaded side.
pub const MOON_DK: Rgb = Rgb::hex(0xbfb1dc);
/// The moon's craters (cheese holes, in a way).
pub const MOON_CRATER: Rgb = Rgb::hex(0xc9bde3);
/// The moon's halo.
pub const MOON_GLOW: Rgb = Rgb::hex(0x9a7ccc);
/// Cloud banks.
pub const CLOUD: Rgb = Rgb::hex(0x3a2466);
/// Cloud banks, the moonlit side.
pub const CLOUD_LT: Rgb = Rgb::hex(0x6a4c98);
/// The far mountains.
pub const NIGHT_FAR: Rgb = Rgb::hex(0x241848);
/// Snow on the far mountains.
pub const NIGHT_SNOW: Rgb = Rgb::hex(0x8a80b8);
/// The near hills.
pub const NIGHT_NEAR: Rgb = Rgb::hex(0x180f36);
/// The castle's silhouette.
pub const CASTLE: Rgb = Rgb::hex(0x140c2c);
/// Lit windows and torches.
pub const WINDOW: Rgb = Rgb::hex(0xffbf4f);
/// Mist in the valley.
pub const MIST: Rgb = Rgb::hex(0x5a4486);
/// Fir silhouettes.
pub const FIR: Rgb = Rgb::hex(0x0e0820);
/// Rock tiles.
pub const ROCK: Rgb = Rgb::hex(0x4a4064);
/// Rock in shadow.
pub const ROCK_DK: Rgb = Rgb::hex(0x2a2240);
/// The lit rim of exposed rock.
pub const ROCK_LT: Rgb = Rgb::hex(0x9c8cbc);
/// Warm flecks on lit rock, as in the paintings' torchlight.
pub const GOLD_MOSS: Rgb = Rgb::hex(0xd8a84a);
/// Wooden planks.
pub const PLANK: Rgb = Rgb::hex(0x7a4a28);
/// The planks' lit top edge.
pub const PLANK_LT: Rgb = Rgb::hex(0xb47c42);
/// The planks' underside and grain.
pub const PLANK_DK: Rgb = Rgb::hex(0x46281a);
/// Nail heads.
pub const NAIL: Rgb = Rgb::hex(0xd8b878);
/// The mouse's fur in the light: cheek, muzzle.
pub const MOUSE_FUR_LT: Rgb = Rgb::hex(0xd6c8de);
/// The mouse's iris (yellow, as on the sheet).
pub const MOUSE_IRIS: Rgb = Rgb::hex(0xf2c94c);
/// The mouse's fur, from the Cheesed character sheet.
pub const MOUSE_FUR: Rgb = Rgb::hex(0xb9a6c6);
/// The mouse's fur in shadow: the far ear, legs.
pub const MOUSE_FUR_DK: Rgb = Rgb::hex(0x8d7a9e);
/// Inside the ears, the tail (the sheet's "fur (light)").
pub const MOUSE_PINK: Rgb = Rgb::hex(0xfab9c9);
/// The far ear's inside, the nose.
pub const MOUSE_PINK_DK: Rgb = Rgb::hex(0xd88aa4);
/// The blonde mane.
pub const MOUSE_HAIR: Rgb = Rgb::hex(0xfdab13);
/// The mane in shadow.
pub const MOUSE_HAIR_DK: Rgb = Rgb::hex(0xd0850a);
/// The mane with the dash spent (as Madeline's hair turns in Celeste).
pub const MOUSE_HAIR_SPENT: Rgb = Rgb::hex(0x9fb6e0);
/// The mane in shadow with the dash spent.
pub const MOUSE_HAIR_SPENT_DK: Rgb = Rgb::hex(0x6f86b8);
/// Headband and cape.
pub const MOUSE_RED: Rgb = Rgb::hex(0xbc1327);
/// The cape's torn hem.
pub const MOUSE_RED_DK: Rgb = Rgb::hex(0x7e0c1c);
/// The tunic.
pub const MOUSE_TUNIC: Rgb = Rgb::hex(0x9c6c89);
/// Boots, belt, gloves.
pub const MOUSE_LEATHER: Rgb = Rgb::hex(0x934729);
/// Leather in shadow: the far boot, straps, the sheath.
pub const MOUSE_LEATHER_DK: Rgb = Rgb::hex(0x5e2c1a);
/// Leather in the light: boot cuffs.
pub const MOUSE_LEATHER_LT: Rgb = Rgb::hex(0xb8693e);
/// The tunic's shaded side.
pub const MOUSE_TUNIC_DK: Rgb = Rgb::hex(0x74506a);
/// Crystal light (the super dash's charge).
pub const CRYSTAL: Rgb = Rgb::hex(0xff7ac8);
/// Crystal light, bright.
pub const CRYSTAL_LT: Rgb = Rgb::hex(0xffd6f0);
/// The sword's blade.
pub const BLADE: Rgb = Rgb::hex(0xe2e8f6);
/// The blade's fuller, its shaded line.
pub const BLADE_DK: Rgb = Rgb::hex(0x8c96b8);
/// The belt buckle.
pub const MOUSE_METAL: Rgb = Rgb::hex(0xee9210);
/// The eye, a single dark pixel at this size.
pub const MOUSE_EYE: Rgb = Rgb::hex(0x2a1626);
/// Cheese: the wedge.
pub const CHEESE: Rgb = Rgb::hex(0xfdc443);
/// Cheese: the rind and the holes.
pub const CHEESE_DK: Rgb = Rgb::hex(0xd99a1a);
/// Wood: blocks.
pub const WOOD: Rgb = Rgb::hex(0x9a6a3a);
/// Dark wood: block outlines and braces.
pub const WOOD_DK: Rgb = Rgb::hex(0x5a3a1e);
/// Light wood: the lit top edge of blocks.
pub const WOOD_LT: Rgb = Rgb::hex(0xc8935a);
/// Mid blue-grey: panel borders, debug trail.
pub const STEEL: Rgb = Rgb::hex(0x4a5591);
/// Light blue-grey: secondary text.
pub const STEEL_LT: Rgb = Rgb::hex(0x7d8cc4);
/// Pale blue-white: primary text.
pub const PALE: Rgb = Rgb::hex(0xc2cbe8);
/// Pure white: the hero's eye, highlights.
pub const WHITE: Rgb = Rgb::hex(0xffffff);
/// Bright yellow: active values in the debug overlay, triggers.
pub const YELLOW: Rgb = Rgb::hex(0xffe14a);
/// Orange: the hero's cloak.
pub const ORANGE: Rgb = Rgb::hex(0xff8a1f);
/// Red: locks and hard landings.
pub const RED: Rgb = Rgb::hex(0xe8302a);
/// Green: grounded, healthy values.
pub const GREEN: Rgb = Rgb::hex(0x5ad65a);
/// Cyan: air states, the double jump ring.
pub const CYAN: Rgb = Rgb::hex(0x5ae8ff);
/// Magenta: rolls.
pub const MAGENTA: Rgb = Rgb::hex(0xd65ae8);
