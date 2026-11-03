//! [`Level`]: a grid of tiles (solid walls, one-way platforms), dash crystals, block
//! starts and a spawn point.

use super::Rect;

/// Tile size in grid pixels.
pub const TILE: i32 = 8;

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

/// Tiles on a grid. Outside the grid, the sides and the top are open air, so the hero can
/// fall out of the bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Level {
    /// Width in tiles.
    pub w: i32,
    /// Height in tiles.
    pub h: i32,
    /// Row-major tiles, top row first.
    tiles: Vec<Tile>,
    /// The spawn tile.
    pub spawn: (i32, i32),
    /// Tiles holding a dash crystal.
    pub crystals: Vec<(i32, i32)>,
    /// Tiles where a pushable block starts (its bottom on that tile's bottom).
    pub block_spawns: Vec<(i32, i32)>,
    /// Moving solids (the blocks' boxes), kept current by the world each step. They
    /// collide like solid tiles.
    pub blocks: Vec<Rect>,
}

impl Level {
    /// Parse a level from tile rows (see [`DEMO`]). Panics on ragged rows: levels are
    /// authored in code, and a typo should fail loudly.
    pub fn parse(rows: &[&str]) -> Self {
        let h = rows.len() as i32;
        let w = rows.first().map_or(0, |r| r.len()) as i32;
        let mut tiles = Vec::with_capacity((w * h) as usize);
        let mut spawn = (1, 1);
        let mut crystals = Vec::new();
        let mut block_spawns = Vec::new();
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
                    b'P' => spawn = at,
                    b'*' => crystals.push(at),
                    b'B' => block_spawns.push(at),
                    _ => {}
                }
                tiles.push(match c {
                    b'#' => Tile::Solid,
                    b'=' => Tile::Platform,
                    _ => Tile::Empty,
                });
            }
        }
        Self {
            w,
            h,
            tiles,
            spawn,
            crystals,
            block_spawns,
            blocks: Vec::new(),
        }
    }

    /// The demo level.
    pub fn demo() -> Self {
        Self::parse(&DEMO)
    }

    /// The tile at `tx, ty`; outside the grid, air.
    pub fn tile(&self, tx: i32, ty: i32) -> Tile {
        if (0..self.w).contains(&tx) && (0..self.h).contains(&ty) {
            self.tiles[(ty * self.w + tx) as usize]
        } else {
            Tile::Empty
        }
    }

    /// Whether tile `tx, ty` is solid.
    pub fn solid(&self, tx: i32, ty: i32) -> bool {
        self.tile(tx, ty) == Tile::Solid
    }

    /// Whether `r` overlaps any solid tile or block (platforms don't count).
    pub fn overlaps(&self, r: Rect) -> bool {
        self.overlaps_except(r, None)
    }

    /// Whether `r` overlaps any solid tile or block other than block `skip`.
    pub fn overlaps_except(&self, r: Rect, skip: Option<usize>) -> bool {
        let (tx0, ty0) = (r.x.div_euclid(TILE), r.y.div_euclid(TILE));
        let (tx1, ty1) = (
            (r.right() - 1).div_euclid(TILE),
            (r.bottom() - 1).div_euclid(TILE),
        );
        (ty0..=ty1).any(|ty| (tx0..=tx1).any(|tx| self.solid(tx, ty)))
            || self
                .blocks
                .iter()
                .enumerate()
                .any(|(i, b)| Some(i) != skip && b.intersects(r))
    }

    /// The block `r` overlaps, if any.
    pub fn block_at(&self, r: Rect) -> Option<usize> {
        self.blocks.iter().position(|b| b.intersects(r))
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

    /// The bottom of the level in pixels; below it, the hero is lost.
    pub fn floor_y(&self) -> i32 {
        self.h * TILE
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
        assert!(l.solid(sx, sy + 1));
        assert_eq!(l.crystals.len(), 3);
        assert_eq!(l.spawn, (11, 24));
        assert_eq!(l.block_spawns, [(42, 14), (21, 24)]);
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
