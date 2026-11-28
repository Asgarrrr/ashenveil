//! Acceptance of spec 0001, slice S7, over WebSocket: the client perceives
//! a creature only through its position, its disposition, its condition and
//! perceived changes (R18, A10).
//!
//! The client chases the neutral creature until a bump, planning each
//! command on a replay of the accepted commands.

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use ash_core::entity::EntityId;
use ash_core::tile::TilePosition;
use ash_core::time::Time;
use ash_protocol::message::{
    Action, ChangeKind, ClientMessage, Disposition, Offer, Outcome, PerceivedChange,
    PerceivedEntity, PerceivedHealth, ResumeToken, ServerMessage, Snapshot,
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
        .copied()
        .map(change::to_wire)
        .collect()
}

/// The offered action that brings `character` closest to `target`: a move
/// onto it once adjacent, which bumps.
fn chase(offers: &[Offer], from: TilePosition, target: TilePosition) -> Action {
    let distance = |position: TilePosition| {
        position
            .x
            .abs_diff(target.x)
            .max(position.y.abs_diff(target.y))
    };
    offers
        .iter()
        .map(|offer| offer.action)
        .min_by_key(|action| match action {
            Action::Move(direction) => distance(from.step(*direction)),
            Action::Wait | Action::Attack(_) => distance(from),
        })
        .unwrap_or(Action::Wait)
}

/// One entity of a snapshot: who, where, its disposition, and its hit points
/// or its condition.
type EntityParts = (EntityId, TilePosition, Option<Disposition>, PerceivedHealth);

/// R18: everything a snapshot says of an entity: a disposition is a kind,
/// never a number. The pattern names every field, so a new one fails to
/// compile here.
fn entity_parts(perceived: &PerceivedEntity) -> EntityParts {
    let PerceivedEntity {
        entity,
        position,
        disposition,
        health,
    } = *perceived;
    (entity, position, disposition, health)
}

/// R18: everything a change says of an entity besides its time: the entity
/// it names and, for an act, the outcome. The patterns name every field, so
/// a new one fails to compile here.
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
    };
    (entity, outcome)
}

#[test]
fn the_client_perceives_the_creature_only_through_what_a_character_may_perceive() {
    let address = start_server();
    let mut replay = World::skeleton(ash_server::world_seed(TOKEN));
    let character = replay.awaited();
    let mut socket = connect(address, TOKEN);

    // R16, R18: the snapshot gives each entity an id, a position, a health
    // and, for a creature, a disposition.
    let entities = match receive(&mut socket) {
        Some(ServerMessage::Snapshot(Snapshot { time, entities, .. })) => {
            assert_eq!(time, Time::ZERO);
            entities.iter().map(entity_parts).collect::<Vec<_>>()
        }
        other => panic!("expected a snapshot, got {other:?}"),
    };
    let expected: Vec<_> = ash_sim::perception::snapshot(&replay, character)
        .entities
        .iter()
        .map(|perceived| {
            (
                perceived.entity,
                perceived.position,
                perceived.disposition.map(disposition::to_wire),
                health::to_wire(perceived.health),
            )
        })
        .collect();
    assert_eq!(entities, expected);
    // A move into the neutral creature is a bump (A2).
    let creature = match entities.as_slice() {
        [(first, ..), _, (creature, _, Some(Disposition::Neutral), _)] if *first == character => {
            *creature
        }
        other => panic!("expected the character, then two creatures, the last neutral: {other:?}"),
    };
    // R13: a bump between the character and the creature, either way.
    let bump_between_them = |change: &PerceivedChange| match change_parts(change) {
        (mover, Some(Outcome::Bumped(bumped))) => {
            [mover, bumped] == [character, creature] || [mover, bumped] == [creature, character]
        }
        _ => false,
    };

    let mut received = Vec::new();
    let mut next = receive(&mut socket);
    for _ in 0..MOST_COMMANDS {
        // R9, R18: every turn the client learns is the character's, at the
        // character's own costs.
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
        for offer in &offers {
            let cost = match offer.action {
                Action::Move(_) | Action::Attack(_) => 100,
                Action::Wait => 50,
            };
            assert_eq!(offer.cost.units(), cost, "{offer:?}");
        }
        // R14: the offers are the simulation's, attacks included.
        let offered: Vec<Action> = offers.iter().map(|offer| offer.action).collect();
        let expected: Vec<Action> = replay
            .offers()
            .into_iter()
            .map(|offer| action::to_wire(offer.action))
            .collect();
        assert_eq!(offered, expected);
        if received.iter().any(bump_between_them) {
            break;
        }

        let action = chase(
            &offers,
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

    // R13: the client perceived a bump between the two entities.
    assert!(
        received.iter().any(bump_between_them),
        "the chase never bumped"
    );

    // R6: the creature's changes decode to outcomes only, yet their times
    // show it moves for 50 and waits for 25.
    let creature_acts: Vec<(Time, Outcome)> = received
        .iter()
        .filter(|change| change.entity == creature)
        .filter_map(|change| match change.kind {
            ChangeKind::Act(outcome) => Some((change.time, outcome)),
            ChangeKind::Death
            | ChangeKind::Condition(_)
            | ChangeKind::Disposition(_)
            | ChangeKind::Arrival(_) => None,
        })
        .collect();
    assert!(creature_acts.len() > 2, "{creature_acts:?}");
    for pair in creature_acts.windows(2) {
        let paid = pair[1].0.units() - pair[0].0.units();
        let cost = match pair[0].1 {
            Outcome::Waited => 25,
            Outcome::Stepped(_) | Outcome::Bumped(_) | Outcome::Hit(_) | Outcome::Missed(_) => 50,
        };
        assert_eq!(paid, cost, "after {:?}", pair[0]);
    }
}
