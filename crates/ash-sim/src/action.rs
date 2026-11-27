//! The actions an entity can take, what they cost (R4 to R6), and the rule
//! that resolves them (ADR 0001, F4).

use ash_core::entity::{EntityId, Speed};
use ash_core::rng::Rng;
use ash_core::tile::Direction;
use ash_core::time::ActionCost;

use crate::combat;
use crate::effect::Effects;
use crate::perception::Perception;
use crate::world::World;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Step to the neighboring tile in a direction.
    Move(Direction),
    /// Let time pass. Always possible (R14).
    Wait,
    /// Strike the entity on the neighboring tile in a direction (A3).
    Attack(Direction),
}

impl Action {
    /// R6: the base cost (R5, A5) divided by the entity's speed.
    pub fn cost(self, speed: Speed) -> ActionCost {
        let base_cost = match self {
            Self::Move(_) | Self::Attack(_) => ActionCost::new(100),
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

/// F4: what `actor` taking `action` changes, or why the rules refuse it.
/// Pure: a refusal reads only `perception` (R10), an outcome reads the true
/// `world`. Every draw starts from `rng`, and the effects carry the RNG
/// after the last one. A refusal draws nothing.
#[expect(
    dead_code,
    reason = "S1 signatures: World::submit and the wander go through resolve in the next commit"
)]
pub(crate) fn resolve(
    world: &World,
    perception: &Perception<'_>,
    actor: EntityId,
    action: Action,
    rng: &Rng,
) -> Result<Effects, Refused> {
    match action {
        Action::Move(_) | Action::Wait => Err(Refused),
        Action::Attack(direction) => combat::attack(world, perception, actor, direction, rng),
    }
}
