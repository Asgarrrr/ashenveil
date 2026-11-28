//! Acceptance of spec 0003, slice S5, over WebSocket: a struck character
//! perceives its own hit points (A9), and the client perceives the
//! condition of the character it strikes, never its hit points (A10, R18).
//!
//! On the test entry, the world of `TOKEN` holds the client's character on
//! the start tile (1, 1), and a second character east of it that the server
//! drives: whenever the world awaits it, it attacks west, toward the start
//! tile, or waits when no entity stands there. The test plays the same rule
//! on a replay of the world. The changes the server must send are those of
//! the characters the client's controller drove, never the list of the
//! world.

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use ash_core::entity::{EntityId, Speed};
use ash_core::tile::{Direction, TilePosition};

use ash_protocol::message::{
    Action, ChangeKind, ClientMessage, Condition, Offer, Outcome, PerceivedChange, ResumeToken,
    ServerMessage,
};
use ash_sim::action::Action as SimAction;
use ash_sim::controller::Controller;
use ash_sim::world::World;
use tungstenite::{Message, WebSocket};

#[path = "websocket/action.rs"]
mod action;
#[path = "websocket/change.rs"]
mod change;

type Socket = WebSocket<TcpStream>;

/// A6: the world of this token draws from seed 2, whose first five draws
/// hit. The second character hits three times, the client's strike hits,
/// and the next hit of the second character kills the client's character.
const TOKEN: ResumeToken = ResumeToken(2);
/// The waits after which a character that has not reached its hit points
/// fails the test.
const MOST_WAITS: usize = 50;

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

/// Every message of one greeting or one reply: up to the turn of the
/// client's character or a refusal, both included. An ended connection or
/// bytes that do not decode end it early.
fn reply(socket: &mut Socket) -> Vec<ServerMessage> {
    let mut messages = Vec::new();
    loop {
        let message = match socket.read() {
            Ok(Message::Binary(bytes)) => ServerMessage::decode(&bytes).ok(),
            Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_)) => continue,
            Ok(Message::Text(_) | Message::Close(_)) | Err(_) => None,
        };
        match message {
            Some(message @ (ServerMessage::Awaited { .. } | ServerMessage::Refused)) => {
                messages.push(message);
                return messages;
            }
            Some(message) => messages.push(message),
            None => return messages,
        }
    }
}

/// The perceived changes `messages` hold, in order.
fn changes(messages: &[ServerMessage]) -> Vec<PerceivedChange> {
    messages
        .iter()
        .filter_map(|message| match message {
            ServerMessage::Perceived(change) => Some(*change),
            _ => None,
        })
        .collect()
}

/// The client's side of the session and the replay that predicts it.
struct Session {
    address: SocketAddr,
    socket: Socket,
    world: World,
    first: EntityId,
    second: EntityId,
    /// Every message the server sent on any connection of the session.
    received: Vec<ServerMessage>,
}

impl Session {
    /// The replay of the test entry, and a first connection that holds no
    /// change: its greeting is the snapshot of the client's character and
    /// its turn.
    fn open() -> Self {
        let address = start_server();
        let mut world = World::new(ash_server::world_seed(TOKEN), Speed::new(1));
        let first = world.awaited();
        let second = world.create_entity(Speed::new(1), EAST_OF_START, Controller::Player);
        let mut session = Self {
            address,
            socket: connect(address, 0),
            world,
            first,
            second,
            received: Vec::new(),
        };
        let greeting = session.read();
        assert!(
            matches!(&greeting[..], [ServerMessage::Snapshot(_), ServerMessage::Awaited { entity, .. }] if *entity == first),
            "{greeting:?}"
        );
        session
    }

    /// The next greeting or reply, kept among the messages received.
    fn read(&mut self) -> Vec<ServerMessage> {
        let messages = reply(&mut self.socket);
        self.received.extend_from_slice(&messages);
        messages
    }

    /// D6: the character the client's controller drives now.
    fn driven(&self) -> EntityId {
        let mut character = self.first;
        while let Some(stranger) = self.world.stranger_of(character) {
            character = stranger;
        }
        character
    }

    /// R17, D11: the changes of each character the client's controller
    /// drove, in turn, as the server must send them.
    fn controller_changes(&self) -> Vec<ServerMessage> {
        std::iter::successors(Some(self.first), |&character| {
            self.world.stranger_of(character)
        })
        .flat_map(|character| self.world.perceived_changes_of(character))
        .map(|change| ServerMessage::Perceived(change::to_wire(change)))
        .collect()
    }

    /// The driven character takes `action`; returns the reply. The replay
    /// takes it too, and the server's rule for the second character.
    /// Asserts the reply holds the controller's new changes, then the
    /// snapshot of a stranger that arrived meanwhile, then the next turn.
    fn command(&mut self, action: Action) -> Vec<ServerMessage> {
        let drove = self.driven();
        let seen = self.controller_changes().len();
        send(
            &mut self.socket,
            ClientMessage::Command {
                entity: drove,
                action,
            },
        );
        let reply = self.read();
        assert_eq!(self.world.submit(action::to_sim(action)), Ok(()));
        while self.world.awaited() == self.second {
            let attack = SimAction::Attack(Direction::West);
            let offered = self
                .world
                .offers()
                .iter()
                .any(|offer| offer.action == attack);
            let action = if offered { attack } else { SimAction::Wait };
            assert_eq!(self.world.submit(action), Ok(()), "{action:?}");
        }

        let expected = &self.controller_changes()[seen..];
        assert_eq!(
            reply[..expected.len().min(reply.len())],
            *expected,
            "{reply:?}"
        );
        let rest = &reply[expected.len().min(reply.len())..];
        let driven = self.driven();
        let turn = self.awaited(driven);
        if driven == drove {
            assert_eq!(rest, [turn], "{reply:?}");
        } else {
            assert!(
                matches!(rest, [ServerMessage::Snapshot(_), last] if *last == turn),
                "{reply:?}"
            );
        }
        reply
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

/// Each hit on `target` among `changes`, with the change that follows it.
fn each_hit_on(
    target: EntityId,
    changes: &[PerceivedChange],
) -> Vec<(PerceivedChange, PerceivedChange)> {
    changes
        .windows(2)
        .filter(|pair| pair[0].kind == ChangeKind::Act(Outcome::Hit(target)))
        .map(|pair| (pair[0], pair[1]))
        .collect()
}

#[test]
fn the_client_perceives_its_own_hit_points_and_the_condition_of_the_character_it_strikes() {
    let mut session = Session::open();
    let (first, second) = (session.first, session.second);
    let hit_points = |time, current| PerceivedChange {
        time,
        entity: first,
        kind: ChangeKind::HitPoints { current, max: 10 },
    };

    // A9: the client waits until its character has 1 hit point left. Each
    // hit it survives is followed by its hit points, at the time of the
    // hit: 7, 4, then 1 of 10.
    let left_one = |change: &PerceivedChange| change.kind == hit_points(change.time, 1).kind;
    let mut waits = 0;
    while !changes(&session.received).iter().any(left_one) {
        assert!(waits < MOST_WAITS, "no third hit in {MOST_WAITS} waits");
        waits += 1;
        session.command(Action::Wait);
    }
    let waited = changes(&session.received);
    let hits = each_hit_on(first, &waited);
    let after_hits: Vec<_> = hits.iter().map(|&(_, after)| after).collect();
    let expected: Vec<_> = hits
        .iter()
        .zip([7, 4, 1])
        .map(|(&(hit, _), current)| hit_points(hit.time, current))
        .collect();
    assert_eq!((after_hits.len(), &after_hits), (3, &expected));
    let given: Vec<_> = waited
        .iter()
        .filter(|change| matches!(change.kind, ChangeKind::HitPoints { .. }))
        .copied()
        .collect();
    assert_eq!(given, after_hits);

    // A10, R18: the client strikes the second character once and hits. It
    // receives the condition of that character, right after the hit.
    let struck = session.command(Action::Attack(Direction::East));
    let struck = changes(&struck);
    let time = struck[0].time;
    assert_eq!(
        struck[..2],
        [
            PerceivedChange {
                time,
                entity: first,
                kind: ChangeKind::Act(Outcome::Hit(second)),
            },
            PerceivedChange {
                time,
                entity: second,
                kind: ChangeKind::Condition(Condition::Wounded),
            },
        ]
    );

    // A11, D10: the next hit kills the client's character. The death
    // follows the lethal hit, with no hit points, then the arrival.
    if session.world.stranger_of(first).is_none() {
        session.command(Action::Wait);
    }
    let Some(stranger) = session.world.stranger_of(first) else {
        panic!("the client's character lives: {:?}", session.received);
    };
    let all = changes(&session.received);
    let lethal = all
        .iter()
        .rposition(|change| change.kind == ChangeKind::Act(Outcome::Hit(first)))
        .expect("a lethal hit");
    let time = all[lethal].time;
    assert_eq!(
        all[lethal..],
        [
            PerceivedChange {
                time,
                entity: second,
                kind: ChangeKind::Act(Outcome::Hit(first)),
            },
            PerceivedChange {
                time,
                entity: first,
                kind: ChangeKind::Death,
            },
            PerceivedChange {
                time,
                entity: stranger,
                kind: ChangeKind::Arrival(TilePosition { x: 1, y: 1, z: 0 }),
            },
        ]
    );

    // R17, D11: a connection at each count receives exactly the rest of the
    // changes of its controller, then the snapshot of the stranger and its
    // turn. The world holds more: the conditions of the client's character
    // and the hit points of the second.
    let controller = session.controller_changes();
    assert!(
        controller.len() < session.world.perceived_changes().len(),
        "the world holds no change the controller lacks"
    );
    session.socket = connect(session.address, controller.len());
    let greeting = session.read();
    assert!(
        matches!(&greeting[..], [ServerMessage::Snapshot(_), turn] if *turn == session.awaited(stranger)),
        "{greeting:?}"
    );
    for received in 0..=controller.len() {
        session.socket = connect(session.address, received);
        let mut expected = controller[received..].to_vec();
        expected.extend_from_slice(&greeting);
        assert_eq!(session.read(), expected, "{received} held");
    }

    // A9, A10, R18: no message names a condition for the client's own
    // characters, or hit points for the second character.
    let own = [first, stranger];
    let crossed: Vec<_> = changes(&session.received)
        .into_iter()
        .filter(|change| match change.kind {
            ChangeKind::Condition(_) => own.contains(&change.entity),
            ChangeKind::HitPoints { .. } => change.entity == second,
            _ => false,
        })
        .collect();
    assert_eq!(crossed, []);
}
