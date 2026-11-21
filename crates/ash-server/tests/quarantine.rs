//! Acceptance of spec 0001, slice S5: quarantine and takeover.
//!
//! Two clients, each with its own world. A test fault makes the world of the
//! second violate an invariant: only that world stops. Then a second
//! connection with the first token takes the controller (R20).

use std::io::ErrorKind;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use ash_core::entity::EntityId;
use ash_core::tile::{Direction, TilePosition};
use ash_core::time::Time;
use ash_protocol::message::{Action, ClientMessage, PerceivedChange, ResumeToken, ServerMessage};
use tungstenite::{Error, Message, WebSocket};

use Direction::{East, South};

type Socket = WebSocket<TcpStream>;

const FIRST: ResumeToken = ResumeToken(1);
const FAULTY: ResumeToken = ResumeToken(2);

fn start_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a free local port");
    let address = listener.local_addr().expect("a bound address");
    thread::spawn(move || ash_server::serve_with_fault(listener, FAULTY));
    address
}

fn connect(address: SocketAddr, token: ResumeToken) -> Socket {
    let stream = TcpStream::connect(address).expect("the server accepts");
    // A missing reply fails the test instead of hanging it.
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("a read timeout");
    let (mut socket, _) =
        tungstenite::client(format!("ws://{address}/"), stream).expect("a WebSocket handshake");
    send(&mut socket, ClientMessage::Hello { token });
    socket
}

fn send(socket: &mut Socket, message: ClientMessage) {
    socket
        .send(Message::binary(message.encode()))
        .expect("the server reads");
}

/// The next message from the server; `None` when the connection ended, the
/// read timed out, or the bytes do not decode.
fn receive(socket: &mut Socket) -> Option<ServerMessage> {
    loop {
        match socket.read() {
            Ok(Message::Binary(bytes)) => return ServerMessage::decode(&bytes).ok(),
            Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_)) => {}
            Ok(Message::Text(_) | Message::Close(_)) | Err(_) => return None,
        }
    }
}

/// True when the server ends the connection; false when it sends a message
/// or stays silent until the read timeout.
fn closed(socket: &mut Socket) -> bool {
    loop {
        match socket.read() {
            // tungstenite answers a close frame; the next read sees the end.
            Ok(Message::Close(_) | Message::Ping(_) | Message::Pong(_) | Message::Frame(_)) => {}
            Ok(Message::Binary(_) | Message::Text(_)) => return false,
            Err(Error::Io(error))
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
            {
                return false;
            }
            Err(_) => return true,
        }
    }
}

fn at(x: u32, y: u32) -> TilePosition {
    TilePosition { x, y, z: 0 }
}

/// The snapshot and pending turn that greet a connection: where the
/// character stands, and the time of its turn.
fn greeting(socket: &mut Socket) -> (TilePosition, Time) {
    let position = match receive(socket) {
        Some(ServerMessage::Snapshot(snapshot)) => snapshot.entities[0].position,
        other => panic!("expected a snapshot, got {other:?}"),
    };
    match receive(socket) {
        Some(ServerMessage::Awaited { time, .. }) => (position, time),
        other => panic!("expected the pending turn, got {other:?}"),
    }
}

/// `character` moves toward `direction` at `time`: the server sends the
/// perceived move, then the next turn.
fn walk(socket: &mut Socket, character: EntityId, direction: Direction, time: u64) {
    let action = Action::Move(direction);
    send(
        socket,
        ClientMessage::Command {
            entity: character,
            action,
        },
    );
    assert_eq!(
        receive(socket),
        Some(ServerMessage::Perceived(PerceivedChange {
            time: Time::new(time),
            entity: character,
            action,
        }))
    );
    match receive(socket) {
        Some(ServerMessage::Awaited { entity, .. }) => assert_eq!(entity, character),
        other => panic!("expected the next turn, got {other:?}"),
    }
}

#[test]
fn a_fault_quarantines_one_world_and_a_new_connection_takes_the_controller() {
    let address = start_server();
    let character = ash_server::create_world().awaited();

    let mut first = connect(address, FIRST);
    assert_eq!(greeting(&mut first), (at(1, 1), Time::ZERO));
    walk(&mut first, character, East, 0);

    let mut faulty = connect(address, FAULTY);
    assert_eq!(greeting(&mut faulty), (at(1, 1), Time::ZERO));
    // The fault fires once the world accepts this command.
    send(
        &mut faulty,
        ClientMessage::Command {
            entity: character,
            action: Action::Wait,
        },
    );
    assert_eq!(receive(&mut faulty), Some(ServerMessage::Quarantined));
    assert!(
        closed(&mut faulty),
        "the quarantined client stays connected"
    );

    // Only the faulty world stops.
    walk(&mut first, character, East, 100);

    // A quarantined world is never stepped again, even for a new connection.
    let mut again = connect(address, FAULTY);
    assert_eq!(receive(&mut again), Some(ServerMessage::Quarantined));
    assert!(closed(&mut again), "a later client stays connected");

    // R20: the newest connection takes the controller; the older one is
    // told, then closed.
    let mut second = connect(address, FIRST);
    assert_eq!(receive(&mut first), Some(ServerMessage::ControllerTaken));
    assert!(closed(&mut first), "the older connection stays open");
    assert_eq!(greeting(&mut second), (at(3, 1), Time::new(200)));
    walk(&mut second, character, South, 200);
}
