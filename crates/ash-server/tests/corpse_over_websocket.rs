//! Acceptance of spec 0003, slice S1, over WebSocket: a death leaves a
//! corpse, and the snapshot of a later connection holds it (D2, D4).
//!
//! The client plays the commands of the web fixture `death.json`: on the
//! world of this token, the second of two strikes kills the hostile
//! creature. A replay of those commands gives the tile of the death.

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use ash_protocol::message::{
    Action, ClientMessage, Died, Direction, Disposition, PerceivedCorpse, ResumeToken,
    ServerMessage, Snapshot,
};
use ash_sim::perception::{self, ChangeKind};
use ash_sim::world::World;
use tungstenite::{Message, WebSocket};

#[path = "websocket/action.rs"]
mod action;

type Socket = WebSocket<TcpStream>;

/// The token of the web fixture `death.json`.
const TOKEN: ResumeToken = ResumeToken(15);

/// Two steps toward the hostile creature, then two strikes: both hit, and
/// the second kills it.
const COMMANDS: [Action; 4] = [
    Action::Move(Direction::East),
    Action::Move(Direction::SouthEast),
    Action::Attack(Direction::SouthEast),
    Action::Attack(Direction::SouthEast),
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

/// The first message after the perceived changes the server sends in a row.
fn after_changes(socket: &mut Socket) -> Option<ServerMessage> {
    loop {
        match receive(socket) {
            Some(ServerMessage::Perceived(_)) => {}
            other => return other,
        }
    }
}

/// The snapshot `message` is.
fn snapshot(message: Option<ServerMessage>) -> Snapshot {
    match message {
        Some(ServerMessage::Snapshot(snapshot)) => snapshot,
        other => panic!("expected a snapshot, got {other:?}"),
    }
}

#[test]
fn the_snapshot_of_a_later_connection_holds_the_corpse_of_the_killed_creature() {
    let address = start_server();
    let mut replay = World::skeleton(ash_server::world_seed(TOKEN));
    let character = replay.awaited();
    let mut socket = connect(address, 0);

    let first = snapshot(receive(&mut socket));
    assert_eq!(first.corpses, []);
    let hostile = first
        .entities
        .iter()
        .find(|perceived| perceived.disposition == Some(Disposition::Hostile))
        .map(|perceived| perceived.entity)
        .expect("the world holds a hostile creature");

    // The tile of the death is where the creature stood when the lethal
    // strike resolved, before any other entity acted (R2).
    let mut died_at = None;
    let mut turn = receive(&mut socket);
    for action in COMMANDS {
        // The server and the replay agree: the same turn, the same offers.
        let Some(ServerMessage::Awaited { entity, offers, .. }) = &turn else {
            panic!("expected the character's turn before {action:?}, got {turn:?}");
        };
        assert_eq!(*entity, character);
        let offered: Vec<Action> = offers.iter().map(|offer| offer.action).collect();
        let expected: Vec<Action> = replay
            .offers()
            .into_iter()
            .map(|offer| action::to_wire(offer.action))
            .collect();
        assert_eq!(offered, expected, "before {action:?}");
        died_at = Some(replay.position(hostile));
        send(
            &mut socket,
            ClientMessage::Command {
                entity: character,
                action,
            },
        );
        assert_eq!(replay.submit(action::to_sim(action)), Ok(()));
        turn = after_changes(&mut socket);
    }
    let died: Vec<_> = replay
        .perceived_changes()
        .iter()
        .filter(|change| change.kind == ChangeKind::Death)
        .map(|change| change.entity)
        .collect();
    assert_eq!(
        died,
        [hostile],
        "the commands of the fixture kill the creature"
    );
    let died_at = died_at.expect("the client struck");
    // The replay shows the creature no more: the strike was lethal.
    assert!(
        perception::snapshot(&replay, character)
            .entities
            .iter()
            .all(|perceived| perceived.entity != hostile)
    );
    drop(socket);

    // D2, D4: a later connection, at each count of changes it holds,
    // perceives the corpse on the tile of the death: a creature, the dead id.
    let changes = u64::try_from(replay.perceived_changes().len()).expect("a small count");
    for received in [0, changes] {
        let mut socket = connect(address, received);
        let later = snapshot(after_changes(&mut socket));
        assert_eq!(
            later.corpses,
            [PerceivedCorpse {
                position: died_at,
                died: Died::Creature,
                entity: Some(hostile),
            }],
            "{received} changes held"
        );
    }
}
