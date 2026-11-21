//! The actions an entity can take, and what they cost (R4 to R6).

use ash_core::entity::Speed;
use ash_core::tile::Direction;
use ash_core::time::ActionCost;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Step to the neighboring tile in a direction.
    Move(Direction),
    /// Let time pass. Always possible (R14).
    Wait,
}

impl Action {
    /// R6: the base cost (R5) divided by the entity's speed.
    pub fn cost(self, speed: Speed) -> ActionCost {
        let base_cost = match self {
            Self::Move(_) => ActionCost::new(100),
            Self::Wait => ActionCost::new(50),
        };
        base_cost.divided_by(speed)
    }
}

/// A move the rules refuse (R10, R11). It is not an action: it costs nothing,
/// and nothing in the world changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refused;

/// An action offered to the awaited character, with the cost it would pay (R9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AvailableAction {
    pub action: Action,
    pub cost: ActionCost,
}
