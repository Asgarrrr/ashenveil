//! Effects: the changes a rule returns (ADR 0001, F4). `World::apply` alone
//! writes them to the world.

use alloc::vec::Vec;

use ash_core::entity::{EntityId, Speed};
use ash_core::rng::Rng;
use ash_core::tile::TilePosition;
use ash_core::time::Time;

use crate::controller::Controller;
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
    /// Every character perceives `change` (R16, R17).
    Perceive(PerceivedChange),
    /// The turn of the scheduler's next entity ends: it is due again at
    /// `due`, after every entity already due then (R7, R8). The clock and
    /// the awaited entity follow the scheduler.
    EndTurn { due: Time },
    /// A new entity at `position`, due now, with an id never issued before
    /// (F2).
    Create {
        speed: Speed,
        position: TilePosition,
        controller: Controller,
    },
}
