//! What characters learn of the world (R15 to R18).

use ash_core::entity::EntityId;
use ash_core::time::Time;

use crate::action::Action;

/// One change, stamped with the time it happened (R17). It names the action,
/// never the entity's speed, cost or next turn (R18).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerceivedChange {
    pub time: Time,
    pub entity: EntityId,
    pub action: Action,
    /// The entity a move bumped into (R13): the mover stayed in place. None
    /// for a move that stepped, and for a wait.
    pub bumped: Option<EntityId>,
}
