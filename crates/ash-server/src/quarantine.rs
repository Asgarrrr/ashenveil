//! A world whose simulation hit a violated invariant stops, alone.
//!
//! `invariant!` panics. The panic unwinds through the world's lock and
//! poisons it; a poisoned lock is the quarantine. The release profile keeps
//! `panic = "unwind"`, so this holds in production too.

use std::panic::{self, AssertUnwindSafe};
use std::sync::Mutex;

/// The world is in quarantine: it is never stepped again.
pub struct Quarantined;

/// Runs `step` on what `lock` guards. A panic inside `step` puts the value
/// in quarantine: this call and every later one return `Quarantined`
/// without running their step. The value itself stays in `lock`.
pub fn run<T, R>(lock: &Mutex<T>, step: impl FnOnce(&mut T) -> R) -> Result<R, Quarantined> {
    // The guard drops while the panic unwinds, which poisons the lock: no
    // later step can observe the broken state, so asserting unwind safety
    // is sound.
    panic::catch_unwind(AssertUnwindSafe(|| {
        let mut guarded = lock.lock().map_err(|_| Quarantined)?;
        Ok(step(&mut guarded))
    }))
    .unwrap_or(Err(Quarantined))
}
