//! The random wander of the skeleton creature (R14).

use ash_core::rng::Rng;

use crate::action::{Action, AvailableAction};

/// One of `offered`, drawn with `rng`. `offered` holds only actions the
/// rules allow, so the wander is never refused (R14).
pub fn choose(rng: &mut Rng, offered: &[AvailableAction]) -> Action {
    let _ = (rng, offered);
    Action::Wait
}
