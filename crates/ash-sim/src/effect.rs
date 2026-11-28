//! Effects: the changes a rule returns (ADR 0001, F4). `World::apply` alone
//! writes them to the world.

use alloc::vec::Vec;

use ash_core::entity::{EntityId, Speed};
use ash_core::rng::Rng;
use ash_core::tile::TilePosition;
use ash_core::time::Time;

use crate::combat::Health;
use crate::controller::{Controller, Disposition};
use crate::death::Corpse;
use crate::perception::PerceivedChange;

/// Everything one resolution changes, applied once.
pub(crate) struct Effects {
    /// Written in this order.
    pub(crate) in_order: Vec<Effect>,
    /// The RNG state after every draw that led to these effects: the
    /// world's own when nothing drew.
    pub(crate) rng_after: Rng,
}

/// One change to the world.
pub(crate) enum Effect {
    /// `entity` now stands at `position` (R4).
    Place {
        entity: EntityId,
        position: TilePosition,
    },
    /// A hit leaves `entity` these hit points (A6).
    Damage { entity: EntityId, health: Health },
    /// `entity` leaves the world: it is no longer embodied (A8). Its id
    /// stays issued, and the scheduler keeps its entry (F2, F3).
    Remove { entity: EntityId },
    /// The creature `entity` now has this disposition (A4).
    ChangeDisposition {
        entity: EntityId,
        disposition: Disposition,
    },
    /// A death leaves this corpse, after every corpse of an earlier death
    /// (D2). No effect removes or moves a corpse.
    LeaveCorpse(Corpse),
    /// Every character perceives `change` (R16, R17).
    Perceive(PerceivedChange),
    /// The turn of the scheduler's next entity ends: it is due again at
    /// `due`, after every entity already due then (R7, R8). The clock and
    /// the awaited entity follow the scheduler.
    EndTurn { due: Time },
    /// The scheduler's next entry leaves it: its entity is not embodied, so
    /// it has no turn to take (F3).
    Unschedule,
    /// A new entity at `position`, due now: at time zero when it is the
    /// first of its world. The world issues its id as it writes this (F2).
    Create {
        speed: Speed,
        position: TilePosition,
        controller: Controller,
        health: Health,
    },
    /// The stranger of the dead character `dead`: a new character at
    /// `position`, due now, after every entity already due (D6, D9). The
    /// world issues its id as it writes this, records it as the stranger of
    /// `dead` (F2), and every character perceives its arrival at `time`
    /// (D10).
    CreateStranger {
        dead: EntityId,
        time: Time,
        speed: Speed,
        position: TilePosition,
        health: Health,
    },
}
