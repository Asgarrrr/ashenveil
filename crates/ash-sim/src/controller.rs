//! What drives an entity: a player connection or the simulation. Every
//! controller chooses under the same rules (R3).

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

/// A4: how a creature stands toward characters. A kind, never a number
/// (R18).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    /// A character's move into the creature is an attack on it (A1).
    Hostile,
    /// A character's move into the creature is a bump (A2).
    Neutral,
}
