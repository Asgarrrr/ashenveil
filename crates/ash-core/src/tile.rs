//! Tiles: the cells of the 3D grid, what they are made of, and the
//! directions between neighbors.

/// What a tile is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terrain {
    Floor,
    Wall,
}

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
        let (east, south) = direction.offset();
        match (
            self.x.checked_add_signed(east),
            self.y.checked_add_signed(south),
        ) {
            (Some(x), Some(y)) => Self { x, y, z: self.z },
            _ => crate::invariant!("{self:?} has no neighbor to the {direction:?}"),
        }
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

impl Direction {
    /// Every direction, clockwise from the north.
    pub const ALL: [Self; 8] = [
        Self::North,
        Self::NorthEast,
        Self::East,
        Self::SouthEast,
        Self::South,
        Self::SouthWest,
        Self::West,
        Self::NorthWest,
    ];

    /// One step, as the change of x (east) and of y (south).
    const fn offset(self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::NorthEast => (1, -1),
            Self::East => (1, 0),
            Self::SouthEast => (1, 1),
            Self::South => (0, 1),
            Self::SouthWest => (-1, 1),
            Self::West => (-1, 0),
            Self::NorthWest => (-1, -1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "invariant violated")]
    fn stepping_out_of_the_coordinate_range_crashes() {
        let _ = TilePosition { x: 0, y: 5, z: 0 }.step(Direction::West);
    }
}
