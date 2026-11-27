//! An outcome as the simulation names it, as the protocol names it, for the
//! tests that build what the server must send from the simulation alone. It
//! repeats the server's mapping on purpose, like `action.rs`.

use ash_protocol::message::Outcome as WireOutcome;
use ash_sim::perception::Outcome as SimOutcome;

pub fn to_wire(outcome: SimOutcome) -> WireOutcome {
    match outcome {
        SimOutcome::Stepped(direction) => WireOutcome::Stepped(direction),
        SimOutcome::Waited => WireOutcome::Waited,
        SimOutcome::Bumped(target) => WireOutcome::Bumped(target),
        SimOutcome::Hit(target) => WireOutcome::Hit(target),
        SimOutcome::Missed(target) => WireOutcome::Missed(target),
    }
}
