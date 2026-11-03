//! [`Level`]: a ready-made [`Terrain`]: a grid of tiles (solid, one-way platforms), dash
//! refills and a spawn point. Parse it from ASCII rows, build it from your own tilemap
//! with [`Level::from_fn`], or implement [`Terrain`] for your own world instead.
//!
//! Tiles of your own ([`Tile::Custom`]: spikes, water, checkpoints) are air to the
//! movement; [`Level::parse_with`] reads them from characters of your choosing and
//! [`Level::tiles_touching`] finds the ones under the character. For more (doors, moving
//! platforms, boxes: state of your own beside the level), wrap it in your own terrain with
//! the level as its [`Terrain::base`], and write only what changes (see
//! [`super::terrain`]).

use std::any::Any;

use super::{Crystal, Rect, Terrain};

/// Tile size in units.
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
    /// Blocks from every side; climbable.
    Solid,
    /// Blocks only from above, and only while not dropping through.
    Platform,
    /// Your game's own tile, by a number of your choosing: spikes, water, a checkpoint, the
    /// goal. The movement treats it as air; find the ones the character touches with
    /// [`Level::tiles_touching`].
    Custom(u8),
}

/// Tiles on a grid, with refills. Outside the grid, the sides and the top are
/// open air, and below the bottom the character falls out.
#[derive(Clone, Debug, PartialEq)]
pub struct Level {
    /// Width in tiles.
    pub w: i32,
    /// Height in tiles.
    pub h: i32,
    /// Row-major tiles, top row first.
    tiles: Vec<Tile>,
    /// The spawn tile.
    pub spawn: (i32, i32),
    /// The dash refills.
    pub crystals: Vec<Crystal>,
    /// Seconds before a used refill comes back.
    pub refill_back: f32,
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
    /// for `^` makes spikes, and `Some(Tile::Empty)` for `*` leaves the refills out).
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
                    '*' => level.crystals.push(Crystal::at(at)),
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
        }
    }

    /// The demo level.
    pub fn demo() -> Self {
        Self::parse(&DEMO)
    }

    /// Where a character starts: the hitbox top-left (units) for a body `w` x `h` standing
    /// on the bottom of the spawn tile, centered on it.
    pub fn spawn_point(&self, w: i32, h: i32) -> (i32, i32) {
        let (sx, sy) = self.spawn;
        (sx * TILE + (TILE - w) / 2, (sy + 1) * TILE - h)
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
        let (tx0, ty0) = (r.x.div_euclid(TILE), r.y.div_euclid(TILE));
        let (tx1, ty1) = (
            (r.right() - 1).div_euclid(TILE),
            (r.bottom() - 1).div_euclid(TILE),
        );
        (ty0..=ty1).flat_map(move |ty| (tx0..=tx1).map(move |tx| (tx, ty, self.tile(tx, ty))))
    }

    /// Whether tile `tx, ty` is solid.
    pub fn solid_tile(&self, tx: i32, ty: i32) -> bool {
        self.tile(tx, ty) == Tile::Solid
    }

    /// Whether `r` overlaps any solid tile (platforms don't count).
    pub fn overlaps(&self, r: Rect) -> bool {
        let (tx0, ty0) = (r.x.div_euclid(TILE), r.y.div_euclid(TILE));
        let (tx1, ty1) = (
            (r.right() - 1).div_euclid(TILE),
            (r.bottom() - 1).div_euclid(TILE),
        );
        (ty0..=ty1).any(|ty| (tx0..=tx1).any(|tx| self.solid_tile(tx, ty)))
    }

    /// Whether `r` stands exactly on top of a one-way platform.
    pub fn on_platform(&self, r: Rect) -> bool {
        if r.bottom().rem_euclid(TILE) != 0 {
            return false;
        }
        let ty = r.bottom().div_euclid(TILE);
        let (tx0, tx1) = (r.x.div_euclid(TILE), (r.right() - 1).div_euclid(TILE));
        (tx0..=tx1).any(|tx| self.tile(tx, ty) == Tile::Platform)
    }

    /// The bottom of the level in units.
    pub fn floor_y(&self) -> i32 {
        self.h * TILE
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
        r.y > self.floor_y() + 4 * TILE
    }

    fn update(&mut self, dt: f32, _bodies: &[Rect]) {
        for c in &mut self.crystals {
            c.cooldown = (c.cooldown - dt).max(0.0);
        }
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }
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
