//! Maps each resume token to the one world it controls, and records the
//! connection that controls its character.

use std::collections::HashMap;
use std::io;
use std::net::{Shutdown, TcpStream};
use std::sync::{Arc, Mutex, PoisonError};

use ash_core::entity::{EntityId, Speed};
use ash_core::tile::Direction;
use ash_protocol::message::ResumeToken;
use ash_sim::action::Action;
use ash_sim::controller::Controller;
use ash_sim::world::World;

use crate::quarantine::{self, Quarantined};

/// The seed of the world `token` starts. The registry keeps every world
/// under its token, so it keeps the seed too: with the world's command log,
/// it replays the world.
pub fn world_seed(token: ResumeToken) -> u64 {
    let bytes = token.0.to_le_bytes();
    let half = |start: usize| u64::from_le_bytes(std::array::from_fn(|byte| bytes[start + byte]));
    half(0) ^ half(8)
}

/// One player connection. The registry keeps a clone of its stream, so a
/// newer connection can interrupt its blocking read (R20).
#[derive(Clone)]
pub struct PlayerConnection(Arc<TcpStream>);

impl PlayerConnection {
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
    controller: Option<PlayerConnection>,
}

impl Controlled {
    /// `controller` takes the controller of the character (R20). The read
    /// of the older connection ends; it can still write its farewell.
    pub fn take(&mut self, controller: &PlayerConnection) {
        if let Some(older) = self.controller.replace(controller.clone()) {
            let _ = older.0.shutdown(Shutdown::Read);
        }
    }

    pub fn is_controlled_by(&self, controller: &PlayerConnection) -> bool {
        self.controller
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(&current.0, &controller.0))
    }
}

/// One world and the first character its controller drove.
pub struct HostedWorld {
    /// Its own lock: stepping one world never waits on another.
    controlled: Mutex<Controlled>,
    /// Never changes: the world records the stranger of each dead
    /// character, so it tells the character the controller drives now (D6).
    pub character: EntityId,
    /// The world violates an invariant once it accepts a command. Only
    /// `Registry::with_fault` sets it; no message can.
    pub faulty: bool,
    /// The second character of the test entry, which the server drives
    /// (`strike`). Only `Registry::with_struck_character` sets it; no
    /// message can.
    pub striker: Option<EntityId>,
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
    struck: Option<ResumeToken>,
}

impl Registry {
    /// A registry where the world of `token` is faulty.
    pub fn with_fault(token: ResumeToken) -> Self {
        Self {
            fault: Some(token),
            ..Self::default()
        }
    }

    /// A registry where the world of `token` is the struck world of the
    /// test entry (`struck_world`).
    pub fn with_struck_character(token: ResumeToken) -> Self {
        Self {
            struck: Some(token),
            ..Self::default()
        }
    }

    /// The world `token` controls, created on first use. The registry lock
    /// is released on return, so no world is stepped while it is held.
    pub fn world(&self, token: ResumeToken) -> Arc<HostedWorld> {
        // Built before the lock, so no simulation code runs while it is
        // held; dropped unused when the token already has a world.
        let seed = world_seed(token);
        let (world, striker) = if self.struck == Some(token) {
            let (world, striker) = struck_world(seed);
            (world, Some(striker))
        } else {
            (World::skeleton(seed), None)
        };
        // R8: the character is created first, so it is the first awaited.
        let candidate = HostedWorld {
            character: world.awaited(),
            controlled: Mutex::new(Controlled {
                world,
                controller: None,
            }),
            faulty: self.fault == Some(token),
            striker,
        };
        // Nothing panics between the reads and writes of the map, so a
        // poisoned lock still guards a consistent map.
        let mut worlds = self.worlds.lock().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(worlds.entry(token).or_insert_with(|| Arc::new(candidate)))
    }
}

/// The world of the test entry: the character on the start tile, then a
/// second character east of it, both at speed 1, and no creature. Returns
/// the second, which no connection controls.
fn struck_world(seed: u64) -> (World, EntityId) {
    let mut world = World::new(seed, Speed::new(1));
    let east = world.position(world.awaited()).step(Direction::East);
    let second = world.create_entity(Speed::new(1), east, Controller::Player);
    (world, second)
}

/// The test entry: while `world` awaits `striker`, it attacks west, toward
/// the start tile of the first character, or waits when that attack is not
/// offered (R14).
pub fn strike(world: &mut World, striker: EntityId) {
    let attack = Action::Attack(Direction::West);
    while world.awaited() == striker {
        let offered = world.offers().iter().any(|offer| offer.action == attack);
        let action = if offered { attack } else { Action::Wait };
        if world.submit(action).is_err() {
            ash_core::invariant!("the world refused the offered {action:?} of {striker:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use super::*;

    fn controller(listener: &TcpListener) -> PlayerConnection {
        let address = listener.local_addr().expect("a bound address");
        let stream = TcpStream::connect(address).expect("the listener accepts");
        PlayerConnection::new(&stream).expect("a clone of the stream")
    }

    #[test]
    fn only_the_newest_connection_controls_the_character() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a free local port");
        let (older, newer) = (controller(&listener), controller(&listener));
        let mut controlled = Controlled {
            world: World::skeleton(0),
            controller: None,
        };
        controlled.take(&older);
        controlled.take(&newer);
        assert!(
            !controlled.is_controlled_by(&older),
            "the older still controls"
        );
        assert!(
            controlled.is_controlled_by(&newer),
            "the newer does not control"
        );
    }

    #[test]
    fn a_quarantined_world_stays_registered_and_replays_from_its_seed_and_log() {
        let registry = Registry::default();
        let token = ResumeToken(7);
        let faulted = registry.world(token).run(|controlled| {
            assert_eq!(controlled.world.submit(Action::Wait), Ok(()));
            assert_eq!(controlled.world.submit(Action::Wait), Ok(()));
            ash_core::invariant!("a test fault");
        });
        assert!(faulted.is_err());

        let hosted = registry.world(token);
        assert!(hosted.run(|_| ()).is_err(), "the quarantined world steps");
        let Err(poisoned) = hosted.controlled.lock() else {
            panic!("the quarantined world was replaced");
        };
        let world = poisoned.into_inner().world.clone();
        assert_eq!(world.command_log(), [Action::Wait, Action::Wait]);
        let mut replay = World::skeleton(world_seed(token));
        for &action in world.command_log() {
            assert_eq!(replay.submit(action), Ok(()));
        }
        assert_eq!(replay, world);
    }

    #[test]
    fn worlds_of_different_tokens_differ() {
        let registry = Registry::default();
        let [first, second] = [ResumeToken(1), ResumeToken(2)].map(|token| {
            let hosted = registry.world(token);
            let stream = hosted.run(|controlled| {
                for _ in 0..10 {
                    assert_eq!(controlled.world.submit(Action::Wait), Ok(()));
                }
                controlled.world.perceived_changes().to_vec()
            });
            (world_seed(token), stream.ok())
        });
        assert_ne!(first.0, second.0, "two worlds share a seed");
        assert_ne!(first.1, second.1, "two worlds behave the same");
    }
}
