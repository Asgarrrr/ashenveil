//! The random wander of the skeleton creature (R14).

use ash_core::rng::Rng;

use crate::action::{Action, Offer};

/// The action of one of `offers`, drawn from `rng`, each about equally
/// likely: the modulo bias of a few choices out of 2^64 draws is negligible.
/// Returns the RNG state after the draw and leaves `rng` as it was (F4).
/// Attacks are set aside before the draw: only the character attacks (A13).
/// `offers` holds only actions the rules allow, so the wander is never
/// refused (R14). Empty `offers` crash through `invariant!`: waiting is
/// always possible.
pub fn choose(rng: &Rng, offers: &[Offer]) -> (Action, Rng) {
    let candidates = || {
        offers
            .iter()
            .map(|offer| offer.action)
            .filter(|action| !matches!(action, Action::Attack(_)))
    };
    let mut after = rng.clone();
    let draw = after.next_u64();
    let chosen = u64::try_from(candidates().count())
        .ok()
        .and_then(|count| draw.checked_rem(count))
        .and_then(|index| usize::try_from(index).ok())
        .and_then(|index| candidates().nth(index));
    match chosen {
        Some(action) => (action, after),
        None => ash_core::invariant!("the wander has no action to choose"),
    }
}
