//! Spec 0001, slice S4: decoding never panics on any input.

use ash_core::entity::EntityId;
use ash_core::rng::Rng;
use ash_core::tile::{Direction, TilePosition};
use ash_core::time::{ActionCost, Time};
use ash_protocol::message::{
    Action, AvailableAction, ClientMessage, PerceivedChange, PerceivedEntity, ResumeToken,
    ServerMessage, Snapshot, Terrain,
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
        },
        ClientMessage::Command {
            entity: EntityId::new(3),
            action: Action::Move(Direction::NorthWest),
        },
        ClientMessage::Command {
            entity: EntityId::new(u32::MAX),
            action: Action::Wait,
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
                },
                PerceivedEntity {
                    entity: EntityId::new(1),
                    position: TilePosition {
                        x: u32::MAX,
                        y: 2,
                        z: 7,
                    },
                },
            ],
        }),
        ServerMessage::Awaited {
            time: Time::new(u64::MAX),
            entity: EntityId::new(0),
            actions: Direction::ALL
                .into_iter()
                .map(Action::Move)
                .chain([Action::Wait])
                .map(|action| AvailableAction {
                    action,
                    cost: ActionCost::new(100),
                })
                .collect(),
        },
        ServerMessage::Perceived(PerceivedChange {
            time: Time::new(100),
            entity: EntityId::new(2),
            action: Action::Move(Direction::South),
            bumped: None,
        }),
        ServerMessage::Perceived(PerceivedChange {
            time: Time::new(150),
            entity: EntityId::new(1),
            action: Action::Move(Direction::West),
            bumped: Some(EntityId::new(u32::MAX)),
        }),
        ServerMessage::Refused,
        ServerMessage::Quarantined,
        ServerMessage::ControllerTaken,
    ]
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
