//! The fixed map of the skeleton: floor and walls on one level.

use alloc::vec::Vec;

use ash_core::tile::{Direction, TilePosition};

const WIDTH: usize = 9;

/// Row by row from the north, `#` a wall and `.` floor, on level 0. The
/// border is wall: a move off the map would be a crash, not a refusal.
const SKELETON: [&[u8; WIDTH]; 6] = [
    b"#########",
    b"#.......#",
    b"#....#..#",
    b"#.....#.#",
    b"#.......#",
    b"#########",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terrain {
    Floor,
    Wall,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Map {
    width: u32,
    /// Row by row from the north, each row from the west.
    terrain: Vec<Terrain>,
}

impl Map {
    /// The fixed map of the skeleton.
    pub fn skeleton() -> Self {
        let Ok(width) = u32::try_from(WIDTH) else {
            ash_core::invariant!("the map is wider than the coordinate range")
        };
        let terrain = SKELETON
            .iter()
            .flat_map(|row| row.iter())
            .map(|&tile| match tile {
                b'#' => Terrain::Wall,
                b'.' => Terrain::Floor,
                _ => ash_core::invariant!("the map holds the unknown tile {tile:?}"),
            })
            .collect();
        Self { width, terrain }
    }

    /// The number of tiles in each row.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Every tile of level 0, row by row from the north, each row from the
    /// west.
    pub fn tiles(&self) -> &[Terrain] {
        &self.terrain
    }

    /// Whether the rules let an entity standing at `from` move in
    /// `direction`. Under whole-map perception (R16), the character perceives
    /// every wall, so a move onto one is refused (R10). A move between two
    /// tiles may not cut a wall corner (R11).
    pub fn allows_move(&self, from: TilePosition, direction: Direction) -> bool {
        let to = from.step(direction);
        // A diagonal passes between (to.x, from.y) and (from.x, to.y). For a
        // straight move these are `to` and `from`, where the entity stands
        // on floor, so one check covers every direction.
        [
            to,
            TilePosition { y: from.y, ..to },
            TilePosition { x: from.x, ..to },
        ]
        .into_iter()
        .all(|tile| self.terrain(tile) == Terrain::Floor)
    }

    /// The terrain at `position`. A position outside the map crashes through
    /// `invariant!`.
    pub fn terrain(&self, position: TilePosition) -> Terrain {
        let index = (position.z == 0 && position.x < self.width)
            .then(|| position.y.checked_mul(self.width)?.checked_add(position.x))
            .flatten()
            .and_then(|index| usize::try_from(index).ok());
        match index.and_then(|index| self.terrain.get(index)) {
            Some(&terrain) => terrain,
            None => ash_core::invariant!("{position:?} is outside the map"),
        }
    }
}
