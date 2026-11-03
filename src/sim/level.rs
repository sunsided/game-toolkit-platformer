//! [`Level`]: a grid of tiles (solid walls, one-way platforms), dash crystals and a
//! spawn point.

use super::Rect;

/// Tile size in grid pixels.
pub const TILE: i32 = 8;

/// The demo level, one string per tile row: `#` solid, `=` one-way platform (stand on it,
/// jump up through it, down + jump drops through), `*` a dash crystal, `.` empty, `P` the
/// spawn (the hero stands on the tile below it). 48x27 tiles, exactly one screen.
///
/// A tower slab on the left (a 128 px drop: hard landing or roll), one-way steps up to it,
/// a pit to respawn from, a low ceiling for corner corrections, a pillar to climb, a 48 px
/// ledge on the right for rolls, and crystals to recharge the dash in the air.
pub const DEMO: [&str; 27] = [
    "#..............................................#",
    "#..............................................#",
    "#..............................................#",
    "#..............................................#",
    "#...........................................*..#",
    "#..............................................#",
    "#...................*..........................#",
    "#..............................................#",
    "#.........................................##...#",
    "#######...................................##...#",
    "#.........................................##...#",
    "#.............................#########...##...#",
    "#.........................................##...#",
    "#...........====..........................##...#",
    "#....................####.................##...#",
    "#.........................................##...#",
    "#..............................................#",
    "#.............====.............................#",
    "#.......................................########",
    "#..............................................#",
    "#.........................#####..#####.........#",
    "#........=====.................................#",
    "#....................*.........................#",
    "#..............................................#",
    "#..P...........................................#",
    "####################....########################",
    "####################....########################",
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

    /// Whether `r` overlaps any solid tile (platforms don't count).
    pub fn overlaps(&self, r: Rect) -> bool {
        let (tx0, ty0) = (r.x.div_euclid(TILE), r.y.div_euclid(TILE));
        let (tx1, ty1) = (
            (r.right() - 1).div_euclid(TILE),
            (r.bottom() - 1).div_euclid(TILE),
        );
        (ty0..=ty1).any(|ty| (tx0..=tx1).any(|tx| self.solid(tx, ty)))
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
