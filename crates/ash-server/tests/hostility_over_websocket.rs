//! Acceptance of spec 0002, slice S2, over WebSocket: the snapshot carries
//! the disposition of each creature (A4). A move into the hostile creature
//! is perceived as a hit or a miss, a move into the neutral creature as a
//! bump (A1, A2).
//!
//! The client only moves. It chases the hostile creature until a move
//! strikes it, then the neutral creature until a move bumps it, planning
//! each command on a replay of the accepted commands.

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use ash_core::tile::TilePosition;
use ash_protocol::message::{
    Action, ChangeKind, ClientMessage, Disposition, Offer, Outcome, PerceivedChange,
    PerceivedEntity, ResumeToken, ServerMessage, Snapshot,
};
use ash_sim::world::World;
use tungstenite::{Message, WebSocket};

#[path = "websocket/action.rs"]
mod action;
#[path = "websocket/change.rs"]
mod change;
#[path = "websocket/disposition.rs"]
mod disposition;
#[path = "websocket/health.rs"]
mod health;

type Socket = WebSocket<TcpStream>;

const TOKEN: ResumeToken = ResumeToken(0x4057_11e0_0000_0002);
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
        .copied()
        .map(change::to_wire)
        .collect()
}

/// The entities of the snapshot of `world`, as the server must send them.
fn snapshot_entities(world: &World) -> Vec<PerceivedEntity> {
    ash_sim::perception::snapshot(world, world.awaited())
        .entities
        .iter()
        .map(|perceived| PerceivedEntity {
            entity: perceived.entity,
            position: perceived.position,
            disposition: perceived.disposition.map(disposition::to_wire),
            health: health::to_wire(perceived.health),
        })
        .collect()
}

/// The offered move that brings `from` closest to `target`: the move onto
/// its tile once adjacent. A wait when no move is offered.
fn chase(offers: &[Offer], from: TilePosition, target: TilePosition) -> Action {
    let distance = |position: TilePosition| {
        position
            .x
            .abs_diff(target.x)
            .max(position.y.abs_diff(target.y))
    };
    offers
        .iter()
        .filter_map(|offer| match offer.action {
            Action::Move(direction) => Some(direction),
            Action::Wait | Action::Attack(_) => None,
        })
        .min_by_key(|&direction| distance(from.step(direction)))
        .map_or(Action::Wait, Action::Move)
}

#[test]
fn the_client_perceives_dispositions_and_strikes_the_hostile_creature_by_moving_into_it() {
    let address = start_server();
    let mut replay = World::skeleton(ash_server::world_seed(TOKEN));
    let character = replay.awaited();
    let mut socket = connect(address, TOKEN);

    // A4, R16: the snapshot gives the character no disposition, then the
    // hostile creature and the neutral creature theirs.
    let entities = match receive(&mut socket) {
        Some(ServerMessage::Snapshot(Snapshot { entities, .. })) => entities,
        other => panic!("expected a snapshot, got {other:?}"),
    };
    assert_eq!(entities, snapshot_entities(&replay));
    let (hostile, neutral) = match entities.as_slice() {
        [first, hostile, neutral] => {
            assert_eq!(
                (first.entity, first.disposition),
                (character, None),
                "{first:?}"
            );
            assert_eq!(hostile.disposition, Some(Disposition::Hostile));
            assert_eq!(neutral.disposition, Some(Disposition::Neutral));
            (hostile.entity, neutral.entity)
        }
        other => panic!("expected the character and two creatures: {other:?}"),
    };

    let (mut struck, mut bumped) = (false, false);
    let mut next = receive(&mut socket);
    for _ in 0..MOST_COMMANDS {
        if struck && bumped {
            return;
        }
        let offers = match next {
            Some(ServerMessage::Awaited {
                time,
                entity,
                offers,
            }) => {
                assert_eq!((time, entity), (replay.clock(), character));
                offers
            }
            other => panic!("expected the character's turn, got {other:?}"),
        };
        // R9, R14: the turn offers what the simulation offers, at its costs.
        let parts: Vec<(Action, u64)> = offers
            .iter()
            .map(|offer| (offer.action, offer.cost.units()))
            .collect();
        let expected: Vec<(Action, u64)> = replay
            .offers()
            .into_iter()
            .map(|offer| (action::to_wire(offer.action), offer.cost.units()))
            .collect();
        assert_eq!(parts, expected);
        let from = replay.position(character);
        let target = if struck { neutral } else { hostile };
        let action = chase(&offers, from, replay.position(target));
        // Read from the snapshot: on the way to the neutral creature, a
        // move may strike the hostile one again, and kill it (A8).
        let ahead = match action {
            Action::Move(direction) => snapshot_entities(&replay)
                .iter()
                .find(|perceived| perceived.position == from.step(direction))
                .map(|perceived| perceived.entity),
            Action::Wait | Action::Attack(_) => None,
        };
        send(
            &mut socket,
            ClientMessage::Command {
                entity: character,
                action,
            },
        );
        let (sent_at, seen) = (replay.clock(), replay.perceived_changes().len());
        assert_eq!(replay.submit(action::to_sim(action)), Ok(()));
        // R17: the changes since the command, in the order they happened.
        let (changes, then) = receive_turn(&mut socket);
        assert_eq!(changes, changes_since(&replay, seen));

        let did = changes.first().and_then(|change| match change.kind {
            ChangeKind::Act(outcome) => Some((change.entity, outcome)),
            ChangeKind::Death | ChangeKind::Condition(_) | ChangeKind::Disposition(_) => None,
        });
        if ahead == Some(hostile) {
            // A1, A5: the move is a hit or a miss of the hostile creature,
            // and the next turn comes 100 later.
            assert!(
                did == Some((character, Outcome::Hit(hostile)))
                    || did == Some((character, Outcome::Missed(hostile))),
                "{action:?} into the hostile creature gave {did:?}"
            );
            match &then {
                Some(ServerMessage::Awaited { time, .. }) => {
                    assert_eq!(time.units() - sent_at.units(), 100);
                }
                other => panic!("expected the character's turn, got {other:?}"),
            }
            struck = true;
        } else if ahead == Some(neutral) {
            // A2: the move into the neutral creature is a bump.
            assert_eq!(did, Some((character, Outcome::Bumped(neutral))));
            bumped = true;
        }
        next = then;
    }
    panic!("no strike and bump by a move in {MOST_COMMANDS} commands");
}
