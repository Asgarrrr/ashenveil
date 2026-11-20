//! The actions an entity can take, and what they cost (R4 to R6).

use ash_core::entity::Speed;
use ash_core::time::ActionCost;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Let time pass. Always possible (R14).
    Wait,
}

impl Action {
    /// R6: the base cost (R5) divided by the entity's speed.
    pub fn cost(self, _speed: Speed) -> ActionCost {
        ActionCost::new(0)
    }
}

/// An action offered to the awaited character, with the cost it would pay (R9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AvailableAction {
    pub action: Action,
    pub cost: ActionCost,
}
