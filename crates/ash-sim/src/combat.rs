//! Attacks between entities (A3, A5, A6).

use ash_core::entity::EntityId;
use ash_core::rng::Rng;
use ash_core::tile::Direction;

use crate::action::Refused;
use crate::effect::Effects;
use crate::perception::Perception;
use crate::world::World;

/// The attack rule: `attacker` strikes the entity it perceives toward
/// `direction` (A3). Refused when it perceives none there. One draw from
/// `rng` decides a hit or a miss (A6).
#[expect(
    unused_variables,
    reason = "S1 signatures: the rule is written in the next commit"
)]
pub(crate) fn attack(
    world: &World,
    perception: &Perception<'_>,
    attacker: EntityId,
    direction: Direction,
    rng: &Rng,
) -> Result<Effects, Refused> {
    Err(Refused)
}
