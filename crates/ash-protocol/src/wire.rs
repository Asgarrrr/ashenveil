//! The binary encoding: a tag byte for each choice, then fixed-width
//! little-endian fields. A list is a `u32` count, then its items.

use alloc::vec::Vec;

use ash_core::entity::EntityId;
use ash_core::tile::{Direction, TilePosition};
use ash_core::time::{ActionCost, Time};

use crate::message::DecodeError;

/// A value with a binary encoding.
pub trait Wire: Sized {
    fn write(&self, out: &mut Vec<u8>);

    /// Reads one value from the front of `input`. Never panics.
    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError>;
}

/// The bytes of a message not read yet.
pub struct Reader<'a> {
    rest: &'a [u8],
}

impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let (head, rest) = self.rest.split_first_chunk::<N>().ok_or(DecodeError)?;
        self.rest = rest;
        Ok(*head)
    }
}

pub fn encode<T: Wire>(value: &T) -> Vec<u8> {
    let mut out = Vec::new();
    value.write(&mut out);
    out
}

/// One value that spans `bytes` exactly: trailing bytes are an error.
pub fn decode<T: Wire>(bytes: &[u8]) -> Result<T, DecodeError> {
    let mut input = Reader { rest: bytes };
    let value = T::read(&mut input)?;
    if input.rest.is_empty() {
        Ok(value)
    } else {
        Err(DecodeError)
    }
}

macro_rules! little_endian {
    ($($integer:ty),+) => {$(
        impl Wire for $integer {
            fn write(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.to_le_bytes());
            }

            fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
                input.take().map(Self::from_le_bytes)
            }
        }
    )+};
}

little_endian!(u8, u32, u64, u128);

impl<T: Wire> Wire for Vec<T> {
    fn write(&self, out: &mut Vec<u8>) {
        let Ok(count) = u32::try_from(self.len()) else {
            ash_core::invariant!("a list of {} items does not fit a message", self.len())
        };
        count.write(out);
        for item in self {
            item.write(out);
        }
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        let count = u32::read(input)?;
        // No capacity from the untrusted count: each item read consumes
        // bytes, so a false count fails at the end of the input.
        let mut items = Vec::new();
        for _ in 0..count {
            items.push(T::read(input)?);
        }
        Ok(items)
    }
}

impl Wire for EntityId {
    fn write(&self, out: &mut Vec<u8>) {
        self.get().write(out);
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        u32::read(input).map(Self::new)
    }
}

impl Wire for Time {
    fn write(&self, out: &mut Vec<u8>) {
        self.units().write(out);
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        u64::read(input).map(Self::new)
    }
}

impl Wire for ActionCost {
    fn write(&self, out: &mut Vec<u8>) {
        self.units().write(out);
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        u64::read(input).map(Self::new)
    }
}

impl Wire for TilePosition {
    fn write(&self, out: &mut Vec<u8>) {
        self.x.write(out);
        self.y.write(out);
        self.z.write(out);
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            x: u32::read(input)?,
            y: u32::read(input)?,
            z: u32::read(input)?,
        })
    }
}

impl Wire for Direction {
    fn write(&self, out: &mut Vec<u8>) {
        let tag: u8 = match self {
            Self::North => 0,
            Self::NorthEast => 1,
            Self::East => 2,
            Self::SouthEast => 3,
            Self::South => 4,
            Self::SouthWest => 5,
            Self::West => 6,
            Self::NorthWest => 7,
        };
        tag.write(out);
    }

    fn read(input: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match u8::read(input)? {
            0 => Ok(Self::North),
            1 => Ok(Self::NorthEast),
            2 => Ok(Self::East),
            3 => Ok(Self::SouthEast),
            4 => Ok(Self::South),
            5 => Ok(Self::SouthWest),
            6 => Ok(Self::West),
            7 => Ok(Self::NorthWest),
            _ => Err(DecodeError),
        }
    }
}
