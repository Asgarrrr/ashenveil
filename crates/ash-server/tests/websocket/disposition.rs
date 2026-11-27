//! A disposition as the simulation names it, as the protocol names it, for
//! the tests that build what the server must send from the simulation alone.
//! It repeats the server's mapping on purpose, like `action.rs`.

use ash_protocol::message::Disposition as WireDisposition;
use ash_sim::controller::Disposition as SimDisposition;

pub fn to_wire(disposition: SimDisposition) -> WireDisposition {
    match disposition {
        SimDisposition::Hostile => WireDisposition::Hostile,
        SimDisposition::Neutral => WireDisposition::Neutral,
    }
}
