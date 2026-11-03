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
