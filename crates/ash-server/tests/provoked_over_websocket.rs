//! Acceptance of spec 0003, slice S2, over WebSocket: the change of
//! disposition of an attacked neutral creature reaches the client, and a
//! later snapshot shows the creature as hostile (A4).
//!
//! The client plays the commands of the web fixture `attack.json`: on the
//! world of this token, two steps bring the character next to the neutral
//! creature, then it attacks it. A replay of those commands plans the test.

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use ash_core::entity::EntityId;
use ash_protocol::message::{
    Action, ChangeKind, ClientMessage, Direction, Disposition, Outcome, PerceivedChange,
    ResumeToken, ServerMessage, Snapshot,
};
use ash_sim::world::World;
use tungstenite::{Message, WebSocket};

#[path = "websocket/action.rs"]
mod action;
#[path = "websocket/change.rs"]
mod change;

type Socket = WebSocket<TcpStream>;

/// The token of the web fixture `attack.json`.
const TOKEN: ResumeToken = ResumeToken(19);

/// Two steps toward the neutral creature, then an attack on it.
const COMMANDS: [Action; 3] = [
    Action::Move(Direction::SouthEast),
    Action::Move(Direction::SouthEast),
    Action::Attack(Direction::NorthWest),
];

fn start_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a free local port");
    let address = listener.local_addr().expect("a bound address");
    thread::spawn(move || ash_server::serve(listener));
    address
}

/// Connects with `TOKEN`, having received `received` perceived changes of
/// its world.
fn connect(address: SocketAddr, received: u64) -> Socket {
    let stream = TcpStream::connect(address).expect("the server accepts");
    // A missing reply fails the test instead of hanging it.
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("a read timeout");
    let (mut socket, _) =
        tungstenite::client(format!("ws://{address}/"), stream).expect("a WebSocket handshake");
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

/// The perceived changes the server sends in a row, then the message after
/// them.
fn receive_turn(socket: &mut Socket) -> (Vec<PerceivedChange>, Option<ServerMessage>) {
    let mut changes = Vec::new();
    loop {
        match receive(socket) {
            Some(ServerMessage::Perceived(change)) => changes.push(change),
            other => return (changes, other),
        }
    }
}

/// The disposition the snapshot `message` shows for `creature`.
fn disposition(message: Option<ServerMessage>, creature: EntityId) -> Option<Disposition> {
    let Some(ServerMessage::Snapshot(Snapshot { entities, .. })) = message else {
        panic!("expected a snapshot, got {message:?}");
    };
    entities
        .iter()
        .find(|perceived| perceived.entity == creature)
        .and_then(|perceived| perceived.disposition)
}

#[test]
fn the_change_of_disposition_reaches_the_client_and_a_later_snapshot_shows_it() {
    let address = start_server();
    let mut replay = World::skeleton(ash_server::world_seed(TOKEN));
    let character = replay.awaited();
    let mut socket = connect(address, 0);

    let Some(ServerMessage::Snapshot(first)) = receive(&mut socket) else {
        panic!("expected the first snapshot");
    };
    let neutral = first
        .entities
        .iter()
        .find(|perceived| perceived.disposition == Some(Disposition::Neutral))
        .map(|perceived| perceived.entity)
        .expect("the world holds a neutral creature");

    let mut received = Vec::new();
    let mut reply = Vec::new();
    let mut turn = receive(&mut socket);
    for action in COMMANDS {
        let Some(ServerMessage::Awaited { entity, offers, .. }) = turn else {
            panic!("expected the character's turn before {action:?}, got {turn:?}");
        };
        assert_eq!(entity, character);
        // The server and the replay agree: the same turn, the same offers.
        let offered: Vec<Action> = offers.iter().map(|offer| offer.action).collect();
        let expected: Vec<Action> = replay
            .offers()
            .into_iter()
            .map(|offer| action::to_wire(offer.action))
            .collect();
        assert_eq!(offered, expected, "before {action:?}");
        send(
            &mut socket,
            ClientMessage::Command {
                entity: character,
                action,
            },
        );
        let seen = replay.perceived_changes().len();
        assert_eq!(replay.submit(action::to_sim(action)), Ok(()));
        // R17: the server sends what the simulation perceived, in order.
        (reply, turn) = receive_turn(&mut socket);
        let expected: Vec<PerceivedChange> = replay.perceived_changes()[seen..]
            .iter()
            .copied()
            .map(change::to_wire)
            .collect();
        assert_eq!(reply, expected, "after {action:?}");
        received.extend(reply.iter().copied());
    }

    // A4, A11: the reply to the attack opens with its hit or its miss of the
    // neutral creature. Its last change, before any other entity acts, is
    // the change of disposition, at the time of the attack.
    let time = reply[0].time;
    assert!(
        matches!(
            reply[0],
            PerceivedChange {
                entity,
                kind: ChangeKind::Act(Outcome::Hit(target) | Outcome::Missed(target)),
                ..
            } if entity == character && target == neutral
        ),
        "{reply:?}"
    );
    let last_of_the_attack = reply[1..]
        .iter()
        .take_while(|change| !matches!(change.kind, ChangeKind::Act(_)))
        .last();
    assert_eq!(
        last_of_the_attack,
        Some(&PerceivedChange {
            time,
            entity: neutral,
            kind: ChangeKind::Disposition(Disposition::Hostile),
        }),
        "{reply:?}"
    );
    drop(socket);

    // A later connection, at each count of changes it holds, perceives the
    // creature as hostile (A4, R16).
    let count = u64::try_from(received.len()).expect("a small count");
    for held in [0, count] {
        let mut socket = connect(address, held);
        let (changes, snapshot) = receive_turn(&mut socket);
        let skipped = usize::try_from(held).expect("a small count");
        assert_eq!(changes, received[skipped..], "{held} changes held");
        assert_eq!(
            disposition(snapshot, neutral),
            Some(Disposition::Hostile),
            "{held} changes held"
        );
    }
}
