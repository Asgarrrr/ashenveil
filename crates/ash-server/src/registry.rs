//! Maps each resume token to the one world it controls.

use ash_core::entity::Speed;
use ash_sim::world::World;

/// Phase 1 worlds draw nothing from their RNG, so one seed serves them all.
const WORLD_SEED: u64 = 0;

/// The world a new resume token starts. A replay of a world's accepted
/// commands starts from it too.
pub fn create_world() -> World {
    World::new(WORLD_SEED, Speed::new(1))
}
