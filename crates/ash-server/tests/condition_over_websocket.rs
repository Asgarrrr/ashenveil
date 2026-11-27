//! Acceptance of spec 0002, slice S4, over WebSocket: the character sees the
//! fight. The client perceives the hostile creature as wounded right after
//! the first hit (A10, A11).
//!
//! The client strikes the hostile creature when an attack toward it is
//! offered and chases it otherwise, until a strike hits. It plans each
//! command on a replay of the accepted commands.

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use ash_core::entity::EntityId;
use ash_core::tile::TilePosition;
use ash_protocol::message::{
    Action, AvailableAction, ClientMessage, Condition, Disposition, Outcome, PerceivedChange,
    ResumeToken, ServerMessage, Snapshot,
};
use ash_sim::world::World;
use tungstenite::{Message, WebSocket};

#[path = "websocket/action.rs"]
mod action;
#[path = "websocket/change.rs"]
mod change;

type Socket = WebSocket<TcpStream>;

const TOKEN: ResumeToken = ResumeToken(0xc0d1_7100_0000_0004);
/// The hunt gives up after this many commands.
const MOST_COMMANDS: usize = 300;

fn start_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a free local port");
    let address = listener.local_addr().expect("a bound address");
    thread::spawn(move || ash_server::serve(listener));
    address
}

/// Connects with `token`, having received `received` perceived changes of
/// its world.
fn connect(address: SocketAddr, token: ResumeToken, received: usize) -> Socket {
    let stream = TcpStream::connect(address).expect("the server accepts");
    // A missing reply fails the test instead of hanging it.
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("a read timeout");
    let (mut socket, _) =
        tungstenite::client(format!("ws://{address}/"), stream).expect("a WebSocket handshake");
    let received = u64::try_from(received).expect("a small count");
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

/// The changes of `world` after the first `seen`, as the server must send
/// them.
fn changes_since(world: &World, seen: usize) -> Vec<PerceivedChange> {
    world.perceived_changes()[seen..]
        .iter()
        .copied()
        .map(change::to_wire)
        .collect()
}

/// The offered attack toward `target`, if any; otherwise the offered move
/// that brings `from` closest to `target`. A wait when no move is offered.
fn strike_or_chase(
    offered: &[AvailableAction],
    from: TilePosition,
    target: TilePosition,
) -> Action {
    let distance = |position: TilePosition| {
        position
            .x
            .abs_diff(target.x)
            .max(position.y.abs_diff(target.y))
    };
    let toward = |direction| from.step(direction) == target;
    let actions = offered.iter().map(|offer| offer.action);
    actions
        .clone()
        .find(|action| matches!(action, Action::Attack(direction) if toward(*direction)))
        .unwrap_or_else(|| {
            actions
                .filter_map(|action| match action {
                    Action::Move(direction) => Some(direction),
                    Action::Wait | Action::Attack(_) => None,
                })
                .min_by_key(|&direction| distance(from.step(direction)))
                .map_or(Action::Wait, Action::Move)
        })
}

/// The offers of the turn `message` is. It must be the turn of `character`
/// at the time of `replay`, offering what the simulation offers (R9, R14).
fn turn_of<'m>(
    message: &'m Option<ServerMessage>,
    replay: &World,
    character: EntityId,
) -> &'m [AvailableAction] {
    let Some(ServerMessage::Awaited {
        time,
        entity,
        actions,
    }) = message
    else {
        panic!("expected the character's turn, got {message:?}");
    };
    assert_eq!((*time, *entity), (replay.clock(), character));
    let offers: Vec<(Action, u64)> = actions
        .iter()
        .map(|offer| (offer.action, offer.cost.units()))
        .collect();
    let expected: Vec<(Action, u64)> = replay
        .available_actions()
        .into_iter()
        .map(|offer| (action::to_wire(offer.action), offer.cost.units()))
        .collect();
    assert_eq!(offers, expected);
    actions
}

#[test]
fn the_client_perceives_the_creature_it_hit_as_wounded_right_after_the_hit() {
    let address = start_server();
    let mut replay = World::skeleton(ash_server::world_seed(TOKEN));
    let character = replay.awaited();
    let mut socket = connect(address, TOKEN, 0);

    let hostile = match receive(&mut socket) {
        Some(ServerMessage::Snapshot(Snapshot { entities, .. })) => match entities.as_slice() {
            [first, hostile, _] => {
                assert_eq!(first.entity, character);
                assert_eq!(hostile.disposition, Some(Disposition::Hostile));
                hostile.entity
            }
            other => panic!("expected the character and two creatures: {other:?}"),
        },
        other => panic!("expected a snapshot, got {other:?}"),
    };

    // The hunt: the client strikes the hostile creature until a strike
    // hits. Every change received is the change of the replayed world, in
    // its order (R17).
    let mut received: Vec<PerceivedChange> = Vec::new();
    let mut next = receive(&mut socket);
    let mut hit = None;
    for _ in 0..MOST_COMMANDS {
        let offered = turn_of(&next, &replay, character);
        let action = strike_or_chase(
            offered,
            replay.position(character),
            replay.position(hostile),
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
        let (changes, then) = receive_turn(&mut socket);
        assert_eq!(changes, changes_since(&replay, seen));
        received.extend(changes);
        next = then;
        if matches!(
            received[seen],
            PerceivedChange::Act { entity, outcome, .. }
                if (entity, outcome) == (character, Outcome::Hit(hostile))
        ) {
            hit = Some(seen);
            break;
        }
    }
    let hit = hit.unwrap_or_else(|| panic!("no hit in {MOST_COMMANDS} commands"));

    // A10, A11: the client received one change of condition: the hostile
    // creature is wounded, right after the hit and at the same time.
    let PerceivedChange::Act { time, .. } = received[hit] else {
        panic!("change {hit} is no act");
    };
    let conditions: Vec<(usize, PerceivedChange)> = received
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, change)| matches!(change, PerceivedChange::Condition { .. }))
        .collect();
    assert_eq!(
        conditions,
        [(
            hit + 1,
            PerceivedChange::Condition {
                time,
                entity: hostile,
                condition: Condition::Wounded,
            }
        )]
    );

    // R17, R19: a later connection gets each change it missed once, in
    // order, then a snapshot and the pending turn. The client left before
    // the hit, between it and the change of condition, or after the last
    // change.
    drop(socket);
    for held in [hit, hit + 1, received.len()] {
        let mut socket = connect(address, TOKEN, held);
        let (missed, then) = receive_turn(&mut socket);
        assert_eq!(missed, received[held..], "{held} changes held");
        assert!(
            matches!(then, Some(ServerMessage::Snapshot(_))),
            "expected a snapshot, got {then:?}"
        );
        assert_eq!(receive(&mut socket), next);
    }
}
