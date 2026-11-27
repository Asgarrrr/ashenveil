//! What drives an entity: a player connection or the simulation. Every
//! controller chooses under the same rules (R3).

use crate::combat::Disposition;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Controller {
    /// A player connection: the entity is a character. Time stops while the
    /// world awaits its command (R2).
    Player,
    /// The simulation: the entity is a creature, with its disposition (A4).
    /// A random wander among the actions the rules allow (R14). It chooses
    /// at once, so time never waits for it.
    Wander(Disposition),
}
