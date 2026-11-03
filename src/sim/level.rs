//! [`Level`]: a ready-made [`Terrain`]: a grid of tiles (solid, one-way platforms), dash
//! refills and a spawn point. Parse it from ASCII rows, build it from your own tilemap
//! with [`Level::from_fn`], or implement [`Terrain`] for your own world instead.
//!
//! Tiles of your own ([`Tile::Custom`]: spikes, water, checkpoints) are air to the
//! movement unless [`Level::define`] makes them solid ground or one-way platforms of some
//! [`Surface`] (ice, a conveyor, a wall that can't be climbed, a bouncy ledge); [`Level::parse_with`] reads them from
//! characters of your choosing and [`Level::tiles_touching`] finds the ones under the
//! character.
//!
//! Tiles are [`TILE`] units unless [`Level::with_tile_size`] says otherwise; the tuning is
//! made for 8-unit tiles, so bigger tiles make jumps relatively lower. For more (doors, moving
//! platforms, boxes: state of your own beside the level), wrap it in your own terrain with
//! the level as its [`Terrain::base`], and write only what changes (see
//! [`super::terrain`]).

use std::collections::BTreeMap;

use super::grid::LevelGrid;
use super::{Crystal, CustomTile, Grid, Hold, Rect, Spot, Surface, Terrain};

/// The default tile size in units (see [`Level::tile_size`]).
pub const TILE: i32 = 8;

/// Seconds before a used dash refill comes back, unless the level says otherwise.
const REFILL_BACK: f32 = 2.5;

/// The demo level, one string per tile row: `#` solid, `=` one-way platform (stand on it,
/// jump up through it, down + jump drops through), `*` a dash crystal, `.` empty, `P` the
/// spawn (the hero stands on the tile below it). 48x27 tiles, exactly one screen.
///
/// Laid out around the debug overlay's corners, the play happens in the middle:
///
/// - left: stairs of 3-tile (24 px) steps, each within a plain jump, up to a tower 96 px
///   over the floor
/// - middle: two chimneys for wall-jump bouncing, a narrow one (24 px inside) and a wide
///   one (40 px), open below, lidded with one-way platforms (drops of 136 and 160 px:
///   rolls, hard landings, dives)
/// - right: a low ceiling for corner corrections, a step up to a ledge over the pit, a
///   free pillar to climb, dash crystals between
/// - `B`: places for boxes (the demo's own; [`Level::parse`] reads them as air): one on
///   the floor between the chimneys, one on the ledge
pub const DEMO: [&str; 27] = [
    "#..............................................#",
    "#..............................................#",
    "#..............................................#",
    "#..........................*...................#",
    "#..............................................#",
    "#.......................=======................#",
    "#.......................#.....#................#",
    "#.......................#.....#....##..........#",
    "#.............=====.....#.....#....##..........#",
    "#.............#...#.....#.....#....##.......*..#",
    "#.............#...#.....#.....#....##..........#",
    "#.............#...#.....#.....#....##..........#",
    "#.............#...#.....#.....#....##..........#",
    "#####.........#...#..*..#.....#....##..........#",
    "#.............#...#.....#.....#...........B....#",
    "#.............#...#.....#.....#.........########",
    "#....====.....#...#.....#.....#................#",
    "#.............#...#.....#.....#................#",
    "#.............#...#.....#.....#........===.....#",
    "#####.........#...#.....#.....#................#",
    "#.............#...#.....#.....#................#",
    "#.............#...#..............######........#",
    "#....====......................................#",
    "#..............................................#",
    "#..........P.........B.........................#",
    "##########################################...###",
    "##########################################...###",
];

/// What fills one tile. New kinds may come, so matches need a `_` arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Tile {
    /// Air.
    Empty,
    /// Blocks from every side; plain ground ([`Surface::DEFAULT`]), climbable.
    Solid,
    /// Blocks only from above, and only while not dropping through.
    Platform,
    /// Your game's own tile, by a number of your choosing: spikes, water, a checkpoint, the
    /// goal, ice. The movement treats it as air unless [`Level::define`] says it is solid
    /// or a one-way platform; find the ones the character touches with
    /// [`Level::tiles_touching`].
    ///
    /// Unlike a [`CustomId`](super::CustomId), the number is not a name hash, on purpose:
    /// one byte per tile keeps the grid small, and the numbers belong to the level, which
    /// one game builds. A crate that ships a tile kind (spikes, water) takes its number as
    /// a parameter, so the game hands out the numbers and two crates never collide.
    Custom(u8),
}

/// Tiles on a grid, with refills. Outside the grid, the sides and the top are
/// open air, and below the bottom the character falls out. Its parts are read and changed
/// through methods, which keep them consistent (the grid with its size, the refills with
/// the tile size).
#[derive(Clone, Debug, PartialEq)]
pub struct Level {
    /// Width in tiles.
    w: i32,
    /// Height in tiles.
    h: i32,
    /// Row-major tiles, top row first.
    tiles: Vec<Tile>,
    /// The spawn tile.
    spawn: (i32, i32),
    /// The dash refills.
    crystals: Vec<Crystal>,
    /// Seconds before a used refill comes back.
    refill_back: f32,
    /// The size of a tile in units ([`TILE`] unless [`Level::with_tile_size`] changed it).
    tile_size: i32,
    /// What custom tiles are to the movement, by id (missing: air).
    custom: BTreeMap<u8, CustomTile>,
}

impl Level {
    /// Parse a level from tile rows (see [`DEMO`]): `#` solid, `=` one-way platform, `*`
    /// dash refill, `P` spawn, anything else air. Panics on ragged rows: levels
    /// written in code should fail loudly on a typo.
    pub fn parse(rows: &[&str]) -> Self {
        Self::parse_with(rows, |_| None)
    }

    /// Like [`Level::parse`], with characters of your own: `tile(c)` is asked first for
    /// each one, and a `Some` wins over the built-in meaning (so `Some(Tile::Custom(0))`
    /// for `^` makes spikes, and `Some(Tile::Empty)` for `*` leaves the refills out). A
    /// spawn marker of your own: [`Level::find`] it and [`Level::set_spawn_tile`] there.
    ///
    /// ```
    /// use game_toolkit_platformer::sim::{Level, Rect, Tile};
    ///
    /// const SPIKES: u8 = 0;
    /// let level = Level::parse_with(&["....", ".^^.", "####"], |c| match c {
    ///     '^' => Some(Tile::Custom(SPIKES)),
    ///     _ => None,
    /// });
    /// let feet = Rect { x: 10, y: 4, w: 6, h: 12 };
    /// assert!(level.tiles_touching(feet).any(|(_, _, t)| t == Tile::Custom(SPIKES)));
    ///
    /// // `S`, a start pad of your own, is the spawn.
    /// const PAD: u8 = 1;
    /// let mut level = Level::parse_with(&["..S.", "####"], |c| (c == 'S').then_some(Tile::Custom(PAD)));
    /// let (tx, ty) = level.find(Tile::Custom(PAD)).unwrap();
    /// level.set_spawn_tile(tx, ty);
    /// assert_eq!(level.spawn_tile(), (2, 0));
    /// ```
    pub fn parse_with(rows: &[&str], tile: impl Fn(char) -> Option<Tile>) -> Self {
        let h = rows.len() as i32;
        let w = rows.first().map_or(0, |r| r.chars().count()) as i32;
        for (y, row) in rows.iter().enumerate() {
            let n = row.chars().count() as i32;
            assert_eq!(n, w, "level row {y} is {n} wide, not {w}");
        }
        let grid: Vec<Vec<char>> = rows.iter().map(|r| r.chars().collect()).collect();
        let mut level = Self::from_fn(w, h, |x, y| {
            let c = grid[y as usize][x as usize];
            tile(c).unwrap_or(match c {
                '#' => Tile::Solid,
                '=' => Tile::Platform,
                _ => Tile::Empty,
            })
        });
        for (y, row) in grid.iter().enumerate() {
            for (x, &c) in row.iter().enumerate() {
                if tile(c).is_some() {
                    continue;
                }
                let at = (x as i32, y as i32);
                match c {
                    'P' => level.spawn = at,
                    '*' => level.add_refill(at.0, at.1),
                    _ => {}
                }
            }
        }
        level
    }

    /// A `w` x `h` level whose tile at `x, y` is `tile(x, y)`: the way in from your own
    /// tilemap. No refills, the spawn at the top-left; add them as you like.
    pub fn from_fn(w: i32, h: i32, tile: impl Fn(i32, i32) -> Tile) -> Self {
        let tiles = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| tile(x, y))
            .collect();
        Self {
            w,
            h,
            tiles,
            spawn: (1, 1),
            crystals: Vec::new(),
            refill_back: REFILL_BACK,
            tile_size: TILE,
            custom: BTreeMap::new(),
        }
    }

    /// This level with tiles of `size` units (refills scaled along). For a tilemap whose
    /// tiles are bigger than the feel's 8 units; to keep the feel at any tile size, keep
    /// 8-unit tiles and draw them scaled, or use a `Builder::scale` with your own terrain.
    pub fn with_tile_size(mut self, size: i32) -> Self {
        assert!(size >= 1, "a tile of {size} units");
        let old = self.tile_size;
        for c in &mut self.crystals {
            let a = &mut c.area;
            (a.x, a.y) = (a.x.div_euclid(old) * size, a.y.div_euclid(old) * size);
            (a.w, a.h) = (size, size);
        }
        self.tile_size = size;
        self
    }

    /// Make custom tile `id` `tile` to the movement: solid ground of some [`Surface`] (ice,
    /// a conveyor, a wall that can't be climbed), a one-way platform
    /// ([`CustomTile::platform`]: a bouncy ledge), or air again with
    /// `CustomTile::default()`.
    ///
    /// ```
    /// use game_toolkit_platformer::sim::{CustomTile, Level, Rect, Surface, Tile};
    ///
    /// const ICE: u8 = 0;
    /// let mut level = Level::parse_with(&["....", "~~~~"], |c| (c == '~').then_some(Tile::Custom(ICE)));
    /// level.define(ICE, CustomTile::solid(Surface { friction: 0.1, ..Surface::DEFAULT }));
    /// let feet = Rect { x: 8, y: 8, w: 6, h: 1 };
    /// assert!(level.solid_tile(1, 1));
    /// assert_eq!(level.surface_at(feet).friction, 0.1);
    /// ```
    pub fn define(&mut self, id: u8, tile: CustomTile) {
        self.custom.insert(id, tile);
    }

    /// What custom tile `id` is to the movement (air unless [`Level::define`]d).
    pub fn custom_tile(&self, id: u8) -> CustomTile {
        self.custom.get(&id).copied().unwrap_or_default()
    }

    /// Width in tiles.
    pub fn width(&self) -> i32 {
        self.w
    }

    /// Height in tiles.
    pub fn height(&self) -> i32 {
        self.h
    }

    /// The size of a tile in units: [`TILE`] unless [`Level::with_tile_size`] changed it.
    pub fn tile_size(&self) -> i32 {
        self.tile_size
    }

    /// The spawn tile (the character stands on the tile below it; see [`Level::start`]).
    pub fn spawn_tile(&self) -> (i32, i32) {
        self.spawn
    }

    /// Move the spawn to tile `tx, ty`: a marker of your own (see [`Level::parse_with`]).
    pub fn set_spawn_tile(&mut self, tx: i32, ty: i32) {
        self.spawn = (tx, ty);
    }

    /// The dash refills, with whether each is there ([`Crystal::ready`]): for drawing.
    pub fn refills(&self) -> &[Crystal] {
        &self.crystals
    }

    /// Add a dash refill filling tile `tx, ty` (of this level's tile size).
    pub fn add_refill(&mut self, tx: i32, ty: i32) {
        self.crystals.push(Crystal::at((tx, ty), self.tile_size));
    }

    /// Seconds before a used refill comes back.
    pub fn refill_back(&self) -> f32 {
        self.refill_back
    }

    /// Make used refills come back after `seconds`.
    pub fn set_refill_back(&mut self, seconds: f32) {
        self.refill_back = seconds;
    }

    /// The first tile that is `tile`, row by row: where a marker of your own is.
    pub fn find(&self, tile: Tile) -> Option<(i32, i32)> {
        let i = self.tiles.iter().position(|&t| t == tile)? as i32;
        Some((i % self.w, i / self.w))
    }

    /// The demo level.
    pub fn demo() -> Self {
        Self::parse(&DEMO)
    }

    /// Where a character starts: standing on the bottom of the spawn tile, centered on it
    /// (units).
    pub fn start(&self) -> Spot {
        let (sx, sy) = self.spawn;
        let t = self.tile_size;
        Spot::Feet(sx * t + t / 2, (sy + 1) * t)
    }

    /// The tile range `r` overlaps: `(tx0, ty0, tx1, ty1)`, inclusive.
    fn tile_span(&self, r: Rect) -> (i32, i32, i32, i32) {
        let t = self.tile_size;
        (
            r.x.div_euclid(t),
            r.y.div_euclid(t),
            (r.right() - 1).div_euclid(t),
            (r.bottom() - 1).div_euclid(t),
        )
    }

    /// The tile at `tx, ty`; outside the grid, air.
    pub fn tile(&self, tx: i32, ty: i32) -> Tile {
        if (0..self.w).contains(&tx) && (0..self.h).contains(&ty) {
            self.tiles[(ty * self.w + tx) as usize]
        } else {
            Tile::Empty
        }
    }

    /// Change the tile at `tx, ty` (outside the grid: nothing happens).
    pub fn set(&mut self, tx: i32, ty: i32, tile: Tile) {
        if (0..self.w).contains(&tx) && (0..self.h).contains(&ty) {
            self.tiles[(ty * self.w + tx) as usize] = tile;
        }
    }

    /// The tiles `r` overlaps, as `(tx, ty, tile)`, row by row: what the character's
    /// hitbox touches (spikes, water, the goal), with `Tile::Custom` tiles of your own.
    pub fn tiles_touching(&self, r: Rect) -> impl Iterator<Item = (i32, i32, Tile)> + '_ {
        let (tx0, ty0, tx1, ty1) = self.tile_span(r);
        (ty0..=ty1).flat_map(move |ty| (tx0..=tx1).map(move |tx| (tx, ty, self.tile(tx, ty))))
    }

    /// The first of `hitboxes` that touches a `tile`, if any: pass
    /// [`Platformer::hitboxes`](crate::Platformer::hitboxes) to test every step of the last
    /// frame. The result is a copy, so you can act on the character right away:
    /// `if level.touches(hero.hitboxes(), SPIKES).is_some() { hero.respawn(); }`.
    pub fn touches(&self, hitboxes: &[Rect], tile: Tile) -> Option<Rect> {
        hitboxes
            .iter()
            .copied()
            .find(|&r| self.tiles_touching(r).any(|(_, _, t)| t == tile))
    }

    /// What tile `tx, ty` is to the movement: [`Tile::Solid`] and [`Tile::Platform`] as
    /// plain ground, a custom tile as [`Level::define`]d, the rest air.
    pub(crate) fn tile_kind(&self, tx: i32, ty: i32) -> CustomTile {
        match self.tile(tx, ty) {
            Tile::Solid => CustomTile::solid(Surface::DEFAULT),
            Tile::Platform => CustomTile::platform(Surface::DEFAULT),
            Tile::Custom(id) => self.custom_tile(id),
            Tile::Empty => CustomTile::default(),
        }
    }

    /// Whether tile `tx, ty` is solid: [`Tile::Solid`], or a custom tile
    /// [`Level::define`]d solid.
    pub fn solid_tile(&self, tx: i32, ty: i32) -> bool {
        self.tile_kind(tx, ty).hold == Hold::Solid
    }

    /// Whether tile `tx, ty` is a one-way platform: [`Tile::Platform`], or a custom tile
    /// [`Level::define`]d one.
    pub fn platform_tile(&self, tx: i32, ty: i32) -> bool {
        self.tile_kind(tx, ty).hold == Hold::Platform
    }

    /// The surface `r` touches (the row under the feet, the column beside a wall): of the
    /// holding tile under `r`'s center if there is one, else of the first holding tile it
    /// overlaps, else [`Surface::DEFAULT`]. The same rule as [`Grid::surface_of`].
    pub fn surface_at(&self, r: Rect) -> Surface {
        LevelGrid(self).surface_of(r)
    }

    /// Whether `r` overlaps any solid tile (platforms don't count).
    pub fn overlaps(&self, r: Rect) -> bool {
        LevelGrid(self).overlaps_solid(r)
    }

    /// Whether `r` stands exactly on top of a one-way platform.
    pub fn on_platform(&self, r: Rect) -> bool {
        LevelGrid(self).on_platform(r)
    }

    /// The bottom of the level in units.
    pub fn floor_y(&self) -> i32 {
        self.h * self.tile_size
    }
}

impl Terrain for Level {
    fn solid(&self, r: Rect) -> bool {
        self.overlaps(r)
    }

    fn platform(&self, r: Rect) -> bool {
        self.on_platform(r)
    }

    fn refill(&mut self, r: Rect) -> bool {
        let back = self.refill_back;
        match self
            .crystals
            .iter_mut()
            .find(|c| c.ready() && c.area.intersects(r))
        {
            Some(c) => {
                c.cooldown = back;
                true
            }
            None => false,
        }
    }

    fn fell_out(&self, r: Rect) -> bool {
        r.y > self.floor_y() + 4 * self.tile_size
    }

    fn surface(&self, r: Rect) -> Surface {
        self.surface_at(r)
    }

    fn tick(&mut self, dt: f32, _bodies: &[Rect]) {
        for c in &mut self.crystals {
            c.cooldown = (c.cooldown - dt).max(0.0);
        }
    }

    crate::terrain_layer!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_parses_with_spawn_on_open_ground() {
        let l = Level::demo();
        assert_eq!((l.w, l.h), (48, 27));
        let (sx, sy) = l.spawn;
        assert_eq!(l.tile(sx, sy), Tile::Empty);
        assert!(l.solid_tile(sx, sy + 1));
        assert_eq!(l.crystals.len(), 3);
        assert_eq!(l.spawn, (11, 24));
        assert_eq!(l.tile(21, 24), Tile::Empty, "box places are air");
    }

    #[test]
    fn overlaps_rounds_negative_coordinates_down() {
        let l = Level::parse(&["#.", ".."]);
        assert!(l.overlaps(Rect {
            x: -3,
            y: 0,
            w: 4,
            h: 4
        }));
        assert!(!l.overlaps(Rect {
            x: 8,
            y: 0,
            w: 8,
            h: 8
        }));
        assert!(!l.overlaps(Rect {
            x: -9,
            y: 0,
            w: 4,
            h: 4
        }));
    }

    #[test]
    fn custom_tiles_are_air_and_can_be_found() {
        let l = Level::parse_with(&["~~P.", "~##*"], |c| (c == '~').then_some(Tile::Custom(7)));
        assert_eq!(l.tile(0, 0), Tile::Custom(7));
        assert_eq!(l.spawn, (2, 0), "built-in characters still work");
        assert_eq!(l.crystals.len(), 1);
        let r = Rect {
            x: 0,
            y: 0,
            w: 8,
            h: 16,
        };
        assert!(!l.overlaps(r), "custom tiles are air");
        let found: Vec<_> = l.tiles_touching(r).collect();
        assert_eq!(
            found,
            vec![(0, 0, Tile::Custom(7)), (0, 1, Tile::Custom(7))]
        );
        // A character can be overridden too.
        let l = Level::parse_with(&["*"], |c| (c == '*').then_some(Tile::Empty));
        assert!(l.crystals.is_empty());
    }

    #[test]
    fn refills_added_after_a_new_tile_size_fill_their_tile() {
        let mut l = Level::parse(&["*...", "...."]).with_tile_size(16);
        l.add_refill(2, 1);
        let areas: Vec<Rect> = l.refills().iter().map(|c| c.area).collect();
        let tile = |x, y| Rect { x, y, w: 16, h: 16 };
        assert_eq!(areas, vec![tile(0, 0), tile(32, 16)]);
        assert_eq!((l.width(), l.height(), l.tile_size()), (4, 2, 16));
    }

    #[test]
    fn a_custom_tile_can_be_a_one_way_platform_and_the_spawn() {
        let mut l = Level::parse_with(&["..S.", "~~~~"], |c| match c {
            '~' => Some(Tile::Custom(3)),
            'S' => Some(Tile::Custom(4)),
            _ => None,
        });
        let bouncy = Surface {
            bounce: 0.5,
            ..Surface::DEFAULT
        };
        l.define(3, CustomTile::platform(bouncy));
        let feet = Rect {
            x: 2,
            y: -4,
            w: 4,
            h: 12,
        };
        assert!(l.on_platform(feet) && l.platform_tile(0, 1));
        assert!(!l.overlaps(feet.shifted(0, 4)), "not solid");
        assert_eq!(l.surface_at(feet.shifted(0, 8)).bounce, 0.5);
        assert_eq!(l.find(Tile::Custom(4)), Some((2, 0)));
        assert_eq!(l.find(Tile::Solid), None);
        l.set_spawn_tile(2, 0);
        assert_eq!(l.start(), Spot::Feet(20, 8));
    }

    #[test]
    fn platforms_hold_only_from_exactly_above() {
        let l = Level::parse(&["..", "=="]);
        let feet_on = Rect {
            x: 2,
            y: -4,
            w: 4,
            h: 12,
        };
        assert!(l.on_platform(feet_on));
        assert!(!l.on_platform(feet_on.shifted(0, 1)));
        assert!(!l.overlaps(feet_on.shifted(0, 4)));
    }
}
