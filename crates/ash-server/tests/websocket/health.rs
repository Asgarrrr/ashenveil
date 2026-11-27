//! The hit points or the condition of a perceived entity as the simulation
//! names them, as the protocol names them, for the tests that build what the
//! server must send from the simulation alone. It repeats the server's
//! mapping on purpose, like `action.rs`.

use ash_protocol::message::{Condition as WireCondition, PerceivedHealth as WireHealth};
use ash_sim::combat::Condition as SimCondition;
use ash_sim::perception::PerceivedHealth as SimHealth;

pub fn to_wire(health: SimHealth) -> WireHealth {
    match health {
        SimHealth::HitPoints { current, max } => WireHealth::HitPoints { current, max },
        SimHealth::Condition(condition) => WireHealth::Condition(match condition {
            SimCondition::Unhurt => WireCondition::Unhurt,
            SimCondition::Wounded => WireCondition::Wounded,
            SimCondition::BadlyWounded => WireCondition::BadlyWounded,
            SimCondition::Dying => WireCondition::Dying,
        }),
    }
}
