//! The random wander of the skeleton creature (R14).

use ash_core::rng::Rng;

use crate::action::{Action, AvailableAction};

/// One of `offered`, drawn from `rng`, each about equally likely: the
/// modulo bias of a few choices out of 2^64 draws is negligible. Returns the
/// RNG after the draw and leaves `rng` as it was (F4). `offered` holds only
/// actions the rules allow, so the wander is never refused (R14). An empty
/// offer crashes through `invariant!`: waiting is always possible.
pub fn choose(rng: &Rng, offered: &[AvailableAction]) -> (Action, Rng) {
    let mut next = rng.clone();
    let draw = next.next_u64();
    let chosen = u64::try_from(offered.len())
        .ok()
        .and_then(|count| draw.checked_rem(count))
        .and_then(|index| usize::try_from(index).ok())
        .and_then(|index| offered.get(index));
    match chosen {
        Some(offer) => (offer.action, next),
        None => ash_core::invariant!("the wander has no action to choose"),
    }
}
