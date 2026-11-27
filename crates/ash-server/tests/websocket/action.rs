//! An action as the protocol names it and as the simulation names it, for
//! the tests that build what the server must send from the simulation alone.
//! It repeats the server's mapping on purpose: an oracle that called the
//! server's code would not catch a bug in it.

use ash_protocol::message::Action as WireAction;
use ash_sim::action::Action as SimAction;

pub fn to_wire(action: SimAction) -> WireAction {
    match action {
        SimAction::Move(direction) => WireAction::Move(direction),
        SimAction::Wait => WireAction::Wait,
        SimAction::Attack(direction) => WireAction::Attack(direction),
    }
}

pub fn to_sim(action: WireAction) -> SimAction {
    match action {
        WireAction::Move(direction) => SimAction::Move(direction),
        WireAction::Wait => SimAction::Wait,
        WireAction::Attack(direction) => SimAction::Attack(direction),
    }
}
