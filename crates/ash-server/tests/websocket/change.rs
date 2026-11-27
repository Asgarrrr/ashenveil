//! A perceived change as the simulation names it, as the protocol names it,
//! for the tests that build what the server must send from the simulation
//! alone. It repeats the server's mapping on purpose, like `action.rs`.

use ash_protocol::message::{
    Condition as WireCondition, Outcome as WireOutcome, PerceivedChange as WireChange,
};
use ash_sim::combat::Condition as SimCondition;
use ash_sim::perception::{Outcome as SimOutcome, PerceivedChange as SimChange};

pub fn to_wire(change: SimChange) -> WireChange {
    match change {
        SimChange::Act {
            time,
            entity,
            outcome,
        } => WireChange::Act {
            time,
            entity,
            outcome: outcome_to_wire(outcome),
        },
        SimChange::Death { time, entity } => WireChange::Death { time, entity },
        SimChange::Condition {
            time,
            entity,
            condition,
        } => WireChange::Condition {
            time,
            entity,
            condition: condition_to_wire(condition),
        },
    }
}

fn condition_to_wire(condition: SimCondition) -> WireCondition {
    match condition {
        SimCondition::Unhurt => WireCondition::Unhurt,
        SimCondition::Wounded => WireCondition::Wounded,
        SimCondition::BadlyWounded => WireCondition::BadlyWounded,
        SimCondition::Dying => WireCondition::Dying,
    }
}

fn outcome_to_wire(outcome: SimOutcome) -> WireOutcome {
    match outcome {
        SimOutcome::Stepped(direction) => WireOutcome::Stepped(direction),
        SimOutcome::Waited => WireOutcome::Waited,
        SimOutcome::Bumped(target) => WireOutcome::Bumped(target),
        SimOutcome::Hit(target) => WireOutcome::Hit(target),
        SimOutcome::Missed(target) => WireOutcome::Missed(target),
    }
}
