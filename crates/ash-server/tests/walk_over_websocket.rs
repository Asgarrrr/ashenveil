//! Acceptance of spec 0001, slice S4: the character walks over WebSocket.
//!
//! The fixed map, with x to the east and y to the south; the character
//! starts at (1, 1):
//!
//! ```text
//!   012345678
//! 0 #########
//! 1 #.......#
//! 2 #....#..#
//! 3 #.....#.#
//! 4 #.......#
//! 5 #########
//! ```

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use ash_core::entity::EntityId;
use ash_core::tile::{Direction, TilePosition};
use ash_core::time::{ActionCost, Time};
use ash_protocol::message::{
    Action, AvailableAction, ClientMessage, Outcome, PerceivedChange, PerceivedEntity, ResumeToken,
    ServerMessage, Snapshot,
};
use ash_sim::world::World;
use tungstenite::{Message, WebSocket};

use Direction::{East, North, South, SouthEast};

#[path = "websocket/action.rs"]
mod action;
#[path = "websocket/change.rs"]
mod change;
#[path = "websocket/disposition.rs"]
mod disposition;

type Socket = WebSocket<TcpStream>;

const TOKEN: ResumeToken = ResumeToken(0x0123_4567_89ab_cdef);

fn start_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a free local port");
    let address = listener.local_addr().expect("a bound address");
    thread::spawn(move || ash_server::serve(listener));
    address
}

/// Connects with `token`, having received `received` perceived changes of
/// its world.
fn connect(address: SocketAddr, token: ResumeToken, received: u64) -> Socket {
    let stream = TcpStream::connect(address).expect("the server accepts");
    // A missing reply fails the test instead of hanging it.
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("a read timeout");
    let (mut socket, _) =
        tungstenite::client(format!("ws://{address}/"), stream).expect("a WebSocket handshake");
    send(&mut socket, ClientMessage::Hello { token, received });
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

fn command(entity: EntityId, action: Action) -> ClientMessage {
    ClientMessage::Command { entity, action }
}

fn at(x: u32, y: u32) -> TilePosition {
    TilePosition { x, y, z: 0 }
}

fn offer(action: Action, cost: u64) -> AvailableAction {
    AvailableAction {
        action,
        cost: ActionCost::new(cost),
    }
}

/// Every perceived change of `world`, as the server sends it.
fn wire_changes(world: &World) -> Vec<PerceivedChange> {
    world
        .perceived_changes()
        .iter()
        .copied()
        .filter_map(change::to_wire)
        .collect()
}

/// What the server must send for `world`, built from the simulation alone.
fn expected_snapshot(world: &World) -> ServerMessage {
    let snapshot = ash_sim::perception::snapshot(world);
    ServerMessage::Snapshot(Snapshot {
        time: snapshot.time,
        width: snapshot.width,
        terrain: snapshot.terrain,
        entities: snapshot
            .entities
            .iter()
            .map(|perceived| PerceivedEntity {
                entity: perceived.entity,
                position: perceived.position,
                disposition: perceived.disposition.map(disposition::to_wire),
            })
            .collect(),
    })
}

fn expected_awaited(world: &World) -> ServerMessage {
    ServerMessage::Awaited {
        time: world.clock(),
        entity: world.awaited(),
        actions: world
            .available_actions()
            .into_iter()
            .map(|offered| AvailableAction {
                action: action::to_wire(offered.action),
                cost: offered.cost,
            })
            .collect(),
    }
}

/// The time of a snapshot, and the entry of `entity` in it.
fn snapshot_parts(
    message: &Option<ServerMessage>,
    entity: EntityId,
) -> (Time, Option<PerceivedEntity>) {
    match message {
        Some(ServerMessage::Snapshot(snapshot)) => (
            snapshot.time,
            snapshot
                .entities
                .iter()
                .find(|perceived| perceived.entity == entity)
                .copied(),
        ),
        other => panic!("expected a snapshot, got {other:?}"),
    }
}

#[test]
fn a_client_walks_disconnects_and_finds_its_world_unchanged() {
    let address = start_server();
    let start = World::skeleton(ash_server::world_seed(TOKEN));
    let character = start.awaited();
    let mut socket = connect(address, TOKEN, 0);

    // R16: the snapshot holds the whole map and every entity, stamped at
    // time zero.
    let snapshot = receive(&mut socket);
    assert_eq!(snapshot, Some(expected_snapshot(&start)));
    assert_eq!(
        snapshot_parts(&snapshot, character),
        (
            Time::ZERO,
            Some(PerceivedEntity {
                entity: character,
                position: at(1, 1),
                disposition: None,
            })
        )
    );

    // R9: the awaited character's controller learns each action and its
    // cost before choosing; the client hardcodes none.
    assert_eq!(
        receive(&mut socket),
        Some(ServerMessage::Awaited {
            time: Time::ZERO,
            entity: character,
            actions: vec![
                offer(Action::Wait, 50),
                offer(Action::Move(East), 100),
                offer(Action::Move(SouthEast), 100),
                offer(Action::Move(South), 100),
            ],
        })
    );

    // R17: each accepted command comes back as a perceived change stamped
    // with its time, then the changes of the creature, then the next turn.
    let mut received_changes = Vec::new();
    let mut accepted = Vec::new();
    let mut last_awaited = None;
    let mut accept =
        |socket: &mut Socket, action: Action, outcome: Outcome, time: u64, then: u64| {
            send(socket, command(character, action));
            let change = PerceivedChange {
                time: Time::new(time),
                entity: character,
                outcome,
            };
            let (changes, awaited) = receive_turn(socket);
            assert_eq!(changes.first(), Some(&change));
            match &awaited {
                Some(ServerMessage::Awaited { time, entity, .. }) => {
                    assert_eq!((*time, *entity), (Time::new(then), character));
                }
                other => panic!("expected the next turn, got {other:?}"),
            }
            received_changes.extend(changes);
            accepted.push(action);
            last_awaited = awaited;
        };
    accept(
        &mut socket,
        Action::Move(East),
        Outcome::Stepped(East),
        0,
        100,
    );
    accept(
        &mut socket,
        Action::Move(East),
        Outcome::Stepped(East),
        100,
        200,
    );

    // R10: a move into a wall is refused and advances nothing. So is a
    // command for an entity this controller does not drive.
    send(&mut socket, command(character, Action::Move(North)));
    assert_eq!(receive(&mut socket), Some(ServerMessage::Refused));
    send(&mut socket, command(EntityId::new(99), Action::Wait));
    assert_eq!(receive(&mut socket), Some(ServerMessage::Refused));

    // The wait is stamped 200: the refusals moved no clock.
    accept(&mut socket, Action::Wait, Outcome::Waited, 200, 250);

    // R19: the controller leaves while its character is awaited.
    drop(socket);
    let received = u64::try_from(received_changes.len()).expect("a small count");
    let mut socket = connect(address, TOKEN, received);

    let snapshot = receive(&mut socket);
    assert_eq!(
        snapshot_parts(&snapshot, character),
        (
            Time::new(250),
            Some(PerceivedEntity {
                entity: character,
                position: at(3, 1),
                disposition: None,
            })
        )
    );
    let pending_turn = receive(&mut socket);
    assert_eq!(pending_turn, last_awaited);

    // The hosted world equals a replay of the accepted commands.
    let mut replay = World::skeleton(ash_server::world_seed(TOKEN));
    for &action in &accepted {
        assert_eq!(replay.submit(action::to_sim(action)), Ok(()));
    }
    assert_eq!(snapshot, Some(expected_snapshot(&replay)));
    assert_eq!(pending_turn, Some(expected_awaited(&replay)));
    assert_eq!(received_changes, wire_changes(&replay));

    // The stream goes on from the pending turn: no change is sent twice.
    send(&mut socket, command(character, Action::Wait));
    let (changes, awaited) = receive_turn(&mut socket);
    assert_eq!(
        changes.first(),
        Some(&PerceivedChange {
            time: Time::new(250),
            entity: character,
            outcome: Outcome::Waited,
        })
    );
    match awaited {
        Some(ServerMessage::Awaited { time, entity, .. }) => {
            assert_eq!((time, entity), (Time::new(300), character));
        }
        other => panic!("expected the next turn, got {other:?}"),
    }
}

#[test]
fn a_turn_the_client_never_read_reaches_it_on_reconnect() {
    const LOST: ResumeToken = ResumeToken(0x1057);
    let address = start_server();
    let mut replay = World::skeleton(ash_server::world_seed(LOST));
    let character = replay.awaited();
    let mut socket = connect(address, LOST, 0);
    assert_eq!(receive(&mut socket), Some(expected_snapshot(&replay)));
    assert_eq!(receive(&mut socket), Some(expected_awaited(&replay)));

    send(&mut socket, command(character, Action::Move(East)));
    // The server sent its reply, so the turn is committed; the client
    // leaves without reading it.
    socket
        .get_ref()
        .peek(&mut [0])
        .expect("the reply reaches the client");
    drop(socket);
    assert_eq!(replay.submit(action::to_sim(Action::Move(East))), Ok(()));

    // R17: a client that received no change gets every one, in order, then
    // the present state and its turn.
    let mut socket = connect(address, LOST, 0);
    let (changes, snapshot) = receive_turn(&mut socket);
    assert_eq!(changes, wire_changes(&replay));
    assert_eq!(snapshot, Some(expected_snapshot(&replay)));
    assert_eq!(receive(&mut socket), Some(expected_awaited(&replay)));
}
