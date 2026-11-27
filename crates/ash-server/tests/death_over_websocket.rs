//! Acceptance of spec 0002, slice S3, over WebSocket: the character kills
//! the hostile creature. The client perceives the lethal hit, then the death
//! (A11), and no later message names the dead creature (A8).
//!
//! The client strikes the hostile creature when an attack toward it is
//! offered and chases it otherwise. Once it died, the client chases the
//! neutral creature. It plans each command on a replay of the accepted
//! commands.

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use ash_core::entity::EntityId;
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

type Socket = WebSocket<TcpStream>;

const TOKEN: ResumeToken = ResumeToken(0xdea7_4000_0000_0003);
/// The hunt gives up after this many commands.
const MOST_COMMANDS: usize = 300;
/// The commands the client sends once the creature died.
const COMMANDS_AFTER: usize = 40;

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

/// Where the characters of `world` perceive `entity`; `None` once it died
/// and left the world (A8).
fn standing(world: &World, entity: EntityId) -> Option<TilePosition> {
    ash_sim::perception::snapshot(world, world.awaited())
        .entities
        .iter()
        .find(|perceived| perceived.entity == entity)
        .map(|perceived| perceived.position)
}

/// The offered attack toward `target`, if any.
fn strike(offers: &[Offer], from: TilePosition, target: TilePosition) -> Option<Action> {
    offers.iter().map(|offer| offer.action).find(
        |action| matches!(action, Action::Attack(direction) if from.step(*direction) == target),
    )
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

/// True when `change` names `entity`: it acts, is acted on, dies, or changes
/// condition. The patterns name every field, so a new one fails to compile
/// here.
fn change_names(change: &PerceivedChange, entity: EntityId) -> bool {
    let PerceivedChange {
        time: _,
        entity: named,
        kind,
    } = *change;
    named == entity
        || match kind {
            ChangeKind::Act(Outcome::Stepped(_) | Outcome::Waited)
            | ChangeKind::Death
            | ChangeKind::Condition(_) => false,
            ChangeKind::Act(
                Outcome::Bumped(target) | Outcome::Hit(target) | Outcome::Missed(target),
            ) => target == entity,
        }
}

/// True when `message` names `entity` anywhere. The patterns name every
/// field, so a new one fails to compile here.
fn names(message: &ServerMessage, entity: EntityId) -> bool {
    match message {
        ServerMessage::Snapshot(Snapshot {
            time: _,
            width: _,
            terrain: _,
            entities,
        }) => entities.iter().any(|perceived| {
            let PerceivedEntity {
                entity: perceived,
                position: _,
                disposition: _,
                health: _,
            } = *perceived;
            perceived == entity
        }),
        ServerMessage::Awaited {
            time: _,
            entity: awaited,
            offers,
        } => {
            // An offer names no entity: an action and its cost.
            for offer in offers {
                let Offer { action: _, cost: _ } = *offer;
            }
            *awaited == entity
        }
        ServerMessage::Perceived(change) => change_names(change, entity),
        ServerMessage::Refused | ServerMessage::Quarantined | ServerMessage::ControllerTaken => {
            false
        }
    }
}

/// The offers of the turn `message` is. It must be the turn of `character`
/// at the time of `replay`, offering what the simulation offers (R9, R14).
fn turn_of<'m>(
    message: &'m Option<ServerMessage>,
    replay: &World,
    character: EntityId,
) -> &'m [Offer] {
    let Some(ServerMessage::Awaited {
        time,
        entity,
        offers,
    }) = message
    else {
        panic!("expected the character's turn, got {message:?}");
    };
    assert_eq!((*time, *entity), (replay.clock(), character));
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
    offers
}

/// The client sends `action` for `character`. Returns the changes of the
/// turn, which are the changes of `replay` in their order (R17), then the
/// message after them.
fn command(
    socket: &mut Socket,
    replay: &mut World,
    character: EntityId,
    action: Action,
) -> (Vec<PerceivedChange>, Option<ServerMessage>) {
    send(
        socket,
        ClientMessage::Command {
            entity: character,
            action,
        },
    );
    let seen = replay.perceived_changes().len();
    assert_eq!(replay.submit(action::to_sim(action)), Ok(()));
    let (changes, then) = receive_turn(socket);
    assert_eq!(changes, changes_since(replay, seen));
    (changes, then)
}

#[test]
fn the_client_perceives_the_death_and_never_the_dead_creature_again() {
    let address = start_server();
    let mut replay = World::skeleton(ash_server::world_seed(TOKEN));
    let character = replay.awaited();
    let mut socket = connect(address, TOKEN, 0);

    let (hostile, neutral) = match receive(&mut socket) {
        Some(ServerMessage::Snapshot(Snapshot { entities, .. })) => match entities.as_slice() {
            [first, hostile, neutral] => {
                assert_eq!(first.entity, character);
                assert_eq!(hostile.disposition, Some(Disposition::Hostile));
                assert_eq!(neutral.disposition, Some(Disposition::Neutral));
                (hostile.entity, neutral.entity)
            }
            other => panic!("expected the character and two creatures: {other:?}"),
        },
        other => panic!("expected a snapshot, got {other:?}"),
    };

    // The hunt: the client strikes the hostile creature until it left the
    // replayed world.
    let mut received: Vec<PerceivedChange> = Vec::new();
    let mut next = receive(&mut socket);
    for _ in 0..MOST_COMMANDS {
        let Some(target) = standing(&replay, hostile) else {
            break;
        };
        let offers = turn_of(&next, &replay, character);
        let from = replay.position(character);
        let action = strike(offers, from, target).unwrap_or_else(|| chase(offers, from, target));
        let (changes, then) = command(&mut socket, &mut replay, character, action);
        received.extend(changes);
        next = then;
    }
    assert_eq!(
        standing(&replay, hostile),
        None,
        "no death in {MOST_COMMANDS} commands"
    );

    // A8, A11: the client received one death, of the hostile creature, right
    // after the hit that killed it and at the same time.
    let deaths: Vec<usize> = received
        .iter()
        .enumerate()
        .filter(|(_, change)| change.kind == ChangeKind::Death)
        .map(|(index, _)| index)
        .collect();
    let [died] = deaths[..] else {
        panic!("expected one death, got {deaths:?} in {received:?}");
    };
    let PerceivedChange { time, entity, .. } = received[died];
    assert_eq!(entity, hostile);
    assert_eq!(
        received[died - 1],
        PerceivedChange {
            time,
            entity: character,
            kind: ChangeKind::Act(Outcome::Hit(hostile)),
        }
    );

    // A8: no later message names the dead creature: no turn, no change.
    // The client chases the neutral creature meanwhile.
    for _ in 0..COMMANDS_AFTER {
        let turn = next.as_ref().expect("the character's turn");
        assert!(!names(turn, hostile), "{turn:?}");
        let target = standing(&replay, neutral).expect("the neutral creature is in the world");
        let action = chase(
            turn_of(&next, &replay, character),
            replay.position(character),
            target,
        );
        let (changes, then) = command(&mut socket, &mut replay, character, action);
        for change in &changes {
            assert!(!change_names(change, hostile), "{change:?}");
        }
        received.extend(changes);
        next = then;
    }

    // R17, R19: a later connection gets each change it missed once, in
    // order, then a snapshot without the dead creature and the pending turn.
    // The client left before the lethal hit, between it and the death, or
    // after the last change.
    drop(socket);
    for held in [died - 1, died, received.len()] {
        let mut socket = connect(address, TOKEN, held);
        let (missed, then) = receive_turn(&mut socket);
        assert_eq!(missed, received[held..], "{held} changes held");
        match then {
            Some(ServerMessage::Snapshot(Snapshot { entities, .. })) => {
                let perceived: Vec<EntityId> =
                    entities.iter().map(|entity| entity.entity).collect();
                assert_eq!(perceived, [character, neutral]);
            }
            other => panic!("expected a snapshot, got {other:?}"),
        }
        assert_eq!(receive(&mut socket), next);
    }
}
