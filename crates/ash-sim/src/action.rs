//! The actions an entity can take, what they cost (R4 to R6), and the rule
//! that resolves them (ADR 0001, F4).

use alloc::vec::Vec;

use ash_core::entity::{EntityId, Speed};
use ash_core::rng::Rng;
use ash_core::tile::Direction;
use ash_core::time::ActionCost;

use crate::combat;
use crate::effect::{Effect, Effects};
use crate::perception::{Outcome, PerceivedChange, Perception};
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

/// R10, R11, R14, A3: whether the rules let `actor` take `action`, judged on
/// what it perceives alone. The offers are the actions it allows, so an
/// offered action is never refused.
pub(crate) fn allowed(perception: &Perception<'_>, actor: EntityId, action: Action) -> bool {
    let from = perception.position(actor);
    match action {
        Action::Wait => true,
        Action::Move(direction) => perception.map().allows_move(from, direction),
        Action::Attack(direction) => perception.occupant(from.step(direction)).is_some(),
    }
}

/// R14: every action `actor` is allowed, the wait first, then the moves and
/// the attacks clockwise from the north. The same for every controller (R3).
pub(crate) fn offers<'p>(
    perception: &'p Perception<'_>,
    actor: EntityId,
) -> impl Iterator<Item = Action> + 'p {
    let moves = Direction::ALL.map(Action::Move);
    let attacks = Direction::ALL.map(Action::Attack);
    core::iter::once(Action::Wait)
        .chain(moves)
        .chain(attacks)
        .filter(move |&action| allowed(perception, actor, action))
}

/// F4: what `actor` taking `action` changes, or `Refused` when `allowed`
/// says no. Pure: the refusal reads only `perception`, the outcome reads the
/// true `world` and cannot fail. Every draw starts from `rng`, and the
/// effects carry the RNG state after the last one. A refusal draws nothing.
pub(crate) fn resolve(
    world: &World,
    perception: &Perception<'_>,
    actor: EntityId,
    action: Action,
    rng: &Rng,
) -> Result<Effects, Refused> {
    if !allowed(perception, actor, action) {
        return Err(Refused);
    }
    let mut in_order = Vec::new();
    let (outcome, rng_after) = match action {
        Action::Wait => (Outcome::Waited, rng.clone()),
        Action::Move(direction) => {
            let to = world.position(actor).step(direction);
            // R13: a move into an occupied tile is a bump. The mover stays
            // and pays the move; nothing else changes.
            let outcome = match world.occupant(to) {
                Some(occupant) => Outcome::Bumped(occupant),
                None => {
                    in_order.push(Effect::Place {
                        entity: actor,
                        position: to,
                    });
                    Outcome::Stepped
                }
            };
            (outcome, rng.clone())
        }
        Action::Attack(direction) => combat::attack(world, actor, direction, rng),
    };
    // R7: the effect happens now; the cost delays the actor's next turn.
    in_order.push(Effect::Perceive(PerceivedChange {
        time: world.clock(),
        entity: actor,
        action,
        outcome,
    }));
    in_order.push(Effect::EndTurn {
        due: world.clock().after(action.cost(world.speed(actor))),
    });
    Ok(Effects {
        in_order,
        rng_after,
    })
}
