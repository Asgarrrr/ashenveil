//! A perceived change as the simulation names it, as the protocol names it,
//! for the tests that build what the server must send from the simulation
//! alone. It repeats the server's mapping on purpose, like `action.rs`.

use ash_protocol::message::{
    ChangeKind as WireKind, Condition as WireCondition, Disposition as WireDisposition,
    Outcome as WireOutcome, PerceivedChange as WireChange,
};
use ash_sim::combat::Condition as SimCondition;
use ash_sim::controller::Disposition as SimDisposition;
use ash_sim::perception::{
    ChangeKind as SimKind, Outcome as SimOutcome, PerceivedChange as SimChange,
};

pub fn to_wire(change: SimChange) -> WireChange {
    WireChange {
        time: change.time,
        entity: change.entity,
        kind: match change.kind {
            SimKind::Act(outcome) => WireKind::Act(outcome_to_wire(outcome)),
            SimKind::Death => WireKind::Death,
            SimKind::Condition(condition) => WireKind::Condition(condition_to_wire(condition)),
            SimKind::Disposition(disposition) => WireKind::Disposition(match disposition {
                SimDisposition::Hostile => WireDisposition::Hostile,
                SimDisposition::Neutral => WireDisposition::Neutral,
            }),
        },
    }
}

pub fn condition_to_wire(condition: SimCondition) -> WireCondition {
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
