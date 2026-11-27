//! Effects: the changes a rule returns (ADR 0001, F4). `World::apply` alone
//! writes them to the world.

use alloc::vec::Vec;

use ash_core::entity::{EntityId, Speed};
use ash_core::rng::Rng;
use ash_core::tile::TilePosition;
use ash_core::time::Time;

use crate::controller::Controller;
use crate::perception::PerceivedChange;

/// Everything one resolution changes, in the order `apply` writes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Effects {
    /// The RNG after every draw that led to these effects: the world's own
    /// when nothing drew.
    pub(crate) rng: Rng,
    pub(crate) effects: Vec<Effect>,
}

/// One change to the world.
#[expect(
    dead_code,
    reason = "S1 signatures: resolve returns these from the next commit"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Effect {
    /// The scheduler's next entry, `entity` due at `time`, starts its turn:
    /// the clock runs to `time` (R2, R8). An entity a player connection
    /// controls becomes awaited.
    Turn { entity: EntityId, time: Time },
    /// `entity` now stands at `position` (R4).
    Place {
        entity: EntityId,
        position: TilePosition,
    },
    /// `entity` acts again at `due`, after every entity already due then
    /// (R7, R8).
    Schedule { entity: EntityId, due: Time },
    /// Every character perceives `change` (R16, R17).
    Perceive(PerceivedChange),
    /// A new entity at `position`, due now, with an id never issued before
    /// (F2).
    Create {
        speed: Speed,
        position: TilePosition,
        controller: Controller,
    },
}
