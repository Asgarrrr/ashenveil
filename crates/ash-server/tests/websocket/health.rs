//! The hit points or the condition of a perceived entity as the simulation
//! names them, as the protocol names them, for the tests that build what the
//! server must send from the simulation alone. It repeats the server's
//! mapping on purpose, like `action.rs`. A test that mounts it mounts
//! `change.rs` too: the mapping of a condition lives there.

use ash_protocol::message::PerceivedHealth as WireHealth;
use ash_sim::perception::PerceivedHealth as SimHealth;

use crate::change;

pub fn to_wire(health: SimHealth) -> WireHealth {
    match health {
        SimHealth::HitPoints { current, max } => WireHealth::HitPoints { current, max },
        SimHealth::Condition(condition) => {
            WireHealth::Condition(change::condition_to_wire(condition))
        }
    }
}
