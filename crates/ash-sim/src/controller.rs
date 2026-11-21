//! What drives an entity: a player connection or the simulation. Every
//! controller chooses under the same rules (R3).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Controller {
    /// A player connection. Time stops while the world awaits its command
    /// (R2).
    Player,
    /// The simulation: a random wander among the actions the rules allow
    /// (R14). It chooses at once, so time never waits for it.
    Wander,
}
