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

    /// The web client steps a perceived move with its own copy of the
    /// offsets, `STEPS` in `web/src/model.ts`. A copy that drifts would
    /// draw every mover off its true tile.
    #[test]
    fn the_web_client_steps_each_direction_by_the_same_offset() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../web/src/model.ts");
        let model = std::fs::read_to_string(path).expect("the web model is readable");
        let table = model
            .split_once("const STEPS")
            .and_then(|(_, rest)| rest.split_once("};"))
            .map(|(table, _)| table)
            .expect("the web model declares STEPS");
        // One `name: [east, south],` per line.
        let mut steps: Vec<(String, (i32, i32))> = table
            .lines()
            .filter_map(|line| {
                let (name, offset) = line.trim().split_once(": [")?;
                let (east, south) = offset.strip_suffix("],")?.split_once(", ")?;
                Some((name.to_owned(), (east.parse().ok()?, south.parse().ok()?)))
            })
            .collect();
        steps.sort();
        // The web client names a direction as the wire JSON does.
        let mut expected: Vec<(String, (i32, i32))> = Direction::ALL
            .iter()
            .map(|direction| (format!("{direction:?}").to_lowercase(), direction.offset()))
            .collect();
        expected.sort();
        assert_eq!(steps, expected);
    }
}
