//! An action as the protocol names it and as the simulation names it. Both
//! directions sit together, so a new action changes one place.

use ash_protocol::message::Action as WireAction;
use ash_sim::action::Action as SimAction;

pub fn to_wire(action: SimAction) -> WireAction {
    match action {
        SimAction::Move(direction) => WireAction::Move(direction),
        SimAction::Wait => WireAction::Wait,
    }
}

pub fn to_sim(action: WireAction) -> SimAction {
    match action {
        WireAction::Move(direction) => SimAction::Move(direction),
        WireAction::Wait => SimAction::Wait,
    }
}
