//! Maps each resume token to the one world it controls.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use ash_core::entity::{EntityId, Speed};
use ash_protocol::message::ResumeToken;
use ash_sim::world::World;

/// Phase 1 worlds draw nothing from their RNG, so one seed serves them all.
const WORLD_SEED: u64 = 0;

/// The world a new resume token starts. A replay of a world's accepted
/// commands starts from it too.
pub fn create_world() -> World {
    World::new(WORLD_SEED, Speed::new(1))
}

/// One world and the character its controller drives.
pub struct HostedWorld {
    /// Its own lock: stepping one world never waits on another.
    pub world: Mutex<World>,
    pub character: EntityId,
}

#[derive(Default)]
pub struct Registry {
    worlds: Mutex<HashMap<ResumeToken, Arc<HostedWorld>>>,
}

impl Registry {
    /// The world `token` controls, created on first use. The registry lock
    /// is released on return, so no world is stepped while it is held.
    pub fn world(&self, token: ResumeToken) -> Arc<HostedWorld> {
        // Nothing panics between the reads and writes of the map, so a
        // poisoned lock still guards a consistent map.
        let mut worlds = self.worlds.lock().unwrap_or_else(PoisonError::into_inner);
        let hosted = worlds.entry(token).or_insert_with(|| {
            let world = create_world();
            // R8: the character is created first, so it is the first awaited.
            Arc::new(HostedWorld {
                character: world.awaited(),
                world: Mutex::new(world),
            })
        });
        Arc::clone(hosted)
    }
}
