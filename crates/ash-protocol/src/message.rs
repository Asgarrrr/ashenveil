//! What the server and a client say to each other.
//!
//! Nothing here can carry another entity's speed, cost or next turn (R18):
//! only the awaited entity's own costs, sent to its controller (R9). A
//! disposition and a condition are kinds, never numbers. Hit points go to
//! their own character alone, in its snapshot (A9).

use alloc::vec::Vec;

use ash_core::time::{ActionCost, Time};

// A client builds commands from these, and may depend on this crate alone.
pub use ash_core::entity::EntityId;
pub use ash_core::tile::{Direction, Terrain, TilePosition};

use crate::wire::{self, Reader, Wire};

/// Identifies one controller of one world across connections. The client
/// chooses it; an unknown token starts a new world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ResumeToken(pub u128);

/// An action an entity can take (R4, A3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Move(Direction),
    Wait,
    Attack(Direction),
}

/// An action offered to the awaited entity, with the cost it would pay (R9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AvailableAction {
    pub action: Action,
    pub cost: ActionCost,
}

/// How a creature stands toward the character (A4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    /// A move into the creature is an attack on it (A1).
    Hostile,
    /// A move into the creature is a bump (A2).
    Neutral,
}

/// What the character perceives of another entity's hit points (A10): a
/// kind, never a number (R18).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    /// All of its hit points.
    Unhurt,
    /// Fewer than all of them.
    Wounded,
    /// Fewer than half of them.
    BadlyWounded,
    /// Fewer than a quarter of them.
    Dying,
}

/// What the character perceives of the hit points of an entity: its own as
/// numbers (A9), those of any other entity as a condition (A10, R18).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PerceivedHealth {
    /// The hit points of the character itself.
    HitPoints { current: u32, max: u32 },
    /// The condition of another entity.
    Condition(Condition),
}

/// An entity the character perceives: where it stands, its hit points or its
/// condition and, for a creature, its disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerceivedEntity {
    pub entity: EntityId,
    pub position: TilePosition,
    /// `None` for a character: it has no disposition.
    pub disposition: Option<Disposition>,
    pub health: PerceivedHealth,
}

/// One change, stamped with the time it happened (R17, A11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PerceivedChange {
    /// What `entity` did.
    Act {
        time: Time,
        entity: EntityId,
        outcome: Outcome,
    },
    /// `entity` died and left the world (A8). Its lethal hit is the change
    /// before, at the same time.
    Death { time: Time, entity: EntityId },
    /// A hit left `entity` in another `condition` (A10). That hit is the
    /// change before, at the same time.
    Condition {
        time: Time,
        entity: EntityId,
        condition: Condition,
    },
}

/// What an action did (A11), naming its target where it has one. Never the
/// odds of a hit (A12).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// A move reached the neighboring tile in this direction.
    Stepped(Direction),
    Waited,
    /// A move ran into this entity (R13): the mover stayed in place.
    Bumped(EntityId),
    /// An attack struck this entity (A6).
    Hit(EntityId),
    /// An attack missed this entity (A6).
    Missed(EntityId),
}

/// Everything one character perceives at `time` (R16).
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
    /// this token names. `received` counts the perceived changes of that
    /// world the client has received, so the server sends it the others
    /// (R17).
    Hello { token: ResumeToken, received: u64 },
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
            Self::Attack(direction) => {
                2_u8.write(out);
                direction.write(out);
            }
        }
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match u8::read(input)? {
            0 => Ok(Self::Wait),
            1 => Direction::read(input).map(Self::Move),
            2 => Direction::read(input).map(Self::Attack),
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

/// One tag: 0 for a character, which has no disposition, then one for each
/// disposition of a creature.
impl Wire for Option<Disposition> {
    fn write(&self, out: &mut Vec<u8>) {
        let tag: u8 = match self {
            None => 0,
            Some(Disposition::Hostile) => 1,
            Some(Disposition::Neutral) => 2,
        };
        tag.write(out);
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match u8::read(input)? {
            0 => Ok(None),
            1 => Ok(Some(Disposition::Hostile)),
            2 => Ok(Some(Disposition::Neutral)),
            _ => Err(DecodeError),
        }
    }
}

impl Wire for Condition {
    fn write(&self, out: &mut Vec<u8>) {
        let tag: u8 = match self {
            Self::Unhurt => 0,
            Self::Wounded => 1,
            Self::BadlyWounded => 2,
            Self::Dying => 3,
        };
        tag.write(out);
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match u8::read(input)? {
            0 => Ok(Self::Unhurt),
            1 => Ok(Self::Wounded),
            2 => Ok(Self::BadlyWounded),
            3 => Ok(Self::Dying),
            _ => Err(DecodeError),
        }
    }
}

impl Wire for PerceivedHealth {
    fn write(&self, out: &mut Vec<u8>) {
        match self {
            Self::HitPoints { current, max } => {
                0_u8.write(out);
                current.write(out);
                max.write(out);
            }
            Self::Condition(condition) => {
                1_u8.write(out);
                condition.write(out);
            }
        }
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match u8::read(input)? {
            0 => Ok(Self::HitPoints {
                current: u32::read(input)?,
                max: u32::read(input)?,
            }),
            1 => Condition::read(input).map(Self::Condition),
            _ => Err(DecodeError),
        }
    }
}

impl Wire for PerceivedEntity {
    fn write(&self, out: &mut Vec<u8>) {
        self.entity.write(out);
        self.position.write(out);
        self.disposition.write(out);
        self.health.write(out);
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            entity: EntityId::read(input)?,
            position: TilePosition::read(input)?,
            disposition: Option::read(input)?,
            health: PerceivedHealth::read(input)?,
        })
    }
}

impl Wire for Outcome {
    fn write(&self, out: &mut Vec<u8>) {
        match self {
            Self::Stepped(direction) => {
                0_u8.write(out);
                direction.write(out);
            }
            Self::Waited => 1_u8.write(out),
            Self::Bumped(target) => {
                2_u8.write(out);
                target.write(out);
            }
            Self::Hit(target) => {
                3_u8.write(out);
                target.write(out);
            }
            Self::Missed(target) => {
                4_u8.write(out);
                target.write(out);
            }
        }
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match u8::read(input)? {
            0 => Direction::read(input).map(Self::Stepped),
            1 => Ok(Self::Waited),
            2 => EntityId::read(input).map(Self::Bumped),
            3 => EntityId::read(input).map(Self::Hit),
            4 => EntityId::read(input).map(Self::Missed),
            _ => Err(DecodeError),
        }
    }
}

impl Wire for PerceivedChange {
    fn write(&self, out: &mut Vec<u8>) {
        match self {
            Self::Act {
                time,
                entity,
                outcome,
            } => {
                0_u8.write(out);
                time.write(out);
                entity.write(out);
                outcome.write(out);
            }
            Self::Death { time, entity } => {
                1_u8.write(out);
                time.write(out);
                entity.write(out);
            }
            Self::Condition {
                time,
                entity,
                condition,
            } => {
                2_u8.write(out);
                time.write(out);
                entity.write(out);
                condition.write(out);
            }
        }
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match u8::read(input)? {
            0 => {
                let time = Time::read(input)?;
                let entity = EntityId::read(input)?;
                // No entity bumps or strikes itself (R13, A3).
                match Outcome::read(input)? {
                    Outcome::Bumped(target) | Outcome::Hit(target) | Outcome::Missed(target)
                        if target == entity =>
                    {
                        Err(DecodeError)
                    }
                    outcome => Ok(Self::Act {
                        time,
                        entity,
                        outcome,
                    }),
                }
            }
            1 => Ok(Self::Death {
                time: Time::read(input)?,
                entity: EntityId::read(input)?,
            }),
            2 => Ok(Self::Condition {
                time: Time::read(input)?,
                entity: EntityId::read(input)?,
                condition: Condition::read(input)?,
            }),
            _ => Err(DecodeError),
        }
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
            Self::Hello { token, received } => {
                0_u8.write(out);
                token.write(out);
                received.write(out);
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
                received: u64::read(input)?,
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
            4 => Ok(Self::Quarantined),
            5 => Ok(Self::ControllerTaken),
            _ => Err(DecodeError),
        }
    }
}
