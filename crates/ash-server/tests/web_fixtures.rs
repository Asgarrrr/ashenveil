//! The fixtures of the web client's `web_model` test, in
//! `web/tests/fixtures/`, are recordings of this server. Each test records
//! its session again and compares the bytes with the fixture. A change of
//! what a world sends, such as its seed derivation or its creature start,
//! then fails here instead of leaving `web_model` green on stale bytes.
//! When the change is meant, write the new recording into the fixture.

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use ash_protocol::message::{Action, ClientMessage, Direction, ResumeToken, ServerMessage};
use tungstenite::{Message, WebSocket};

use Direction::{East, North, NorthWest, SouthEast, West};

type Socket = WebSocket<TcpStream>;

fn start_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a free local port");
    let address = listener.local_addr().expect("a bound address");
    thread::spawn(move || ash_server::serve(listener));
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
    send(&mut socket, ClientMessage::Hello { token, received: 0 });
    socket
}

fn send(socket: &mut Socket, message: ClientMessage) {
    socket
        .send(Message::binary(message.encode()))
        .expect("the server reads");
}

/// The next message from the server.
fn receive(socket: &mut Socket) -> Vec<u8> {
    loop {
        match socket.read() {
            Ok(Message::Binary(bytes)) => return bytes.to_vec(),
            Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_)) => {}
            other => panic!("expected a message, got {other:?}"),
        }
    }
}

/// The greeting, then each of `commands` for the awaited entity with every
/// message up to the turn or the refusal it causes.
fn record(socket: &mut Socket, commands: &[Action]) -> Vec<Vec<u8>> {
    let mut messages = vec![receive(socket)];
    let awaited = receive(socket);
    let entity = match ServerMessage::decode(&awaited) {
        Ok(ServerMessage::Awaited { entity, .. }) => entity,
        other => panic!("expected the first turn, got {other:?}"),
    };
    messages.push(awaited);
    for &action in commands {
        send(socket, ClientMessage::Command { entity, action });
        loop {
            let message = receive(socket);
            let perceived = matches!(
                ServerMessage::decode(&message),
                Ok(ServerMessage::Perceived(_))
            );
            messages.push(message);
            if !perceived {
                break;
            }
        }
    }
    messages
}

/// Each message as hex, the form the fixtures hold.
fn hex(messages: &[Vec<u8>]) -> Vec<String> {
    messages
        .iter()
        .map(|bytes| bytes.iter().map(|byte| format!("{byte:02x}")).collect())
        .collect()
}

/// The messages fixture `name` holds: a JSON array of hex strings.
fn fixture(name: &str) -> Vec<String> {
    let path = format!(
        "{}/../../web/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let json = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    json.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

// The fixtures do not hold the Hello: each token below is the one its
// recording used. To re-record after a deliberate change, paste the left side
// of the failing `assert_eq!` into the fixture file: it prints valid JSON.

#[test]
fn the_walk_fixture_is_what_the_server_sends() {
    const TOKEN: ResumeToken = ResumeToken(6);
    let address = start_server();
    let mut socket = connect(address, TOKEN);
    let mut messages = record(
        &mut socket,
        &[
            Action::Move(East),
            Action::Move(North),
            Action::Wait,
            Action::Move(SouthEast),
        ],
    );
    let _newer = connect(address, TOKEN);
    messages.push(receive(&mut socket));
    assert_eq!(hex(&messages), fixture("walk.json"));
}

#[test]
fn the_bump_fixture_is_what_the_server_sends() {
    const TOKEN: ResumeToken = ResumeToken(19);
    let address = start_server();
    let mut socket = connect(address, TOKEN);
    let messages = record(
        &mut socket,
        &[
            Action::Move(SouthEast),
            Action::Move(SouthEast),
            Action::Move(NorthWest),
        ],
    );
    assert_eq!(hex(&messages), fixture("bump.json"));
}

#[test]
fn the_attack_fixture_is_what_the_server_sends() {
    const TOKEN: ResumeToken = ResumeToken(19);
    let address = start_server();
    let mut socket = connect(address, TOKEN);
    let messages = record(
        &mut socket,
        &[
            Action::Move(SouthEast),
            Action::Move(SouthEast),
            Action::Attack(NorthWest),
            Action::Attack(West),
        ],
    );
    assert_eq!(hex(&messages), fixture("attack.json"));
}

#[test]
fn the_death_fixture_is_what_the_server_sends() {
    const TOKEN: ResumeToken = ResumeToken(15);
    let address = start_server();
    let mut socket = connect(address, TOKEN);
    let messages = record(
        &mut socket,
        &[
            Action::Move(East),
            Action::Move(SouthEast),
            Action::Attack(SouthEast),
            Action::Attack(SouthEast),
            Action::Move(SouthEast),
        ],
    );
    assert_eq!(hex(&messages), fixture("death.json"));
}
