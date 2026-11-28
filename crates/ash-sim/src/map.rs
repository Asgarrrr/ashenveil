//! The fixed map of the skeleton: floor and walls on one level.

use alloc::vec::Vec;

use ash_core::tile::{Direction, Terrain, TilePosition};

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

    /// R4, R11: every floor tile that moves reach from `from`, a floor tile,
    /// with the fewest moves to it. Every entity is ignored. `from` comes
    /// first, at zero moves; no other order is promised.
    pub(crate) fn moves_from(&self, from: TilePosition) -> Vec<(TilePosition, u32)> {
        let mut reached = alloc::vec![(from, 0)];
        // The tiles first reached at the last count of moves.
        let mut frontier = alloc::vec![from];
        let mut moves: u32 = 0;
        while !frontier.is_empty() {
            let Some(next) = moves.checked_add(1) else {
                ash_core::invariant!("the map holds more tiles than a count of moves")
            };
            moves = next;
            let mut next_frontier = Vec::new();
            for tile in frontier {
                for direction in Direction::ALL {
                    let to = tile.step(direction);
                    if self.allows_move(tile, direction)
                        && !reached.iter().any(|&(seen, _)| seen == to)
                    {
                        reached.push((to, moves));
                        next_frontier.push(to);
                    }
                }
            }
            frontier = next_frontier;
        }
        reached
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

#[cfg(test)]
mod tests {
    use super::*;

    fn moves(from: TilePosition, to: TilePosition) -> Option<u32> {
        Map::skeleton()
            .moves_from(from)
            .into_iter()
            .find(|&(tile, _)| tile == to)
            .map(|(_, moves)| moves)
    }

    /// R11 lengthens the way to (6, 2): the diagonal from (5, 1) cuts the
    /// corner of the wall at (5, 2), so the way turns at (6, 1).
    #[test]
    fn a_wall_corner_lengthens_the_moves_to_a_tile() {
        let start = TilePosition { x: 1, y: 1, z: 0 };
        assert_eq!(moves(start, TilePosition { x: 6, y: 2, z: 0 }), Some(6));
        // A diagonal is one move (R4), not two.
        assert_eq!(moves(start, TilePosition { x: 2, y: 2, z: 0 }), Some(1));
        assert_eq!(moves(start, start), Some(0));
        // A wall is no tile a move reaches.
        assert_eq!(moves(start, TilePosition { x: 5, y: 2, z: 0 }), None);
    }

    /// The tile east of the last one of a row. Its index is the first tile
    /// of the next row: without the bound, a wall would answer.
    #[test]
    #[should_panic(expected = "is outside the map")]
    fn a_position_east_of_the_map_crashes() {
        let map = Map::skeleton();
        let _ = map.terrain(TilePosition {
            x: map.width(),
            y: 0,
            z: 0,
        });
    }

    /// The map holds level 0 alone. Without the bound, the floor below
    /// would answer.
    #[test]
    #[should_panic(expected = "is outside the map")]
    fn a_position_above_level_0_crashes() {
        let _ = Map::skeleton().terrain(TilePosition { x: 1, y: 1, z: 1 });
    }

    #[test]
    #[should_panic(expected = "is outside the map")]
    fn a_position_south_of_the_map_crashes() {
        let _ = Map::skeleton().terrain(TilePosition { x: 1, y: 6, z: 0 });
    }
}
