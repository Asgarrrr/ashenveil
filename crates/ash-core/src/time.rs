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

    pub const fn units(self) -> u64 {
        self.0
    }

    /// The time `cost` after `self`. An overflow crashes through `invariant!`.
    pub fn after(self, cost: ActionCost) -> Self {
        match self.0.checked_add(cost.0) {
            Some(units) => Self(units),
            None => crate::invariant!("game time overflows: {self:?} after {cost:?}"),
        }
    }
}

/// The number of time units an action takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionCost(u64);

impl ActionCost {
    pub const fn new(units: u64) -> Self {
        Self(units)
    }

    pub const fn units(self) -> u64 {
        self.0
    }

    /// R6: the cost of an action at `speed`, when `self` is its base cost.
    /// The design does not say how to round, so an inexact division crashes
    /// through `invariant!` instead of picking a rounding.
    pub fn divided_by(self, speed: Speed) -> Self {
        let divisor = speed.get();
        match (self.0.checked_div(divisor), self.0.checked_rem(divisor)) {
            (Some(units), Some(0)) => Self(units),
            _ => crate::invariant!("{self:?} is not divisible by {speed:?}"),
        }
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
