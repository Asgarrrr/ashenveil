//! A perceived change as the simulation names it, as the protocol names it,
//! for the tests that build what the server must send from the simulation
//! alone. It repeats the server's mapping on purpose, like `action.rs`.

use ash_protocol::message::{Outcome as WireOutcome, PerceivedChange as WireChange};
use ash_sim::perception::{Outcome as SimOutcome, PerceivedChange as SimChange};

/// S3 signatures: `None` for a death, which the wire carries in a later
/// commit.
pub fn to_wire(change: SimChange) -> Option<WireChange> {
    match change {
        SimChange::Act {
            time,
            entity,
            outcome,
        } => Some(WireChange {
            time,
            entity,
            outcome: outcome_to_wire(outcome),
        }),
        SimChange::Death { .. } => None,
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
