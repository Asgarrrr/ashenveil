//! Acceptance of spec 0002, slice S1, over WebSocket: the character strikes
//! the creature on purpose, and the client perceives each strike as a hit or
//! a miss, never with its odds (A11, A12).
//!
//! The client attacks whenever an attack is offered and chases the creature
//! otherwise, planning each command on a replay of the accepted commands.

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use ash_core::entity::EntityId;
use ash_core::tile::{Direction, TilePosition};
use ash_protocol::message::{
    Action, ChangeKind, ClientMessage, Offer, Outcome, PerceivedChange, ResumeToken, ServerMessage,
};
use ash_sim::world::World;
use tungstenite::{Message, WebSocket};

#[path = "websocket/action.rs"]
mod action;
#[path = "websocket/change.rs"]
mod change;

type Socket = WebSocket<TcpStream>;

const TOKEN: ResumeToken = ResumeToken(0xa77a_c4ed_0000_0001);
/// The fight gives up after this many commands.
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

/// Where the characters of `world` perceive `entity`; `None` once it died
/// and left the world (A8).
fn standing(world: &World, entity: EntityId) -> Option<TilePosition> {
    ash_sim::perception::snapshot(world, world.awaited())
        .entities
        .iter()
        .find(|perceived| perceived.entity == entity)
        .map(|perceived| perceived.position)
}

/// The offered attack toward `target`, if any; otherwise the offered move
/// that brings `from` closest to `target`.
fn strike_or_chase(offers: &[Offer], from: TilePosition, target: TilePosition) -> Action {
    let distance = |position: TilePosition| {
        position
            .x
            .abs_diff(target.x)
            .max(position.y.abs_diff(target.y))
    };
    let actions = offers.iter().map(|offer| offer.action);
    actions
        .clone()
        .find(
            |action| matches!(action, Action::Attack(direction) if from.step(*direction) == target),
        )
        .or_else(|| {
            actions.min_by_key(|action| match action {
                Action::Move(direction) => distance(from.step(*direction)),
                Action::Wait | Action::Attack(_) => distance(from),
            })
        })
        .unwrap_or(Action::Wait)
}

/// A12, R18: everything a change says besides its time: the entity it
/// names and, for an act, the outcome. The patterns name every field, so a
/// new one, such as odds, fails to compile here.
fn change_parts(change: &PerceivedChange) -> (EntityId, Option<Outcome>) {
    let PerceivedChange {
        time: _,
        entity,
        kind,
    } = *change;
    let outcome = match kind {
        ChangeKind::Act(outcome) => Some(outcome),
        ChangeKind::Death => None,
        // A condition is a kind, never a number (A10).
        ChangeKind::Condition(_) => None,
        // A disposition is a kind, never a number (A4).
        ChangeKind::Disposition(_) => None,
        ChangeKind::Arrival(_) => None,
        // A9: hit points reach their own character alone.
        ChangeKind::HitPoints { .. } => None,
    };
    (entity, outcome)
}

/// A12: everything an offer says. The pattern names every field, so odds
/// beside the cost fail to compile here.
fn offer_parts(offer: &Offer) -> (Action, u64) {
    let Offer { action, cost } = *offer;
    (action, cost.units())
}

#[test]
fn the_client_perceives_its_strikes_as_hits_and_misses() {
    let address = start_server();
    let mut replay = World::skeleton(ash_server::world_seed(TOKEN));
    let character = replay.awaited();
    let creature = EntityId::new(1);
    let mut socket = connect(address, TOKEN);
    assert!(matches!(
        receive(&mut socket),
        Some(ServerMessage::Snapshot(_))
    ));
    let mut next = receive(&mut socket);

    // R10, A3: an attack toward the wall north of the start is refused.
    send(
        &mut socket,
        ClientMessage::Command {
            entity: character,
            action: Action::Attack(Direction::North),
        },
    );
    assert_eq!(receive(&mut socket), Some(ServerMessage::Refused));

    let mut received: Vec<PerceivedChange> = Vec::new();
    for _ in 0..MOST_COMMANDS {
        let strikes: Vec<Outcome> = received
            .iter()
            .map(change_parts)
            .filter(|&(entity, _)| entity == character)
            .filter_map(|(_, outcome)| outcome)
            .filter(|outcome| matches!(outcome, Outcome::Hit(_) | Outcome::Missed(_)))
            .collect();
        if strikes.contains(&Outcome::Hit(creature)) && strikes.contains(&Outcome::Missed(creature))
        {
            // A11: every strike names the creature.
            assert!(
                strikes
                    .iter()
                    .all(|&strike| strike == Outcome::Hit(creature)
                        || strike == Outcome::Missed(creature)),
                "{strikes:?}"
            );
            return;
        }

        // R9, R14: the turn is the character's, offering what the
        // simulation offers, an attack for 100 next to the creature (A5).
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
        let parts: Vec<(Action, u64)> = offers.iter().map(offer_parts).collect();
        let expected: Vec<(Action, u64)> = replay
            .offers()
            .into_iter()
            .map(|offer| (action::to_wire(offer.action), offer.cost.units()))
            .collect();
        assert_eq!(parts, expected);

        // The creature dies at its second hit (A8). A world whose second
        // hit comes before its first miss cannot show a miss: the test fails
        // on it, by name.
        let Some(target) = standing(&replay, creature) else {
            panic!("the creature died before the client saw a miss: {received:?}");
        };
        let action = strike_or_chase(&offers, replay.position(character), target);
        send(
            &mut socket,
            ClientMessage::Command {
                entity: character,
                action,
            },
        );
        let seen = replay.perceived_changes().len();
        assert_eq!(replay.submit(action::to_sim(action)), Ok(()));
        // R17: the changes since the command carry their outcomes.
        let (changes, then) = receive_turn(&mut socket);
        assert_eq!(changes, changes_since(&replay, seen));
        received.extend(changes);
        next = then;
    }
    panic!("no hit and miss in {MOST_COMMANDS} commands: {received:?}");
}
