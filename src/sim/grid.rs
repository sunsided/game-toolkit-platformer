//! [`Grid`]: a tilemap of your own, one answer per tile, turned into the [`Terrain`]
//! rules the movement relies on (solid ground, one-way platforms held only from exactly
//! above, the surface under the feet or beside the body) by [`GridTerrain`]. [`Level`]
//! answers by the same rules.
//!
//! ```
//! use game_toolkit_platformer::sim::{CustomTile, Grid, GridTerrain, Rect, Surface, Terrain};
//!
//! /// The game's own map: `#` ground, `~` ice, `=` a one-way platform.
//! struct Map(Vec<&'static [u8]>);
//!
//! impl Grid for Map {
//!     fn tile_size(&self) -> i32 {
//!         16
//!     }
//!     fn tile(&self, tx: i32, ty: i32) -> CustomTile {
//!         let at = usize::try_from(ty).ok().and_then(|y| self.0.get(y));
//!         match at.zip(usize::try_from(tx).ok()).and_then(|(row, x)| row.get(x)) {
//!             Some(b'#') => CustomTile::solid(Surface::DEFAULT),
//!             Some(b'~') => CustomTile::solid(Surface { friction: 0.1, ..Surface::DEFAULT }),
//!             Some(b'=') => CustomTile::platform(Surface::DEFAULT),
//!             _ => CustomTile::default(),
//!         }
//!     }
//! }
//!
//! let map = Map(vec![b"..==", b"....", b"##~~"]);
//! let view = GridTerrain(&map); // borrows: the game keeps its map
//! let body = |x, y| Rect { x, y, w: 12, h: 24 };
//! assert!(view.solid(body(0, 10)) && !view.solid(body(0, 8)));
//! assert!(view.platform(body(34, -24)), "standing exactly on the platform");
//! assert!(!view.platform(body(34, -20)), "not from inside it");
//! let feet = Rect { x: 40, y: 32, w: 12, h: 1 }; // the row under a body standing on ice
//! assert_eq!(view.surface(feet).friction, 0.1);
//! ```

use super::{CustomTile, Hold, Level, Rect, Surface, Terrain};

/// A tilemap: its tile size and what each tile is to the movement. Write these two;
/// [`GridTerrain`] turns them into a [`Terrain`], and the provided methods answer the
/// way [`Level`] does.
pub trait Grid {
    /// Tile size in the terrain's space (pixels with a `Builder::scale`).
    fn tile_size(&self) -> i32;

    /// What the tile at `tx, ty` is to the movement: air (the default), solid, or a
    /// one-way platform, with its [`Surface`]. Outside the map, air (or solid, to wall it
    /// in).
    fn tile(&self, tx: i32, ty: i32) -> CustomTile;

    /// Whether `r` has fallen out of the map (a pit): never, unless you say so.
    fn fell_out(&self, _r: Rect) -> bool {
        false
    }

    /// The tiles `r` overlaps: the first and last column and row.
    fn span(&self, r: Rect) -> (i32, i32, i32, i32) {
        let t = self.tile_size();
        (
            r.x.div_euclid(t),
            r.y.div_euclid(t),
            (r.right() - 1).div_euclid(t),
            (r.bottom() - 1).div_euclid(t),
        )
    }

    /// Whether `r` overlaps a solid tile (platforms don't count).
    fn overlaps_solid(&self, r: Rect) -> bool {
        let (tx0, ty0, tx1, ty1) = self.span(r);
        (ty0..=ty1).any(|ty| (tx0..=tx1).any(|tx| self.tile(tx, ty).hold == Hold::Solid))
    }

    /// Whether `r` stands exactly on top of a one-way platform: its bottom edge on the
    /// tile row's top, a platform tile under any of its columns.
    fn on_platform(&self, r: Rect) -> bool {
        let t = self.tile_size();
        if r.bottom().rem_euclid(t) != 0 {
            return false;
        }
        let ty = r.bottom().div_euclid(t);
        let (tx0, _, tx1, _) = self.span(r);
        (tx0..=tx1).any(|tx| self.tile(tx, ty).hold == Hold::Platform)
    }

    /// The surface `r` touches (the row under the feet, the column beside a wall): of the
    /// holding tile under `r`'s center if there is one, else of the first holding tile it
    /// overlaps (row by row), else [`Surface::DEFAULT`].
    fn surface_of(&self, r: Rect) -> Surface {
        let t = self.tile_size();
        let holding = |tx, ty| {
            let c = self.tile(tx, ty);
            (c.hold != Hold::Air).then_some(c.surface)
        };
        let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
        if let Some(s) = holding(cx.div_euclid(t), cy.div_euclid(t)) {
            return s;
        }
        let (tx0, ty0, tx1, ty1) = self.span(r);
        (ty0..=ty1)
            .flat_map(|ty| (tx0..=tx1).map(move |tx| (tx, ty)))
            .find_map(|(tx, ty)| holding(tx, ty))
            .unwrap_or_default()
    }
}

impl<G: Grid + ?Sized> Grid for &G {
    fn tile_size(&self) -> i32 {
        (**self).tile_size()
    }

    fn tile(&self, tx: i32, ty: i32) -> CustomTile {
        (**self).tile(tx, ty)
    }

    fn fell_out(&self, r: Rect) -> bool {
        (**self).fell_out(r)
    }
}

/// A [`Grid`] as a [`Terrain`]: solid tiles, one-way platforms, surfaces and pits by the
/// grid's answers. Wrap a borrow of your map (`GridTerrain(&map)`) to step through it
/// each frame, or own it.
///
/// It builds on nothing and isn't findable by `find_mut` (it may borrow). An ability
/// that needs your map: write the [`Terrain`] yourself, forwarding to the grid's
/// [`Grid::overlaps_solid`], [`Grid::on_platform`] and [`Grid::surface_of`].
#[derive(Clone, Copy, Debug)]
pub struct GridTerrain<G>(pub G);

impl<G: Grid> Terrain for GridTerrain<G> {
    fn solid(&self, r: Rect) -> bool {
        self.0.overlaps_solid(r)
    }

    fn platform(&self, r: Rect) -> bool {
        self.0.on_platform(r)
    }

    fn surface(&self, r: Rect) -> Surface {
        self.0.surface_of(r)
    }

    fn fell_out(&self, r: Rect) -> bool {
        self.0.fell_out(r)
    }

    crate::terrain_layer!(borrowed);
}

/// A [`Level`]'s tiles as a [`Grid`], so it answers by the same rules.
pub(crate) struct LevelGrid<'a>(pub &'a Level);

impl Grid for LevelGrid<'_> {
    fn tile_size(&self) -> i32 {
        self.0.tile_size()
    }

    fn tile(&self, tx: i32, ty: i32) -> CustomTile {
        self.0.tile_kind(tx, ty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::Tile;

    /// A level's tiles, through the public [`Grid`] only.
    struct Mirror(Level);

    impl Grid for Mirror {
        fn tile_size(&self) -> i32 {
            self.0.tile_size()
        }

        fn tile(&self, tx: i32, ty: i32) -> CustomTile {
            match self.0.tile(tx, ty) {
                Tile::Solid => CustomTile::solid(Surface::DEFAULT),
                Tile::Platform => CustomTile::platform(Surface::DEFAULT),
                Tile::Custom(id) => self.0.custom_tile(id),
                Tile::Empty => CustomTile::default(),
            }
        }
    }

    #[test]
    fn a_grid_answers_like_the_equivalent_level() {
        let mut level =
            Level::parse_with(
                &["....==..", ".~~.....", "##ii##==", "........"],
                |c| match c {
                    '~' => Some(Tile::Custom(1)),
                    'i' => Some(Tile::Custom(2)),
                    _ => None,
                },
            );
        let ice = Surface {
            friction: 0.1,
            ..Surface::DEFAULT
        };
        level.define(1, CustomTile::platform(Surface { bounce: 0.5, ..ice }));
        level.define(2, CustomTile::solid(ice));
        let grid = GridTerrain(Mirror(level.clone()));
        for y in -14..34 {
            for x in -8..66 {
                for (w, h) in [(6, 12), (3, 3), (12, 2)] {
                    let r = Rect { x, y, w, h };
                    assert_eq!(grid.solid(r), level.solid(r), "{r:?}");
                    assert_eq!(grid.platform(r), level.platform(r), "{r:?}");
                    assert_eq!(grid.surface(r), level.surface(r), "{r:?}");
                }
            }
        }
    }
}
