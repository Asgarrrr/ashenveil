//! Acceptance of spec 0002, slice S4, over WebSocket: the character sees the
//! fight. The client perceives its own hit points (A9) and the hostile
//! creature as wounded right after the first hit (A10, A11). No message
//! gives it the hit points of another entity (R18).
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
    Action, AvailableAction, ChangeKind, ClientMessage, Condition, Disposition, Outcome,
    PerceivedChange, PerceivedEntity, PerceivedHealth, ResumeToken, ServerMessage, Snapshot,
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

/// What a snapshot says of the hit points of each entity, in its order.
fn healths(message: &Option<ServerMessage>) -> Vec<(EntityId, PerceivedHealth)> {
    match message {
        Some(ServerMessage::Snapshot(Snapshot { entities, .. })) => entities
            .iter()
            .map(|perceived| (perceived.entity, perceived.health))
            .collect(),
        other => panic!("expected a snapshot, got {other:?}"),
    }
}

/// A10, R18: the entities `message` gives hit points to. The patterns name
/// every field of every message and every condition, so a new field, or a
/// condition that holds a number, fails to compile here.
fn given_hit_points(message: &ServerMessage) -> Vec<EntityId> {
    match message {
        ServerMessage::Snapshot(Snapshot {
            time: _,
            width: _,
            terrain: _,
            entities,
        }) => entities
            .iter()
            .filter_map(|perceived| {
                let PerceivedEntity {
                    entity,
                    position: TilePosition { x: _, y: _, z: _ },
                    disposition: None | Some(Disposition::Hostile | Disposition::Neutral),
                    health,
                } = *perceived;
                match health {
                    PerceivedHealth::HitPoints { current: _, max: _ } => Some(entity),
                    PerceivedHealth::Condition(
                        Condition::Unhurt
                        | Condition::Wounded
                        | Condition::BadlyWounded
                        | Condition::Dying,
                    ) => None,
                }
            })
            .collect(),
        ServerMessage::Awaited {
            time: _,
            entity: _,
            actions,
        } => {
            // R9: an offer holds an action and the cost of the awaited
            // character, never a hit point.
            for offer in actions {
                let AvailableAction { action: _, cost: _ } = *offer;
            }
            Vec::new()
        }
        ServerMessage::Perceived(PerceivedChange {
            time: _,
            entity: _,
            kind:
                ChangeKind::Act(
                    Outcome::Stepped(_)
                    | Outcome::Waited
                    | Outcome::Bumped(_)
                    | Outcome::Hit(_)
                    | Outcome::Missed(_),
                )
                | ChangeKind::Death
                | ChangeKind::Condition(
                    Condition::Unhurt
                    | Condition::Wounded
                    | Condition::BadlyWounded
                    | Condition::Dying,
                ),
        })
        | ServerMessage::Refused
        | ServerMessage::Quarantined
        | ServerMessage::ControllerTaken => Vec::new(),
    }
}

#[test]
fn the_client_perceives_its_own_hit_points_and_the_condition_of_the_creature_it_hits() {
    let address = start_server();
    let mut replay = World::skeleton(ash_server::world_seed(TOKEN));
    let character = replay.awaited();
    let mut socket = connect(address, TOKEN, 0);
    // A9, A13: the hit points of the character, which nothing strikes.
    let own = PerceivedHealth::HitPoints {
        current: 10,
        max: 10,
    };
    let unhurt = PerceivedHealth::Condition(Condition::Unhurt);
    let wounded = PerceivedHealth::Condition(Condition::Wounded);

    // A9, A10: the snapshot gives the character its own hit points, and
    // each creature a condition: unhurt at full.
    let snapshot = receive(&mut socket);
    let (hostile, neutral) = match healths(&snapshot)[..] {
        [(first, _), (hostile, _), (neutral, _)] if first == character => (hostile, neutral),
        ref other => panic!("expected the character and two creatures: {other:?}"),
    };
    assert_eq!(
        healths(&snapshot),
        [(character, own), (hostile, unhurt), (neutral, unhurt)]
    );
    // Every message the server sends.
    let mut sent: Vec<ServerMessage> = snapshot.into_iter().collect();

    // The hunt: the client strikes the hostile creature until a strike
    // hits. Every change received is the change of the replayed world, in
    // its order (R17).
    let mut received: Vec<PerceivedChange> = Vec::new();
    let mut next = receive(&mut socket);
    let mut hit = None;
    for _ in 0..MOST_COMMANDS {
        sent.extend(next.clone());
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
        if (received[seen].entity, received[seen].kind)
            == (character, ChangeKind::Act(Outcome::Hit(hostile)))
        {
            hit = Some(seen);
            break;
        }
    }
    let hit = hit.unwrap_or_else(|| panic!("no hit in {MOST_COMMANDS} commands"));
    sent.extend(received.iter().copied().map(ServerMessage::Perceived));
    sent.extend(next.clone());

    // A10, A11: the client received one change of condition: the hostile
    // creature is wounded, right after the hit and at the same time.
    let time = received[hit].time;
    let conditions: Vec<(usize, PerceivedChange)> = received
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, change)| matches!(change.kind, ChangeKind::Condition(_)))
        .collect();
    assert_eq!(
        conditions,
        [(
            hit + 1,
            PerceivedChange {
                time,
                entity: hostile,
                kind: ChangeKind::Condition(Condition::Wounded),
            }
        )]
    );

    // R17, R19: a later connection gets each change it missed once, in
    // order, then a snapshot and the pending turn. The client left before
    // the hit, between it and the change of condition, or after the last
    // change. Each snapshot shows the hostile creature wounded (A10).
    drop(socket);
    for held in [hit, hit + 1, received.len()] {
        let mut socket = connect(address, TOKEN, held);
        let (missed, snapshot) = receive_turn(&mut socket);
        assert_eq!(missed, received[held..], "{held} changes held");
        assert_eq!(
            healths(&snapshot),
            [(character, own), (hostile, wounded), (neutral, unhurt)],
            "{held} changes held"
        );
        sent.extend(snapshot);
        assert_eq!(receive(&mut socket), next);
    }

    // A10, R18: no message gives hit points to another entity. Only a
    // snapshot gives any, to the character it is for.
    for message in &sent {
        let given: &[EntityId] = match message {
            ServerMessage::Snapshot(_) => &[character],
            _ => &[],
        };
        assert_eq!(given_hit_points(message), given, "{message:?}");
    }
}
