//! Maps each resume token to the one world it controls, and records the
//! connection that controls its character.

use std::collections::HashMap;
use std::io;
use std::net::{Shutdown, TcpStream};
use std::sync::{Arc, Mutex, PoisonError};

use ash_core::entity::{EntityId, Speed};
use ash_protocol::message::ResumeToken;
use ash_sim::world::World;

use crate::quarantine::{self, Quarantined};

/// Phase 1 worlds draw nothing from their RNG, so one seed serves them all.
const WORLD_SEED: u64 = 0;

/// The world a new resume token starts. A replay of a world's accepted
/// commands starts from it too.
pub fn create_world() -> World {
    World::new(WORLD_SEED, Speed::new(1))
}

/// One player connection. The registry keeps a clone of its stream, so a
/// newer connection can interrupt its blocking read (R20).
#[derive(Clone)]
pub struct Controller(Arc<TcpStream>);

impl Controller {
    pub fn new(stream: &TcpStream) -> io::Result<Self> {
        stream.try_clone().map(|clone| Self(Arc::new(clone)))
    }

    /// Ends the connection both ways, even while the registry still holds a
    /// clone of its stream.
    pub fn close(&self) {
        let _ = self.0.shutdown(Shutdown::Both);
    }
}

/// A world and the connection that controls its character.
pub struct Controlled {
    pub world: World,
    controller: Option<Controller>,
}

impl Controlled {
    /// `controller` takes the controller of the character (R20). The read
    /// of the older connection ends; it can still write its farewell.
    pub fn take(&mut self, controller: &Controller) {
        if let Some(older) = self.controller.replace(controller.clone()) {
            let _ = older.0.shutdown(Shutdown::Read);
        }
    }

    pub fn is_controlled_by(&self, controller: &Controller) -> bool {
        self.controller
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(&current.0, &controller.0))
    }
}

/// One world and the character its controller drives.
pub struct HostedWorld {
    /// Its own lock: stepping one world never waits on another.
    controlled: Mutex<Controlled>,
    pub character: EntityId,
    /// The world violates an invariant once it accepts a command. Only
    /// `Registry::with_fault` sets it; no message can.
    pub faulty: bool,
}

impl HostedWorld {
    /// Runs `step` under the world's lock. A panic inside `step` puts this
    /// world, and no other, in quarantine.
    pub fn run<R>(&self, step: impl FnOnce(&mut Controlled) -> R) -> Result<R, Quarantined> {
        quarantine::run(&self.controlled, step)
    }
}

#[derive(Default)]
pub struct Registry {
    worlds: Mutex<HashMap<ResumeToken, Arc<HostedWorld>>>,
    fault: Option<ResumeToken>,
}

impl Registry {
    /// A registry where the world of `token` is faulty.
    pub fn with_fault(token: ResumeToken) -> Self {
        Self {
            worlds: Mutex::default(),
            fault: Some(token),
        }
    }

    /// The world `token` controls, created on first use. The registry lock
    /// is released on return, so no world is stepped while it is held.
    pub fn world(&self, token: ResumeToken) -> Arc<HostedWorld> {
        // Built before the lock, so no simulation code runs while it is
        // held; dropped unused when the token already has a world.
        let world = create_world();
        // R8: the character is created first, so it is the first awaited.
        let candidate = HostedWorld {
            character: world.awaited(),
            controlled: Mutex::new(Controlled {
                world,
                controller: None,
            }),
            faulty: self.fault == Some(token),
        };
        // Nothing panics between the reads and writes of the map, so a
        // poisoned lock still guards a consistent map.
        let mut worlds = self.worlds.lock().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(worlds.entry(token).or_insert_with(|| Arc::new(candidate)))
    }
}

#[cfg(test)]
mod tests {
    use ash_sim::action::Action;

    use super::*;

    #[test]
    fn a_quarantined_world_stays_registered_with_its_command_log() {
        let registry = Registry::default();
        let token = ResumeToken(7);
        let faulted = registry.world(token).run(|controlled| {
            assert_eq!(controlled.world.submit(Action::Wait), Ok(()));
            ash_core::invariant!("a test fault");
        });
        assert!(faulted.is_err());

        let hosted = registry.world(token);
        assert!(hosted.run(|_| ()).is_err(), "the quarantined world steps");
        let Err(poisoned) = hosted.controlled.lock() else {
            panic!("the quarantined world was replaced");
        };
        assert_eq!(poisoned.into_inner().world.command_log(), [Action::Wait]);
    }
}
