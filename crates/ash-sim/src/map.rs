//! The fixed map of the skeleton: floor and walls on one level.

use ash_core::tile::{Direction, TilePosition};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Map {}

impl Map {
    /// The fixed map of the skeleton.
    pub fn skeleton() -> Self {
        Self {}
    }

    /// Whether the rules let an entity standing at `from` move in
    /// `direction` (R10, R11).
    pub fn allows_move(&self, from: TilePosition, direction: Direction) -> bool {
        let _ = (from, direction);
        true
    }
}
