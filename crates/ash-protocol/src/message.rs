//! What the server and a client say to each other.
//!
//! Nothing here can carry another entity's speed, cost or next turn (R18):
//! only the awaited entity's own costs, sent to its controller (R9).

use alloc::vec::Vec;

use ash_core::entity::EntityId;
use ash_core::tile::{Direction, TilePosition};
use ash_core::time::{ActionCost, Time};

use crate::wire::{self, Reader, Wire};

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
    /// The world hit a violated invariant: it is in quarantine and never
    /// steps again. The server closes the connection after this message,
    /// and answers every later `Hello` for the world with it.
    Quarantined,
    /// A newer connection took the controller of this character (R20). The
    /// server closes this connection after this message.
    ControllerTaken,
}

/// The bytes are not one well-formed message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodeError;

impl ClientMessage {
    pub fn encode(&self) -> Vec<u8> {
        wire::encode(self)
    }

    /// Never panics, whatever the bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        wire::decode(bytes)
    }
}

impl ServerMessage {
    pub fn encode(&self) -> Vec<u8> {
        wire::encode(self)
    }

    /// Never panics, whatever the bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        wire::decode(bytes)
    }
}

impl Wire for ResumeToken {
    fn write(&self, out: &mut Vec<u8>) {
        self.0.write(out);
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        u128::read(input).map(Self)
    }
}

impl Wire for Terrain {
    fn write(&self, out: &mut Vec<u8>) {
        let tag: u8 = match self {
            Self::Floor => 0,
            Self::Wall => 1,
        };
        tag.write(out);
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match u8::read(input)? {
            0 => Ok(Self::Floor),
            1 => Ok(Self::Wall),
            _ => Err(DecodeError),
        }
    }
}

impl Wire for Action {
    fn write(&self, out: &mut Vec<u8>) {
        match self {
            Self::Wait => 0_u8.write(out),
            Self::Move(direction) => {
                1_u8.write(out);
                direction.write(out);
            }
        }
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match u8::read(input)? {
            0 => Ok(Self::Wait),
            1 => Direction::read(input).map(Self::Move),
            _ => Err(DecodeError),
        }
    }
}

impl Wire for AvailableAction {
    fn write(&self, out: &mut Vec<u8>) {
        self.action.write(out);
        self.cost.write(out);
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            action: Action::read(input)?,
            cost: ActionCost::read(input)?,
        })
    }
}

impl Wire for PerceivedEntity {
    fn write(&self, out: &mut Vec<u8>) {
        self.entity.write(out);
        self.position.write(out);
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            entity: EntityId::read(input)?,
            position: TilePosition::read(input)?,
        })
    }
}

impl Wire for PerceivedChange {
    fn write(&self, out: &mut Vec<u8>) {
        self.time.write(out);
        self.entity.write(out);
        self.action.write(out);
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            time: Time::read(input)?,
            entity: EntityId::read(input)?,
            action: Action::read(input)?,
        })
    }
}

impl Wire for Snapshot {
    fn write(&self, out: &mut Vec<u8>) {
        self.time.write(out);
        self.width.write(out);
        self.terrain.write(out);
        self.entities.write(out);
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        let snapshot = Self {
            time: Time::read(input)?,
            width: u32::read(input)?,
            terrain: Vec::read(input)?,
            entities: Vec::read(input)?,
        };
        let width = usize::try_from(snapshot.width).map_err(|_| DecodeError)?;
        // A zero width has no remainder: it is refused with the ragged rows.
        match snapshot.terrain.len().checked_rem(width) {
            Some(0) => Ok(snapshot),
            _ => Err(DecodeError),
        }
    }
}

impl Wire for ClientMessage {
    fn write(&self, out: &mut Vec<u8>) {
        match self {
            Self::Hello { token } => {
                0_u8.write(out);
                token.write(out);
            }
            Self::Command { entity, action } => {
                1_u8.write(out);
                entity.write(out);
                action.write(out);
            }
        }
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match u8::read(input)? {
            0 => Ok(Self::Hello {
                token: ResumeToken::read(input)?,
            }),
            1 => Ok(Self::Command {
                entity: EntityId::read(input)?,
                action: Action::read(input)?,
            }),
            _ => Err(DecodeError),
        }
    }
}

impl Wire for ServerMessage {
    fn write(&self, out: &mut Vec<u8>) {
        match self {
            Self::Snapshot(snapshot) => {
                0_u8.write(out);
                snapshot.write(out);
            }
            Self::Awaited {
                time,
                entity,
                actions,
            } => {
                1_u8.write(out);
                time.write(out);
                entity.write(out);
                actions.write(out);
            }
            Self::Perceived(change) => {
                2_u8.write(out);
                change.write(out);
            }
            Self::Refused => 3_u8.write(out),
            Self::Quarantined => 4_u8.write(out),
            Self::ControllerTaken => 5_u8.write(out),
        }
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match u8::read(input)? {
            0 => Snapshot::read(input).map(Self::Snapshot),
            1 => Ok(Self::Awaited {
                time: Time::read(input)?,
                entity: EntityId::read(input)?,
                actions: Vec::read(input)?,
            }),
            2 => PerceivedChange::read(input).map(Self::Perceived),
            3 => Ok(Self::Refused),
            _ => Err(DecodeError),
        }
    }
}
