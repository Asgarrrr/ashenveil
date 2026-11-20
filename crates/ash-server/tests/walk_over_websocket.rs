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
    Action, AvailableAction, ClientMessage, PerceivedChange, PerceivedEntity, ResumeToken,
    ServerMessage, Snapshot, Terrain,
};
use ash_sim::map::Terrain as SimTerrain;
use ash_sim::world::World;
use tungstenite::{Message, WebSocket};

use Direction::{East, North, South, SouthEast};

type Socket = WebSocket<TcpStream>;

const TOKEN: ResumeToken = ResumeToken(0x0123_4567_89ab_cdef);

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
    send(&mut socket, ClientMessage::Hello { token });
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

fn wire_action(action: ash_sim::action::Action) -> Action {
    match action {
        ash_sim::action::Action::Move(direction) => Action::Move(direction),
        ash_sim::action::Action::Wait => Action::Wait,
    }
}

fn sim_action(action: Action) -> ash_sim::action::Action {
    match action {
        Action::Move(direction) => ash_sim::action::Action::Move(direction),
        Action::Wait => ash_sim::action::Action::Wait,
    }
}

/// What the server must send for `world`, built from the simulation alone.
fn expected_snapshot(world: &World) -> ServerMessage {
    ServerMessage::Snapshot(Snapshot {
        time: world.clock(),
        width: world.map().width(),
        terrain: world
            .map()
            .tiles()
            .iter()
            .map(|tile| match tile {
                SimTerrain::Floor => Terrain::Floor,
                SimTerrain::Wall => Terrain::Wall,
            })
            .collect(),
        entities: world
            .entities()
            .map(|entity| PerceivedEntity {
                entity,
                position: world.position(entity),
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
                action: wire_action(offered.action),
                cost: offered.cost,
            })
            .collect(),
    }
}

fn snapshot_parts(message: &Option<ServerMessage>) -> (Time, Vec<PerceivedEntity>) {
    match message {
        Some(ServerMessage::Snapshot(snapshot)) => (snapshot.time, snapshot.entities.clone()),
        other => panic!("expected a snapshot, got {other:?}"),
    }
}

#[test]
fn a_client_walks_disconnects_and_finds_its_world_unchanged() {
    let address = start_server();
    let start = ash_server::create_world();
    let character = start.awaited();
    let mut socket = connect(address, TOKEN);

    // R16: the snapshot holds the whole map and every entity, stamped at
    // time zero.
    let snapshot = receive(&mut socket);
    assert_eq!(snapshot, Some(expected_snapshot(&start)));
    assert_eq!(
        snapshot_parts(&snapshot),
        (
            Time::ZERO,
            vec![PerceivedEntity {
                entity: character,
                position: at(1, 1),
            }]
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
    // with its time, then the next turn.
    let mut received_changes = Vec::new();
    let mut accepted = Vec::new();
    let mut last_awaited = None;
    let mut accept = |socket: &mut Socket, action: Action, time: u64, then: u64| {
        send(socket, command(character, action));
        let change = PerceivedChange {
            time: Time::new(time),
            entity: character,
            action,
        };
        assert_eq!(receive(socket), Some(ServerMessage::Perceived(change)));
        let awaited = receive(socket);
        match &awaited {
            Some(ServerMessage::Awaited { time, entity, .. }) => {
                assert_eq!((*time, *entity), (Time::new(then), character));
            }
            other => panic!("expected the next turn, got {other:?}"),
        }
        received_changes.push(change);
        accepted.push(action);
        last_awaited = awaited;
    };
    accept(&mut socket, Action::Move(East), 0, 100);
    accept(&mut socket, Action::Move(East), 100, 200);

    // R10: a move into a wall is refused and advances nothing. So is a
    // command for an entity this controller does not drive.
    send(&mut socket, command(character, Action::Move(North)));
    assert_eq!(receive(&mut socket), Some(ServerMessage::Refused));
    send(&mut socket, command(EntityId::new(99), Action::Wait));
    assert_eq!(receive(&mut socket), Some(ServerMessage::Refused));

    // The wait is stamped 200: the refusals moved no clock.
    accept(&mut socket, Action::Wait, 200, 250);

    // R19: the controller leaves while its character is awaited.
    drop(socket);
    let mut socket = connect(address, TOKEN);

    let snapshot = receive(&mut socket);
    assert_eq!(
        snapshot_parts(&snapshot),
        (
            Time::new(250),
            vec![PerceivedEntity {
                entity: character,
                position: at(3, 1),
            }]
        )
    );
    let pending_turn = receive(&mut socket);
    assert_eq!(pending_turn, last_awaited);

    // The hosted world equals a replay of the accepted commands.
    let mut replay = ash_server::create_world();
    for &action in &accepted {
        assert_eq!(replay.submit(sim_action(action)), Ok(()));
    }
    assert_eq!(snapshot, Some(expected_snapshot(&replay)));
    assert_eq!(pending_turn, Some(expected_awaited(&replay)));
    let replayed_changes: Vec<_> = replay
        .perceived_changes()
        .iter()
        .map(|change| PerceivedChange {
            time: change.time,
            entity: change.entity,
            action: wire_action(change.action),
        })
        .collect();
    assert_eq!(received_changes, replayed_changes);

    // The stream goes on from the pending turn: no change is sent twice.
    send(&mut socket, command(character, Action::Wait));
    assert_eq!(
        receive(&mut socket),
        Some(ServerMessage::Perceived(PerceivedChange {
            time: Time::new(250),
            entity: character,
            action: Action::Wait,
        }))
    );
    match receive(&mut socket) {
        Some(ServerMessage::Awaited { time, entity, .. }) => {
            assert_eq!((time, entity), (Time::new(300), character));
        }
        other => panic!("expected the next turn, got {other:?}"),
    }
}
