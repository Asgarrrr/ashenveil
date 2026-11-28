//! Acceptance of spec 0003, slice S4, over WebSocket: the controller
//! follows the stranger, which knows nothing of the past (D6, D10, D11).
//!
//! On the test entry, the world of `TOKEN` holds the client's character on
//! the start tile (1, 1), and a second character east of it that the server
//! drives: whenever the world awaits it, it attacks west, toward the start
//! tile, or waits when no entity stands there. The test plays the same rule
//! on a replay of the world, which gives the changes the server must send:
//! those of the characters the client's controller drove (spec 0003, S5).

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use ash_core::entity::{EntityId, Speed};
use ash_core::tile::{Direction, TilePosition};

use ash_protocol::message::{
    Action, ClientMessage, Condition, Died, Offer, PerceivedCorpse, PerceivedEntity,
    PerceivedHealth, ResumeToken, ServerMessage, Snapshot,
};
use ash_sim::action::Action as SimAction;
use ash_sim::controller::Controller;
use ash_sim::perception::{ChangeKind, Outcome, PerceivedChange};
use ash_sim::world::World;
use tungstenite::{Message, WebSocket};

#[path = "websocket/action.rs"]
mod action;
#[path = "websocket/change.rs"]
mod change;

type Socket = WebSocket<TcpStream>;

const TOKEN: ResumeToken = ResumeToken(0x0003_0004);
/// The waits after which a character that has not died fails the test.
const MOST_WAITS: usize = 200;

/// Where the first character of every world starts (D8).
const START: TilePosition = TilePosition { x: 1, y: 1, z: 0 };
/// Where the second character stands, east of the start tile.
const EAST_OF_START: TilePosition = TilePosition { x: 2, y: 1, z: 0 };

fn start_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a free local port");
    let address = listener.local_addr().expect("a bound address");
    thread::spawn(move || ash_server::serve_with_struck_character(listener, TOKEN));
    address
}

/// Connects with `TOKEN`, having received `received` perceived changes.
fn connect(address: SocketAddr, received: usize) -> Socket {
    let stream = TcpStream::connect(address).expect("the server accepts");
    // A missing reply fails the test instead of hanging it.
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("a read timeout");
    let (mut socket, _) =
        tungstenite::client(format!("ws://{address}/"), stream).expect("a WebSocket handshake");
    let received = u64::try_from(received).expect("a small count");
    send(
        &mut socket,
        ClientMessage::Hello {
            token: TOKEN,
            received,
        },
    );
    socket
}

fn send(socket: &mut Socket, message: ClientMessage) {
    socket
        .send(Message::binary(message.encode()))
        .expect("the server reads");
}

/// The next message from the server; `None` when the connection ended or the
/// bytes do not decode.
fn receive(socket: &mut Socket) -> Option<ServerMessage> {
    loop {
        match socket.read() {
            Ok(Message::Binary(bytes)) => return ServerMessage::decode(&bytes).ok(),
            Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_)) => {}
            Ok(Message::Text(_) | Message::Close(_)) | Err(_) => return None,
        }
    }
}

/// Every message of one greeting or one reply: up to the turn of the
/// client's character or a refusal, both included.
fn reply(socket: &mut Socket) -> Vec<ServerMessage> {
    let mut messages = Vec::new();
    loop {
        match receive(socket) {
            Some(message @ (ServerMessage::Awaited { .. } | ServerMessage::Refused)) => {
                messages.push(message);
                return messages;
            }
            Some(message) => messages.push(message),
            None => panic!("the server sent {messages:?}, then nothing"),
        }
    }
}

/// The world of the test entry, replayed: the client's character on the
/// start tile, then the second character east of it, both at speed 1.
fn replay() -> (World, EntityId, EntityId) {
    let mut world = World::new(ash_server::world_seed(TOKEN), Speed::new(1));
    let first = world.awaited();
    let second = world.create_entity(Speed::new(1), EAST_OF_START, Controller::Player);
    (world, first, second)
}

/// The rule of the test entry: while the world awaits `second`, it attacks
/// west when that attack is offered, and waits otherwise.
fn drive_second(world: &mut World, second: EntityId) {
    while world.awaited() == second {
        let attack = SimAction::Attack(Direction::West);
        let offered = world.offers().iter().any(|offer| offer.action == attack);
        let action = if offered { attack } else { SimAction::Wait };
        assert_eq!(world.submit(action), Ok(()), "{action:?}");
    }
}

/// The client's side of the session and the replay that predicts it.
struct Session {
    socket: Socket,
    world: World,
    /// The first character the client's controller drove.
    first: EntityId,
    second: EntityId,
}

impl Session {
    /// `entity` takes `action`; returns the reply. The replay takes it too,
    /// unless the server refused it.
    fn command(&mut self, entity: EntityId, action: Action) -> Vec<ServerMessage> {
        send(&mut self.socket, ClientMessage::Command { entity, action });
        let reply = reply(&mut self.socket);
        if reply != [ServerMessage::Refused] {
            assert_eq!(self.world.submit(action::to_sim(action)), Ok(()));
            drive_second(&mut self.world, self.second);
        }
        reply
    }

    /// `character` waits until it dies. Returns the count of changes before
    /// the command during which it died, and the reply to that command.
    fn wait_until_dead(&mut self, character: EntityId) -> (usize, Vec<ServerMessage>) {
        for _ in 0..MOST_WAITS {
            let seen = self.controller().len();
            let reply = self.command(character, Action::Wait);
            if self.world.stranger_of(character).is_some() {
                return (seen, reply);
            }
            assert!(
                matches!(reply.last(), Some(ServerMessage::Awaited { entity, .. }) if *entity == character),
                "{reply:?}"
            );
        }
        panic!("{character:?} still lives after {MOST_WAITS} waits");
    }

    /// R17, D11: the changes of each character the client's controller
    /// drove, in turn, on the replay.
    fn controller(&self) -> Vec<PerceivedChange> {
        std::iter::successors(Some(self.first), |&character| {
            self.world.stranger_of(character)
        })
        .flat_map(|character| self.world.perceived_changes_of(character))
        .collect()
    }

    /// The changes of the controller after the first `seen`, as the server
    /// must send them.
    fn changes_since(&self, seen: usize) -> Vec<ServerMessage> {
        self.controller()[seen..]
            .iter()
            .map(|&change| ServerMessage::Perceived(change::to_wire(change)))
            .collect()
    }

    /// The turn of `character` as the server must send it.
    fn awaited(&self, character: EntityId) -> ServerMessage {
        assert_eq!(self.world.awaited(), character);
        ServerMessage::Awaited {
            time: self.world.clock(),
            entity: character,
            offers: self
                .world
                .offers()
                .into_iter()
                .map(|offer| Offer {
                    action: action::to_wire(offer.action),
                    cost: offer.cost,
                })
                .collect(),
        }
    }
}

/// Opens the session: the greeting of a first connection is the snapshot
/// of the client's character and its turn, with no change before them.
fn open(address: SocketAddr) -> (Session, EntityId) {
    let (world, first, second) = replay();
    let mut socket = connect(address, 0);
    let greeting = reply(&mut socket);
    let session = Session {
        socket,
        world,
        first,
        second,
    };
    assert!(
        matches!(&greeting[..], [ServerMessage::Snapshot(_), ServerMessage::Awaited { entity, .. }] if *entity == first),
        "{greeting:?}"
    );
    (session, first)
}

/// D10: the changes of the command during which `dead` died: its wait, the
/// lethal hit, the death, then the arrival of `stranger` on the start tile,
/// at the time of the hit.
fn assert_died_waiting(
    changes: &[PerceivedChange],
    dead: EntityId,
    second: EntityId,
    stranger: EntityId,
) {
    let [waited, hit, death, arrival] = changes else {
        panic!("{changes:?}");
    };
    assert_eq!(
        (waited.entity, waited.kind),
        (dead, ChangeKind::Act(Outcome::Waited))
    );
    let time = hit.time;
    assert_eq!(
        [*hit, *death, *arrival],
        [
            PerceivedChange {
                time,
                entity: second,
                kind: ChangeKind::Act(Outcome::Hit(dead)),
            },
            PerceivedChange {
                time,
                entity: dead,
                kind: ChangeKind::Death,
            },
            PerceivedChange {
                time,
                entity: stranger,
                kind: ChangeKind::Arrival(START),
            },
        ]
    );
}

/// The snapshot `message` is.
fn snapshot(message: &ServerMessage) -> &Snapshot {
    match message {
        ServerMessage::Snapshot(snapshot) => snapshot,
        other => panic!("expected a snapshot, got {other:?}"),
    }
}

/// D6, A7: what `stranger` perceives right after its arrival on the start
/// tile: the unhurt second character, and its own 10 hit points of 10.
fn arrived(stranger: EntityId, second: EntityId) -> Vec<PerceivedEntity> {
    vec![
        PerceivedEntity {
            entity: second,
            position: EAST_OF_START,
            disposition: None,
            health: PerceivedHealth::Condition(Condition::Unhurt),
        },
        PerceivedEntity {
            entity: stranger,
            position: START,
            disposition: None,
            health: PerceivedHealth::HitPoints {
                current: 10,
                max: 10,
            },
        },
    ]
}

/// D2, D4, D11: the corpse of a character that died on the start tile
/// before the viewer arrived: a character, with no entity named.
const UNNAMED_CHARACTER_CORPSE: PerceivedCorpse = PerceivedCorpse {
    position: START,
    died: Died::Character,
    entity: None,
};

#[test]
fn the_reply_to_the_lethal_command_holds_the_death_the_arrival_and_the_stranger() {
    let address = start_server();
    let (mut session, first) = open(address);

    let (seen, lethal) = session.wait_until_dead(first);

    // D10, D11: the lethal hit, the death, the arrival, the snapshot of the
    // stranger and its turn, in that order, and nothing else.
    let stranger = session.world.stranger_of(first).expect("a stranger");
    assert_died_waiting(
        &session.controller()[seen..],
        first,
        session.second,
        stranger,
    );
    let changes = session.changes_since(seen);
    assert_eq!(lethal.len(), changes.len() + 2, "{lethal:?}");
    assert_eq!(lethal[..changes.len()], changes);
    let snapshot = snapshot(&lethal[changes.len()]);
    assert_eq!(snapshot.time, session.world.clock());
    assert_eq!(snapshot.entities, arrived(stranger, session.second));
    assert_eq!(snapshot.corpses, [UNNAMED_CHARACTER_CORPSE]);
    assert_eq!(lethal[changes.len() + 1], session.awaited(stranger));

    // The next command moves the stranger. No entity stands on the start
    // tile then, so the second character waits.
    let seen = session.controller().len();
    let moved = session.command(stranger, Action::Move(Direction::South));
    let mut expected = session.changes_since(seen);
    assert_eq!(
        expected.first(),
        Some(&ServerMessage::Perceived(change::to_wire(
            PerceivedChange {
                time: session.controller()[seen].time,
                entity: stranger,
                kind: ChangeKind::Act(Outcome::Stepped(Direction::South)),
            }
        )))
    );
    expected.push(session.awaited(stranger));
    assert_eq!(moved, expected);
}

/// The test entry: the server makes the second character act on every turn
/// the world awaits it, however many come in a row, before it answers.
#[test]
fn the_striker_acts_on_every_turn_the_world_awaits_it() {
    let address = start_server();
    let (mut session, first) = open(address);
    session.wait_until_dead(first);
    let stranger = session.world.stranger_of(first).expect("a stranger");

    // R6, R8: each move costs the stranger 100, each wait 50 to the second
    // character. After the move west, the world awaits the second character
    // twice in a row. The stranger walks a loop back to the start tile, and
    // the second character strikes it there.
    for direction in [
        Direction::South,
        Direction::East,
        Direction::West,
        Direction::North,
    ] {
        let seen = session.controller().len();
        let moved = session.command(stranger, Action::Move(direction));
        let mut expected = session.changes_since(seen);
        expected.push(session.awaited(stranger));
        assert_eq!(moved, expected, "{direction:?}");
    }
}

#[test]
fn a_command_for_the_dead_character_is_refused_and_every_count_resumes_through_two_deaths() {
    let address = start_server();
    let (mut session, first) = open(address);
    let (_, lethal) = session.wait_until_dead(first);
    let stranger = session.world.stranger_of(first).expect("a stranger");

    // D6, R10: a command that names the dead character is refused. Nothing
    // advanced: a connection that holds every change gets none, and the
    // same snapshot and turn as the reply to the lethal command.
    assert_eq!(
        session.command(first, Action::Wait),
        [ServerMessage::Refused]
    );
    let held = session.controller().len();
    session.socket = connect(address, held);
    assert_eq!(reply(&mut session.socket), lethal[lethal.len() - 2..]);

    // R17, D11: a connection that holds no change receives every change from
    // the first, then the snapshot of the stranger and its turn.
    session.socket = connect(address, 0);
    let mut expected = session.changes_since(0);
    expected.extend_from_slice(&lethal[lethal.len() - 2..]);
    assert_eq!(reply(&mut session.socket), expected);

    // D13: the stranger dies in turn, and the client controls a second
    // stranger.
    let (seen, lethal) = session.wait_until_dead(stranger);
    let second_stranger = session.world.stranger_of(stranger).expect("a stranger");
    assert_died_waiting(
        &session.controller()[seen..],
        stranger,
        session.second,
        second_stranger,
    );
    let changes = session.changes_since(seen);
    assert_eq!(lethal[..changes.len()], changes);
    let snapshot_of_second = snapshot(&lethal[changes.len()]).clone();
    assert_eq!(
        snapshot_of_second.entities,
        arrived(second_stranger, session.second)
    );
    assert_eq!(
        snapshot_of_second.corpses,
        [UNNAMED_CHARACTER_CORPSE, UNNAMED_CHARACTER_CORPSE]
    );
    assert_eq!(
        lethal[changes.len() + 1..],
        [session.awaited(second_stranger)]
    );
    for dead in [first, stranger] {
        assert_eq!(
            session.command(dead, Action::Wait),
            [ServerMessage::Refused],
            "{dead:?}"
        );
    }
    let seen = session.controller().len();
    let waited = session.command(second_stranger, Action::Wait);
    let mut expected = session.changes_since(seen);
    expected.push(session.awaited(second_stranger));
    assert_eq!(waited, expected);

    // R17: a connection at each count receives exactly the rest of the
    // changes, then the snapshot of the second stranger and its turn. The
    // counts stand before each lethal hit, after each death and after each
    // arrival, and at both ends.
    let all = session.controller().to_vec();
    let after = |found: &dyn Fn(&PerceivedChange) -> bool| {
        all.iter()
            .enumerate()
            .filter(|(_, change)| found(change))
            .map(|(index, _)| index + 1)
            .collect::<Vec<_>>()
    };
    let deaths = after(&|change| change.kind == ChangeKind::Death);
    let arrivals = after(&|change| matches!(change.kind, ChangeKind::Arrival(_)));
    let lethal_hits: Vec<usize> = deaths.iter().map(|&after_death| after_death - 2).collect();
    assert_eq!((deaths.len(), arrivals.len()), (2, 2));
    let counts = [vec![0, all.len()], lethal_hits, deaths, arrivals].concat();
    session.socket = connect(address, all.len());
    let greeting = reply(&mut session.socket);
    let [ServerMessage::Snapshot(now), turn] = &greeting[..] else {
        panic!("{greeting:?}");
    };
    // D2, D4: both corpses cross as a character's, which the second stranger
    // does not recognize.
    assert_eq!(
        now.corpses,
        [UNNAMED_CHARACTER_CORPSE, UNNAMED_CHARACTER_CORPSE]
    );
    assert_eq!(*turn, session.awaited(second_stranger));
    for received in counts {
        session.socket = connect(address, received);
        let mut expected = session.changes_since(received);
        expected.extend_from_slice(&greeting);
        assert_eq!(reply(&mut session.socket), expected, "{received} held");
    }
    // The replay agrees with the server on the clock it reached.
    assert_eq!(now.time, session.world.clock());
}
