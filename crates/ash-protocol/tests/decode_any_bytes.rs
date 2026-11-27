//! Spec 0001, slice S4: decoding never panics on any input.

use ash_core::entity::EntityId;
use ash_core::rng::Rng;
use ash_core::tile::{Direction, TilePosition};
use ash_core::time::{ActionCost, Time};
use ash_protocol::message::{
    Action, AvailableAction, ClientMessage, Disposition, Outcome, PerceivedChange, PerceivedEntity,
    ResumeToken, ServerMessage, Snapshot, Terrain,
};

const SAMPLES: usize = 200_000;

fn byte(rng: &mut Rng) -> u8 {
    rng.next_u64().to_le_bytes()[0]
}

fn below(rng: &mut Rng, bound: u64) -> usize {
    usize::try_from(rng.next_u64() % bound).expect("a small bound fits in usize")
}

fn decode_both(bytes: &[u8]) {
    let _ = ClientMessage::decode(bytes);
    let _ = ServerMessage::decode(bytes);
}

fn client_messages() -> Vec<ClientMessage> {
    vec![
        ClientMessage::Hello {
            token: ResumeToken(u128::MAX - 7),
            received: u64::MAX - 3,
        },
        ClientMessage::Command {
            entity: EntityId::new(3),
            action: Action::Move(Direction::NorthWest),
        },
        ClientMessage::Command {
            entity: EntityId::new(u32::MAX),
            action: Action::Wait,
        },
        ClientMessage::Command {
            entity: EntityId::new(0),
            action: Action::Attack(Direction::SouthWest),
        },
    ]
}

fn server_messages() -> Vec<ServerMessage> {
    vec![
        ServerMessage::Snapshot(Snapshot {
            time: Time::new(250),
            width: 3,
            terrain: vec![
                Terrain::Wall,
                Terrain::Floor,
                Terrain::Wall,
                Terrain::Floor,
                Terrain::Floor,
                Terrain::Wall,
            ],
            entities: vec![
                PerceivedEntity {
                    entity: EntityId::new(0),
                    position: TilePosition { x: 1, y: 1, z: 0 },
                    disposition: None,
                },
                PerceivedEntity {
                    entity: EntityId::new(1),
                    position: TilePosition {
                        x: u32::MAX,
                        y: 2,
                        z: 7,
                    },
                    disposition: Some(Disposition::Hostile),
                },
                PerceivedEntity {
                    entity: EntityId::new(2),
                    position: TilePosition { x: 1, y: 0, z: 0 },
                    disposition: Some(Disposition::Neutral),
                },
            ],
        }),
        ServerMessage::Awaited {
            time: Time::new(u64::MAX),
            entity: EntityId::new(0),
            actions: Direction::ALL
                .into_iter()
                .map(Action::Move)
                .chain([Action::Wait, Action::Attack(Direction::East)])
                .map(|action| AvailableAction {
                    action,
                    cost: ActionCost::new(100),
                })
                .collect(),
        },
        perceived(100, 2, Outcome::Stepped(Direction::South)),
        perceived(125, 2, Outcome::Waited),
        perceived(150, 1, Outcome::Bumped(EntityId::new(u32::MAX))),
        perceived(200, 0, Outcome::Hit(EntityId::new(1))),
        perceived(u64::MAX, 0, Outcome::Missed(EntityId::new(1))),
        ServerMessage::Refused,
        ServerMessage::Quarantined,
        ServerMessage::ControllerTaken,
    ]
}

fn perceived(time: u64, entity: u32, outcome: Outcome) -> ServerMessage {
    ServerMessage::Perceived(PerceivedChange {
        time: Time::new(time),
        entity: EntityId::new(entity),
        outcome,
    })
}

fn valid_encodings() -> Vec<Vec<u8>> {
    client_messages()
        .iter()
        .map(ClientMessage::encode)
        .chain(server_messages().iter().map(ServerMessage::encode))
        .collect()
}

#[test]
fn every_message_survives_a_round_trip() {
    for message in client_messages() {
        assert_eq!(ClientMessage::decode(&message.encode()), Ok(message));
    }
    for message in server_messages() {
        assert_eq!(ServerMessage::decode(&message.encode()), Ok(message));
    }
}

/// Every strict prefix of `bytes`, and `bytes` with one more byte.
fn truncated_or_extended(bytes: Vec<u8>) -> impl Iterator<Item = Vec<u8>> {
    let mut extended = bytes.clone();
    extended.push(0);
    (0..bytes.len())
        .map(move |length| bytes[..length].to_vec())
        .chain([extended])
}

#[test]
fn a_truncated_or_extended_message_is_refused() {
    for message in client_messages() {
        for bytes in truncated_or_extended(message.encode()) {
            assert!(ClientMessage::decode(&bytes).is_err(), "{bytes:?}");
        }
    }
    for message in server_messages() {
        for bytes in truncated_or_extended(message.encode()) {
            assert!(ServerMessage::decode(&bytes).is_err(), "{bytes:?}");
        }
    }
}

#[test]
fn random_bytes_never_panic_the_decoders() {
    let mut rng = Rng::new(0x5eed);
    for _ in 0..SAMPLES {
        // A small first byte reaches past the message tag most of the time.
        let mut bytes = vec![byte(&mut rng) % 6];
        let length = below(&mut rng, 96);
        bytes.extend((0..length).map(|_| byte(&mut rng)));
        decode_both(&bytes);
    }
}

#[test]
fn mutated_messages_never_panic_the_decoders() {
    let mut rng = Rng::new(0xf00d);
    let encodings = valid_encodings();
    for _ in 0..SAMPLES {
        let mut bytes = encodings[below(&mut rng, 1 << 16) % encodings.len()].clone();
        for _ in 0..=below(&mut rng, 4) {
            let index = below(&mut rng, 64) % bytes.len();
            bytes[index] = byte(&mut rng);
        }
        bytes.truncate(below(&mut rng, 128));
        decode_both(&bytes);
    }
}

/// The bytes of a perceived change at time 100 by entity 2, its outcome
/// already encoded in `outcome`.
fn perceived_bytes(outcome: &[u8]) -> Vec<u8> {
    let mut bytes = vec![2];
    bytes.extend(100_u64.to_le_bytes());
    bytes.extend(2_u32.to_le_bytes());
    bytes.extend(outcome);
    bytes
}

#[test]
fn a_change_the_simulation_cannot_make_is_refused() {
    let targeting = |tag: u8, entity: u32| [vec![tag], entity.to_le_bytes().to_vec()].concat();
    // The controls: a bump, a hit and a miss of another entity decode.
    for tag in [2, 3, 4] {
        assert!(ServerMessage::decode(&perceived_bytes(&targeting(tag, 1))).is_ok());
    }

    let refused = [
        ("a bump into its mover", perceived_bytes(&targeting(2, 2))),
        ("a hit on its attacker", perceived_bytes(&targeting(3, 2))),
        ("a miss of its attacker", perceived_bytes(&targeting(4, 2))),
        ("an outcome tag of 5", perceived_bytes(&[5])),
        (
            "an outcome tag of 5 with a target",
            perceived_bytes(&targeting(5, 1)),
        ),
        // Entity 2 acts: a missing target read as entity 0 is not its own
        // actor, so only the missing bytes refuse these.
        ("a bump with no target", perceived_bytes(&[2])),
        ("a hit with no target", perceived_bytes(&[3])),
        ("a miss with no target", perceived_bytes(&[4])),
        ("a step toward no direction", perceived_bytes(&[0, 8])),
    ];
    for (what, bytes) in refused {
        assert!(ServerMessage::decode(&bytes).is_err(), "{what} decodes");
    }
}

/// The bytes of a snapshot at time 0 of one floor tile, holding entity 1 at
/// (0, 0, 0), its disposition already encoded in `disposition`.
fn snapshot_bytes(disposition: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0];
    bytes.extend(0_u64.to_le_bytes());
    // A width of 1, then one tile: floor.
    bytes.extend(1_u32.to_le_bytes());
    bytes.extend(1_u32.to_le_bytes());
    bytes.push(0);
    // One entity: its id, then its position.
    bytes.extend(1_u32.to_le_bytes());
    bytes.extend(1_u32.to_le_bytes());
    bytes.extend([0; 12]);
    bytes.extend(disposition);
    bytes
}

#[test]
fn an_unknown_disposition_is_refused() {
    // The controls: no disposition, hostile and neutral decode, each from
    // its own tag.
    let known = [
        (0, None),
        (1, Some(Disposition::Hostile)),
        (2, Some(Disposition::Neutral)),
    ];
    for (tag, disposition) in known {
        let decoded = match ServerMessage::decode(&snapshot_bytes(&[tag])) {
            Ok(ServerMessage::Snapshot(snapshot)) => snapshot.entities,
            other => panic!("a disposition tag of {tag} gives {other:?}"),
        };
        let dispositions: Vec<_> = decoded.iter().map(|entity| entity.disposition).collect();
        assert_eq!(dispositions, [disposition], "tag {tag}");
    }

    for tag in [3, u8::MAX] {
        assert!(
            ServerMessage::decode(&snapshot_bytes(&[tag])).is_err(),
            "a disposition tag of {tag} decodes"
        );
    }
}

/// The bytes of a command of entity 3, its action already encoded in
/// `action`.
fn command_bytes(action: &[u8]) -> Vec<u8> {
    let mut bytes = vec![1];
    bytes.extend(3_u32.to_le_bytes());
    bytes.extend(action);
    bytes
}

#[test]
fn an_unknown_action_tag_is_refused() {
    // The controls: a wait, a move east and an attack east decode.
    for action in [&[0][..], &[1, 2], &[2, 2]] {
        assert!(ClientMessage::decode(&command_bytes(action)).is_ok());
    }
    // An action tag of 3, alone and with a direction.
    for action in [&[3][..], &[3, 2]] {
        assert!(
            ClientMessage::decode(&command_bytes(action)).is_err(),
            "{action:?} decodes"
        );
    }
}
