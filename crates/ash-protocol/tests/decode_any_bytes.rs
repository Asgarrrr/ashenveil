//! Spec 0001, slice S4: decoding never panics on any input.

use ash_core::entity::EntityId;
use ash_core::rng::Rng;
use ash_core::tile::{Direction, TilePosition};
use ash_core::time::{ActionCost, Time};
use ash_protocol::message::{
    Action, ChangeKind, ClientMessage, Condition, Died, Disposition, Offer, Outcome,
    PerceivedChange, PerceivedCorpse, PerceivedEntity, PerceivedHealth, ResumeToken, ServerMessage,
    Snapshot, Terrain,
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
                    health: PerceivedHealth::HitPoints {
                        current: 7,
                        max: u32::MAX,
                    },
                },
                PerceivedEntity {
                    entity: EntityId::new(1),
                    position: TilePosition {
                        x: u32::MAX,
                        y: 2,
                        z: 7,
                    },
                    disposition: Some(Disposition::Hostile),
                    health: PerceivedHealth::Condition(Condition::Unhurt),
                },
                PerceivedEntity {
                    entity: EntityId::new(2),
                    position: TilePosition { x: 1, y: 0, z: 0 },
                    disposition: Some(Disposition::Neutral),
                    health: PerceivedHealth::Condition(Condition::Dying),
                },
            ],
            corpses: vec![
                PerceivedCorpse {
                    position: TilePosition { x: 1, y: 1, z: 0 },
                    died: Died::Creature,
                    entity: Some(EntityId::new(u32::MAX)),
                },
                PerceivedCorpse {
                    position: TilePosition {
                        x: 2,
                        y: u32::MAX,
                        z: 3,
                    },
                    died: Died::Character,
                    entity: None,
                },
            ],
        }),
        ServerMessage::Awaited {
            time: Time::new(u64::MAX),
            entity: EntityId::new(0),
            offers: Direction::ALL
                .into_iter()
                .map(Action::Move)
                .chain([Action::Wait, Action::Attack(Direction::East)])
                .map(|action| Offer {
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
        ServerMessage::Perceived(PerceivedChange {
            time: Time::new(u64::MAX),
            entity: EntityId::new(u32::MAX),
            kind: ChangeKind::Death,
        }),
        condition_change(0, 3, Condition::Unhurt),
        condition_change(300, 1, Condition::Wounded),
        condition_change(u64::MAX, u32::MAX, Condition::BadlyWounded),
        condition_change(301, 0, Condition::Dying),
        ServerMessage::Refused,
        ServerMessage::Quarantined,
        ServerMessage::ControllerTaken,
    ]
}

fn perceived(time: u64, entity: u32, outcome: Outcome) -> ServerMessage {
    ServerMessage::Perceived(PerceivedChange {
        time: Time::new(time),
        entity: EntityId::new(entity),
        kind: ChangeKind::Act(outcome),
    })
}

fn condition_change(time: u64, entity: u32, condition: Condition) -> ServerMessage {
    ServerMessage::Perceived(PerceivedChange {
        time: Time::new(time),
        entity: EntityId::new(entity),
        kind: ChangeKind::Condition(condition),
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

/// The bytes of a perceived change of kind `tag` at time 100 naming entity
/// 2, then `rest`.
fn change_bytes(tag: u8, rest: &[u8]) -> Vec<u8> {
    let mut bytes = vec![2, tag];
    bytes.extend(100_u64.to_le_bytes());
    bytes.extend(2_u32.to_le_bytes());
    bytes.extend(rest);
    bytes
}

/// The bytes of an act at time 100 by entity 2, its outcome already encoded
/// in `outcome`.
fn perceived_bytes(outcome: &[u8]) -> Vec<u8> {
    change_bytes(0, outcome)
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

#[test]
fn a_malformed_death_is_refused() {
    // The control: a change of kind 1 is the death of the entity it names,
    // at its time, and nothing more.
    assert_eq!(
        ServerMessage::decode(&change_bytes(1, &[])),
        Ok(ServerMessage::Perceived(PerceivedChange {
            time: Time::new(100),
            entity: EntityId::new(2),
            kind: ChangeKind::Death,
        }))
    );

    // A death carries no outcome: not a wait, a step north or a hit on
    // entity 1.
    for outcome in [&[1][..], &[0, 0], &[3, 1, 0, 0, 0]] {
        assert!(
            ServerMessage::decode(&change_bytes(1, outcome)).is_err(),
            "a death with the outcome {outcome:?} decodes"
        );
    }
    // A death cut short, down to its kind alone.
    let death = change_bytes(1, &[]);
    for length in 2..death.len() {
        assert!(
            ServerMessage::decode(&death[..length]).is_err(),
            "a death of {length} bytes decodes"
        );
    }
}

#[test]
fn a_malformed_change_of_condition_is_refused() {
    // The controls: a change of kind 2 is the entity it names coming, at its
    // time, to the condition its one tag names.
    let known = [
        (0, Condition::Unhurt),
        (1, Condition::Wounded),
        (2, Condition::BadlyWounded),
        (3, Condition::Dying),
    ];
    for (tag, condition) in known {
        assert_eq!(
            ServerMessage::decode(&change_bytes(2, &[tag])),
            Ok(ServerMessage::Perceived(PerceivedChange {
                time: Time::new(100),
                entity: EntityId::new(2),
                kind: ChangeKind::Condition(condition),
            })),
            "tag {tag}"
        );
    }

    for tag in 4..=u8::MAX {
        assert!(
            ServerMessage::decode(&change_bytes(2, &[tag])).is_err(),
            "a condition tag of {tag} decodes"
        );
    }
    // A change of condition carries one condition: not none, and no hit
    // points after it.
    for rest in [&[][..], &[1, 3, 0, 0, 0], &[1, 3, 0, 0, 0, 6, 0, 0, 0]] {
        assert!(
            ServerMessage::decode(&change_bytes(2, rest)).is_err(),
            "a change of condition with {rest:?} decodes"
        );
    }
    // No kind follows the change of condition, with the bytes of a death or
    // of a wounded entity.
    for tag in 3..=u8::MAX {
        for rest in [&[][..], &[1]] {
            assert!(
                ServerMessage::decode(&change_bytes(tag, rest)).is_err(),
                "a change of kind {tag} decodes"
            );
        }
    }
}

/// The bytes of a snapshot at time 0 of one floor tile: its tag, its time,
/// a width of 1, then one tile: floor.
fn floor_snapshot_bytes() -> Vec<u8> {
    let mut bytes = vec![0];
    bytes.extend(0_u64.to_le_bytes());
    bytes.extend(1_u32.to_le_bytes());
    bytes.extend(1_u32.to_le_bytes());
    bytes.push(0);
    bytes
}

/// The bytes of a snapshot at time 0 of one floor tile, holding entity 1 at
/// (0, 0, 0), its disposition and its hit points or condition already
/// encoded in `disposition` and `health`, and no corpse.
fn snapshot_bytes(disposition: &[u8], health: &[u8]) -> Vec<u8> {
    let mut bytes = floor_snapshot_bytes();
    // One entity: its id, then its position.
    bytes.extend(1_u32.to_le_bytes());
    bytes.extend(1_u32.to_le_bytes());
    bytes.extend([0; 12]);
    bytes.extend(disposition);
    bytes.extend(health);
    bytes.extend(0_u32.to_le_bytes());
    bytes
}

/// The bytes of a snapshot at time 0 of one floor tile, holding no entity
/// and one corpse at (0, 0, 0), what died and its entity already encoded in
/// `rest`.
fn corpse_snapshot_bytes(rest: &[u8]) -> Vec<u8> {
    let mut bytes = floor_snapshot_bytes();
    bytes.extend(0_u32.to_le_bytes());
    bytes.extend(1_u32.to_le_bytes());
    bytes.extend([0; 12]);
    bytes.extend(rest);
    bytes
}

#[test]
fn a_malformed_corpse_is_refused() {
    // The controls: what died is tag 0 for a character, 1 for a creature;
    // then tag 0 for an unrecognized corpse, or tag 1 and the dead entity.
    let at_origin = TilePosition { x: 0, y: 0, z: 0 };
    let known = [
        (&[0, 0][..], Died::Character, None),
        (&[1, 0], Died::Creature, None),
        (&[0, 1, 7, 0, 0, 0], Died::Character, Some(EntityId::new(7))),
        (&[1, 1, 7, 0, 0, 0], Died::Creature, Some(EntityId::new(7))),
    ];
    for (rest, died, entity) in known {
        match ServerMessage::decode(&corpse_snapshot_bytes(rest)) {
            Ok(ServerMessage::Snapshot(snapshot)) => assert_eq!(
                snapshot.corpses,
                [PerceivedCorpse {
                    position: at_origin,
                    died,
                    entity,
                }],
                "{rest:?}"
            ),
            other => panic!("{rest:?} give {other:?}"),
        }
    }

    for tag in 2..=u8::MAX {
        for rest in [[tag, 0], [0, tag], [1, tag]] {
            assert!(
                ServerMessage::decode(&corpse_snapshot_bytes(&rest)).is_err(),
                "a corpse of {rest:?} decodes"
            );
        }
    }
    let refused = [
        ("a corpse with nothing after its tile", &[][..]),
        ("a corpse with no tag for its entity", &[1]),
        ("a recognized corpse with no entity", &[1, 1]),
        (
            "a recognized corpse with three bytes of entity",
            &[1, 1, 7, 0, 0],
        ),
        ("an unrecognized corpse with an entity", &[1, 0, 7, 0, 0, 0]),
    ];
    for (what, rest) in refused {
        assert!(
            ServerMessage::decode(&corpse_snapshot_bytes(rest)).is_err(),
            "{what} decodes"
        );
    }
    // A corpse cut short inside its tile.
    let whole = corpse_snapshot_bytes(&[1, 0]);
    for cut in 1..=12 {
        let mut bytes = whole.clone();
        bytes.drain(whole.len() - 2 - cut..whole.len() - 2);
        assert!(
            ServerMessage::decode(&bytes).is_err(),
            "a corpse with {cut} bytes fewer in its tile decodes"
        );
    }
}

/// The entities of the snapshot `bytes` hold.
fn snapshot_entities(bytes: &[u8]) -> Vec<PerceivedEntity> {
    match ServerMessage::decode(bytes) {
        Ok(ServerMessage::Snapshot(snapshot)) => snapshot.entities,
        other => panic!("{bytes:?} give {other:?}"),
    }
}

/// The bytes of an unhurt entity: the tag of a condition, then unhurt.
const UNHURT: [u8; 2] = [1, 0];

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
        let decoded = snapshot_entities(&snapshot_bytes(&[tag], &UNHURT));
        let dispositions: Vec<_> = decoded.iter().map(|entity| entity.disposition).collect();
        assert_eq!(dispositions, [disposition], "tag {tag}");
    }

    for tag in 3..=u8::MAX {
        assert!(
            ServerMessage::decode(&snapshot_bytes(&[tag], &UNHURT)).is_err(),
            "a disposition tag of {tag} decodes"
        );
    }
}

#[test]
fn malformed_hit_points_or_condition_are_refused() {
    // The controls: tag 0 is hit points, the current ones then the most;
    // tag 1 is a condition, named by its own tag.
    let known = [
        (
            &[0, 7, 0, 0, 0, 10, 0, 0, 0][..],
            PerceivedHealth::HitPoints {
                current: 7,
                max: 10,
            },
        ),
        (&[1, 0], PerceivedHealth::Condition(Condition::Unhurt)),
        (&[1, 1], PerceivedHealth::Condition(Condition::Wounded)),
        (&[1, 2], PerceivedHealth::Condition(Condition::BadlyWounded)),
        (&[1, 3], PerceivedHealth::Condition(Condition::Dying)),
    ];
    for (bytes, health) in known {
        let decoded = snapshot_entities(&snapshot_bytes(&[0], bytes));
        let healths: Vec<_> = decoded.iter().map(|entity| entity.health).collect();
        assert_eq!(healths, [health], "{bytes:?}");
    }

    let refused = [
        ("no hit points and no condition", &[][..]),
        ("hit points with no number", &[0]),
        ("hit points with one number", &[0, 7, 0, 0, 0]),
        ("a condition with no tag", &[1]),
        ("a condition tag of 4", &[1, 4]),
        (
            "a condition, then hit points",
            &[1, 1, 7, 0, 0, 0, 10, 0, 0, 0],
        ),
        (
            "hit points, then a condition",
            &[0, 7, 0, 0, 0, 10, 0, 0, 0, 1],
        ),
    ];
    for (what, bytes) in refused {
        assert!(
            ServerMessage::decode(&snapshot_bytes(&[0], bytes)).is_err(),
            "{what} decodes"
        );
    }
    // Neither hit points nor a condition: no kind follows them, with the
    // bytes of either.
    for tag in 2..=u8::MAX {
        for rest in [&[0][..], &[7, 0, 0, 0, 10, 0, 0, 0]] {
            let bytes = [&[tag][..], rest].concat();
            assert!(
                ServerMessage::decode(&snapshot_bytes(&[0], &bytes)).is_err(),
                "a tag of {tag} for hit points or a condition decodes"
            );
        }
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
