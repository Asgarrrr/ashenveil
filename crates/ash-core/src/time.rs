//! Game time, counted in time units (R1), and action costs.

use crate::entity::Speed;

/// A point in game time: the number of time units since the world began.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time(u64);

impl Time {
    pub const ZERO: Self = Self(0);

    pub const fn new(units: u64) -> Self {
        Self(units)
    }

    /// The time `cost` after `self`. An overflow crashes through `invariant!`.
    pub fn after(self, _cost: ActionCost) -> Self {
        self
    }
}

/// The number of time units an action takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionCost(u64);

impl ActionCost {
    pub const fn new(units: u64) -> Self {
        Self(units)
    }

    /// R6: the cost of an action at `speed`, when `self` is its base cost.
    pub fn divided_by(self, _speed: Speed) -> Self {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "invariant violated")]
    fn time_overflow_crashes() {
        let _ = Time::new(u64::MAX).after(ActionCost::new(1));
    }

    #[test]
    #[should_panic(expected = "invariant violated")]
    fn inexact_division_by_speed_crashes() {
        let _ = ActionCost::new(50).divided_by(Speed::new(3));
    }
}
