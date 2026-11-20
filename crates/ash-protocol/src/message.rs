//! What the server and a client say to each other.
//!
//! Nothing here can carry another entity's speed, cost or next turn (R18):
//! only the awaited entity's own costs, sent to its controller (R9).

use alloc::vec::Vec;

use ash_core::entity::EntityId;
use ash_core::tile::{Direction, TilePosition};
use ash_core::time::{ActionCost, Time};

/// Identifies one controller of one world across connections. The client
/// chooses it; an unknown token starts a new world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ResumeToken(pub u128);

/// What a tile is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terrain {
    Floor,
    Wall,
}

/// An action an entity can take (R4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Move(Direction),
    Wait,
}

/// An action offered to the awaited entity, with the cost it would pay (R9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AvailableAction {
    pub action: Action,
    pub cost: ActionCost,
}

/// An entity the character perceives, and where it stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerceivedEntity {
    pub entity: EntityId,
    pub position: TilePosition,
}

/// One change, stamped with the time it happened (R17).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerceivedChange {
    pub time: Time,
    pub entity: EntityId,
    pub action: Action,
}

/// Everything the character perceives at `time` (R16).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub time: Time,
    /// The number of tiles in each row of `terrain`. Never zero.
    pub width: u32,
    /// Level 0, row by row from the north, each row from the west. Its
    /// length is a multiple of `width`.
    pub terrain: Vec<Terrain>,
    pub entities: Vec<PerceivedEntity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientMessage {
    /// The first message of a connection: take the controller of the world
    /// this token names.
    Hello { token: ResumeToken },
    /// `entity` takes `action`.
    Command { entity: EntityId, action: Action },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerMessage {
    Snapshot(Snapshot),
    /// The world waits for the controller of `entity` at `time` (R2). Sent
    /// only to that controller.
    Awaited {
        time: Time,
        entity: EntityId,
        actions: Vec<AvailableAction>,
    },
    Perceived(PerceivedChange),
    /// The last command was refused. Nothing advanced (R10).
    Refused,
}

/// The bytes are not one well-formed message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodeError;

impl ClientMessage {
    pub fn encode(&self) -> Vec<u8> {
        Vec::new()
    }

    /// Never panics, whatever the bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let _ = bytes;
        Err(DecodeError)
    }
}

impl ServerMessage {
    pub fn encode(&self) -> Vec<u8> {
        Vec::new()
    }

    /// Never panics, whatever the bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let _ = bytes;
        Err(DecodeError)
    }
}
