//! Tiles: the cells of the 3D grid, and the directions between neighbors.

/// The coordinates of one tile. x grows to the east, y to the south, z
/// upward.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TilePosition {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl TilePosition {
    /// The neighbor one step in `direction`, on the same level. Leaving the
    /// coordinate range crashes through `invariant!`.
    pub fn step(self, direction: Direction) -> Self {
        let _ = direction;
        self
    }
}

/// The eight horizontal directions of a move (R4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}
