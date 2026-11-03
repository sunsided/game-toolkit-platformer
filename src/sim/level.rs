//! [`Level`]: a ready-made [`Terrain`]: a grid of tiles (solid, one-way platforms), dash
//! refills, pushable boxes and a spawn point. Parse it from ASCII rows, build it from your
//! own tilemap with [`Level::from_fn`], or implement [`Terrain`] for your own world instead.

use super::{Block, Crystal, Rect, Terrain};

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
/// - blocks to push and pull: one on the floor between the chimneys, one on the ledge
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

/// What fills one tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    /// Air.
    Empty,
    /// Blocks from every side; climbable.
    Solid,
    /// Blocks only from above, and only while not dropping through.
    Platform,
}

/// Tiles on a grid, with refills and boxes. Outside the grid, the sides and the top are
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
    /// The pushable boxes.
    pub blocks: Vec<Block>,
    /// Seconds before a used refill comes back.
    pub refill_back: f32,
}

impl Level {
    /// Parse a level from tile rows (see [`DEMO`]): `#` solid, `=` one-way platform, `*`
    /// dash refill, `B` box, `P` spawn, anything else air. Panics on ragged rows: levels
    /// written in code should fail loudly on a typo.
    pub fn parse(rows: &[&str]) -> Self {
        let h = rows.len() as i32;
        let w = rows.first().map_or(0, |r| r.len()) as i32;
        let mut level = Self::from_fn(w, h, |x, y| match rows[y as usize].as_bytes()[x as usize] {
            b'#' => Tile::Solid,
            b'=' => Tile::Platform,
            _ => Tile::Empty,
        });
        for (y, row) in rows.iter().enumerate() {
            assert_eq!(
                row.len() as i32,
                w,
                "level row {y} is {} wide, not {w}",
                row.len()
            );
            for (x, c) in row.bytes().enumerate() {
                let at = (x as i32, y as i32);
                match c {
                    b'P' => level.spawn = at,
                    b'*' => level.crystals.push(Crystal::at(at)),
                    b'B' => level.blocks.push(Block::at(at)),
                    _ => {}
                }
            }
        }
        level
    }

    /// A `w` x `h` level whose tile at `x, y` is `tile(x, y)`: the way in from your own
    /// tilemap. No refills, no boxes, the spawn at the top-left; add them as you like.
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
            blocks: Vec::new(),
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

    /// Whether tile `tx, ty` is solid.
    pub fn solid_tile(&self, tx: i32, ty: i32) -> bool {
        self.tile(tx, ty) == Tile::Solid
    }

    /// Whether `r` overlaps any solid tile or box (platforms don't count).
    pub fn overlaps(&self, r: Rect) -> bool {
        self.overlaps_except(r, None)
    }

    /// Whether `r` overlaps any solid tile or box other than box `skip`.
    pub fn overlaps_except(&self, r: Rect, skip: Option<usize>) -> bool {
        let (tx0, ty0) = (r.x.div_euclid(TILE), r.y.div_euclid(TILE));
        let (tx1, ty1) = (
            (r.right() - 1).div_euclid(TILE),
            (r.bottom() - 1).div_euclid(TILE),
        );
        (ty0..=ty1).any(|ty| (tx0..=tx1).any(|tx| self.solid_tile(tx, ty)))
            || self
                .blocks
                .iter()
                .enumerate()
                .any(|(i, b)| Some(i) != skip && b.rect().intersects(r))
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

    fn pushable(&self, r: Rect) -> Option<usize> {
        self.blocks
            .iter()
            .position(|b| b.grounded && b.rect().intersects(r))
    }

    fn shove(&mut self, id: usize, dx: f32, hero: Rect) -> i32 {
        // Take the box out while it moves, so it doesn't collide with itself.
        let mut b = self.blocks[id];
        let moved = b.shove(self, id, hero, dx);
        self.blocks[id] = b;
        moved
    }

    fn fell_out(&self, r: Rect) -> bool {
        r.y > self.floor_y() + 4 * TILE
    }

    fn update(&mut self, dt: f32, hero: Rect) {
        for c in &mut self.crystals {
            c.cooldown = (c.cooldown - dt).max(0.0);
        }
        for i in 0..self.blocks.len() {
            let mut b = self.blocks[i];
            b.fall(self, i, hero, dt);
            self.blocks[i] = b;
        }
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
        assert_eq!(l.blocks.len(), 2);
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
