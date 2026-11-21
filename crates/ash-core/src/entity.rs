//! Entity identity and speed.

use core::num::NonZeroU64;

/// Names one entity of a world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntityId(u32);

impl EntityId {
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

/// R6: the divisor of an entity's base costs. Never zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Speed(NonZeroU64);

impl Speed {
    /// A zero speed crashes through `invariant!`.
    pub fn new(value: u64) -> Self {
        match NonZeroU64::new(value) {
            Some(value) => Self(value),
            None => crate::invariant!("an entity speed is zero"),
        }
    }

    pub(crate) const fn get(self) -> u64 {
        self.0.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "invariant violated")]
    fn zero_speed_crashes() {
        let _ = Speed::new(0);
    }
}
