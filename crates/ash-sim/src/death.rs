//! What a death changes (D1, D2): the entity leaves the world and leaves a
//! corpse on its tile, and every character perceives the death. A dead
//! character is followed by a stranger (D6).

use alloc::vec::Vec;

use ash_core::entity::{EntityId, Speed};
use ash_core::tile::TilePosition;

use crate::combat::Health;
use crate::controller::Controller;
use crate::effect::Effect;
use crate::perception::{ChangeKind, PerceivedChange};
use crate::world::{self, World};

/// D6, R6: the speed of a character, which a stranger has.
const CHARACTER_SPEED: u64 = 1;

/// What a death leaves on the tile where the entity died (D2). Not an
/// entity: it has no id of its own, no scheduler entry and no controller.
/// It never acts and never blocks (D3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Corpse {
    /// The entity that died. Its id stays issued and is never embodied again
    /// (F2).
    pub(crate) entity: EntityId,
    pub(crate) position: TilePosition,
    pub(crate) died: Died,
}

/// What died: a character or a creature (D4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Died {
    Character,
    Creature,
}

/// D1, D2: the effects of the death of `entity`, which is embodied, in
/// order: it leaves the world, its corpse lies on its tile, and every
/// character perceives its death at the time of the lethal hit (A11). A
/// dead character is then followed by its stranger (D6).
pub(crate) fn effects(world: &World, entity: EntityId) -> Vec<Effect> {
    let died = match world.controller(entity) {
        Controller::Player => Died::Character,
        Controller::Wander(_) => Died::Creature,
    };
    let mut effects = alloc::vec![
        Effect::Remove { entity },
        Effect::LeaveCorpse(Corpse {
            entity,
            position: world.position(entity),
            died,
        }),
        Effect::Perceive(PerceivedChange {
            time: world.clock(),
            entity,
            kind: ChangeKind::Death,
        }),
    ];
    if died == Died::Character {
        // D6, A7: a new character, with all of its hit points.
        effects.push(Effect::CreateStranger {
            dead: entity,
            speed: Speed::new(CHARACTER_SPEED),
            position: arrival(world, entity),
            health: Health::full(Controller::Player),
        });
    }
    effects
}

/// D8: where the stranger of `dead` arrives, on the world without `dead`.
/// The tile where the world's first character started, or, when an entity
/// holds it, the free floor tile the fewest moves away, the moves ignoring
/// every entity. Among equals, the first in row order: from the north, each
/// row from the west.
fn arrival(world: &World, dead: EntityId) -> TilePosition {
    let free = |tile| world.occupant(tile).is_none_or(|occupant| occupant == dead);
    let nearest = world
        .map()
        .moves_from(world::CHARACTER_START)
        .into_iter()
        .filter(|&(tile, _)| free(tile))
        .min_by_key(|&(tile, moves)| (moves, tile.y, tile.x));
    match nearest {
        Some((tile, _)) => tile,
        None => ash_core::invariant!("no free floor tile is left for the stranger of {dead:?}"),
    }
}
