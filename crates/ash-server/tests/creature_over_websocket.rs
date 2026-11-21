//! Acceptance of spec 0001, slice S7, over WebSocket: the client perceives
//! the creature only through positions and perceived actions (R18).
//!
//! The client chases the creature until a bump, planning each command on a
//! replay of the accepted commands.

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use ash_core::entity::EntityId;
use ash_core::tile::TilePosition;
use ash_core::time::Time;
use ash_protocol::message::{
    Action, AvailableAction, ClientMessage, PerceivedChange, PerceivedEntity, ResumeToken,
    ServerMessage, Snapshot,
};
use ash_sim::world::World;
use tungstenite::{Message, WebSocket};

#[path = "websocket/action.rs"]
mod action;

type Socket = WebSocket<TcpStream>;

const TOKEN: ResumeToken = ResumeToken(0xc0ff_ee00_5eed_0007);
/// The chase gives up after this many commands.
const MOST_COMMANDS: usize = 200;

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

/// The changes of `world` after the first `seen`, as the server must send
/// them.
fn changes_since(world: &World, seen: usize) -> Vec<PerceivedChange> {
    world.perceived_changes()[seen..]
        .iter()
        .map(|change| PerceivedChange {
            time: change.time,
            entity: change.entity,
            action: action::to_wire(change.action),
            bumped: change.bumped,
        })
        .collect()
}

/// The offered action that brings `character` closest to `target`: a move
/// onto it once adjacent, which bumps.
fn chase(offered: &[AvailableAction], from: TilePosition, target: TilePosition) -> Action {
    let distance = |position: TilePosition| {
        position
            .x
            .abs_diff(target.x)
            .max(position.y.abs_diff(target.y))
    };
    offered
        .iter()
        .map(|offer| offer.action)
        .min_by_key(|action| match action {
            Action::Move(direction) => distance(from.step(*direction)),
            Action::Wait => distance(from),
        })
        .unwrap_or(Action::Wait)
}

/// R18: everything a snapshot says of an entity. The pattern names every
/// field, so a new one fails to compile here.
fn entity_parts(perceived: &PerceivedEntity) -> (EntityId, TilePosition) {
    let PerceivedEntity { entity, position } = *perceived;
    (entity, position)
}

/// R18: everything a change says of an entity besides its time. The pattern
/// names every field, so a new one fails to compile here.
fn change_parts(change: &PerceivedChange) -> (EntityId, Action, Option<EntityId>) {
    let PerceivedChange {
        time: _,
        entity,
        action,
        bumped,
    } = *change;
    (entity, action, bumped)
}

#[test]
fn the_client_perceives_the_creature_only_through_positions_and_actions() {
    let address = start_server();
    let mut replay = ash_server::create_world(ash_server::world_seed(TOKEN));
    let character = replay.awaited();
    let mut socket = connect(address, TOKEN);

    // R16, R18: the snapshot gives each entity an id and a position.
    let entities = match receive(&mut socket) {
        Some(ServerMessage::Snapshot(Snapshot { time, entities, .. })) => {
            assert_eq!(time, Time::ZERO);
            entities.iter().map(entity_parts).collect::<Vec<_>>()
        }
        other => panic!("expected a snapshot, got {other:?}"),
    };
    let expected: Vec<_> = replay
        .entities()
        .map(|entity| (entity, replay.position(entity)))
        .collect();
    assert_eq!(entities, expected);
    let creature = match entities.as_slice() {
        [(first, _), (creature, _)] if *first == character => *creature,
        other => panic!("expected the character, then the creature: {other:?}"),
    };

    let mut received = Vec::new();
    let mut next = receive(&mut socket);
    for _ in 0..MOST_COMMANDS {
        // R9, R18: every turn the client learns is the character's, at the
        // character's own costs.
        let actions = match next {
            Some(ServerMessage::Awaited {
                time,
                entity,
                actions,
            }) => {
                assert_eq!((time, entity), (replay.clock(), character));
                actions
            }
            other => panic!("expected the character's turn, got {other:?}"),
        };
        for offer in &actions {
            let cost = match offer.action {
                Action::Move(_) => 100,
                Action::Wait => 50,
            };
            assert_eq!(offer.cost.units(), cost, "{offer:?}");
        }
        if received
            .iter()
            .any(|change: &PerceivedChange| change.bumped.is_some())
        {
            break;
        }

        let action = chase(
            &actions,
            replay.position(character),
            replay.position(creature),
        );
        send(
            &mut socket,
            ClientMessage::Command {
                entity: character,
                action,
            },
        );
        let seen = replay.perceived_changes().len();
        assert_eq!(replay.submit(action::to_sim(action)), Ok(()));
        // R17: the changes since the command, in the order they happened.
        let (changes, then) = receive_turn(&mut socket);
        assert_eq!(changes, changes_since(&replay, seen));
        received.extend(changes);
        next = then;
    }

    // R13: the client perceived a bump between the two entities, and the
    // mover stayed in place.
    let bump = received
        .iter()
        .position(|change| change.bumped.is_some())
        .expect("the chase never bumped");
    let (mover, _, bumped) = change_parts(&received[bump]);
    let pair = [Some(mover), bumped];
    assert!(
        pair == [Some(character), Some(creature)] || pair == [Some(creature), Some(character)],
        "{:?}",
        received[bump]
    );

    // R6: the creature's changes decode to actions only, yet their times
    // show it moves for 50 and waits for 25.
    let creature_changes: Vec<_> = received
        .iter()
        .filter(|change| change.entity == creature)
        .collect();
    assert!(creature_changes.len() > 2, "{creature_changes:?}");
    for pair in creature_changes.windows(2) {
        let (_, action, _) = change_parts(pair[0]);
        let paid = pair[1].time.units() - pair[0].time.units();
        let cost = match action {
            Action::Move(_) => 50,
            Action::Wait => 25,
        };
        assert_eq!(paid, cost, "after {:?}", pair[0]);
    }
}
