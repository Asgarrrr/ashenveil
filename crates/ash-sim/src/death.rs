//! What a death changes (D1, D2): the entity leaves the world and leaves a
//! corpse on its tile, and every character perceives the death.

use alloc::vec::Vec;

use ash_core::entity::EntityId;
use ash_core::tile::TilePosition;

use crate::effect::Effect;
use crate::perception::{ChangeKind, PerceivedChange};
use crate::world::World;

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
/// character perceives its death at the time of the lethal hit (A11).
pub(crate) fn effects(world: &World, entity: EntityId) -> Vec<Effect> {
    alloc::vec![
        Effect::Remove { entity },
        Effect::Perceive(PerceivedChange {
            time: world.clock(),
            entity,
            kind: ChangeKind::Death,
        }),
    ]
}
