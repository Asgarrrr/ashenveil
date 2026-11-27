//! The web client's codec: `ash-protocol` compiled to WASM, so the browser
//! and the server share one codec.
//!
//! Only free functions cross the boundary. A decoded message crosses as JSON
//! text; `web/src/protocol.ts` declares its shape. Times and costs become
//! JSON numbers, exact up to 2^53 time units.

use ash_protocol::message::{
    Action, AvailableAction, ClientMessage, Condition, Direction, Disposition, EntityId, Outcome,
    PerceivedChange, PerceivedEntity, PerceivedHealth, ResumeToken, ServerMessage, Snapshot,
    Terrain,
};
use wasm_bindgen::prelude::wasm_bindgen;

/// The server message `bytes` hold, as JSON; `None` when they do not decode.
#[wasm_bindgen]
pub fn decode_server_message(bytes: &[u8]) -> Option<String> {
    ServerMessage::decode(bytes)
        .ok()
        .map(|message| message_json(&message))
}

/// The first message of a connection: take the controller of the world
/// `token` names, having received `received` of its perceived changes.
#[wasm_bindgen]
pub fn encode_hello(token: u128, received: u64) -> Vec<u8> {
    ClientMessage::Hello {
        token: ResumeToken(token),
        received,
    }
    .encode()
}

/// `entity` waits.
#[wasm_bindgen]
pub fn encode_wait(entity: u32) -> Vec<u8> {
    command(entity, Action::Wait)
}

/// `entity` moves toward `direction`, a name the decoded JSON uses; `None`
/// for any other name.
#[wasm_bindgen]
pub fn encode_move(entity: u32, direction: &str) -> Option<Vec<u8>> {
    named_direction(direction).map(|direction| command(entity, Action::Move(direction)))
}

/// `entity` attacks toward `direction`, a name the decoded JSON uses; `None`
/// for any other name.
#[wasm_bindgen]
pub fn encode_attack(entity: u32, direction: &str) -> Option<Vec<u8>> {
    named_direction(direction).map(|direction| command(entity, Action::Attack(direction)))
}

fn named_direction(name: &str) -> Option<Direction> {
    Direction::ALL
        .into_iter()
        .find(|candidate| direction_name(*candidate) == name)
}

fn command(entity: u32, action: Action) -> Vec<u8> {
    ClientMessage::Command {
        entity: EntityId::new(entity),
        action,
    }
    .encode()
}

fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::North => "north",
        Direction::NorthEast => "northeast",
        Direction::East => "east",
        Direction::SouthEast => "southeast",
        Direction::South => "south",
        Direction::SouthWest => "southwest",
        Direction::West => "west",
        Direction::NorthWest => "northwest",
    }
}

fn message_json(message: &ServerMessage) -> String {
    match message {
        ServerMessage::Snapshot(snapshot) => snapshot_json(snapshot),
        ServerMessage::Awaited {
            time,
            entity,
            actions,
        } => format!(
            r#"{{"kind":"awaited","time":{},"entity":{},"actions":[{}]}}"#,
            time.units(),
            entity.get(),
            list(actions, available_action_json)
        ),
        ServerMessage::Perceived(change) => format!(
            r#"{{"kind":"perceived","change":{}}}"#,
            change_json(*change)
        ),
        ServerMessage::Refused => r#"{"kind":"refused"}"#.to_owned(),
        ServerMessage::Quarantined => r#"{"kind":"quarantined"}"#.to_owned(),
        ServerMessage::ControllerTaken => r#"{"kind":"controller_taken"}"#.to_owned(),
    }
}

fn snapshot_json(snapshot: &Snapshot) -> String {
    let terrain = list(&snapshot.terrain, |terrain| {
        match terrain {
            Terrain::Floor => r#""floor""#,
            Terrain::Wall => r#""wall""#,
        }
        .to_owned()
    });
    let entities = list(
        &snapshot.entities,
        |PerceivedEntity {
             entity,
             position,
             disposition,
             health,
         }| {
            let disposition = match disposition {
                None => "null",
                Some(Disposition::Hostile) => r#""hostile""#,
                Some(Disposition::Neutral) => r#""neutral""#,
            };
            let health = match *health {
                PerceivedHealth::HitPoints { current, max } => {
                    format!(r#"{{"kind":"hit_points","current":{current},"max":{max}}}"#)
                }
                PerceivedHealth::Condition(condition) => format!(
                    r#"{{"kind":"condition","condition":"{}"}}"#,
                    condition_name(condition)
                ),
            };
            format!(
                r#"{{"entity":{},"position":{{"x":{},"y":{},"z":{}}},"disposition":{disposition},"health":{health}}}"#,
                entity.get(),
                position.x,
                position.y,
                position.z
            )
        },
    );
    format!(
        r#"{{"kind":"snapshot","time":{},"width":{},"terrain":[{terrain}],"entities":[{entities}]}}"#,
        snapshot.time.units(),
        snapshot.width
    )
}

fn available_action_json(offered: &AvailableAction) -> String {
    format!(
        r#"{{"action":{},"cost":{}}}"#,
        action_json(offered.action),
        offered.cost.units()
    )
}

fn action_json(action: Action) -> String {
    match action {
        Action::Wait => r#"{"kind":"wait"}"#.to_owned(),
        Action::Move(direction) => format!(
            r#"{{"kind":"move","direction":"{}"}}"#,
            direction_name(direction)
        ),
        Action::Attack(direction) => format!(
            r#"{{"kind":"attack","direction":"{}"}}"#,
            direction_name(direction)
        ),
    }
}

fn change_json(change: PerceivedChange) -> String {
    match change {
        PerceivedChange::Act {
            time,
            entity,
            outcome,
        } => format!(
            r#"{{"kind":"act","time":{},"entity":{},"outcome":{}}}"#,
            time.units(),
            entity.get(),
            outcome_json(outcome)
        ),
        PerceivedChange::Death { time, entity } => format!(
            r#"{{"kind":"death","time":{},"entity":{}}}"#,
            time.units(),
            entity.get()
        ),
        PerceivedChange::Condition {
            time,
            entity,
            condition,
        } => format!(
            r#"{{"kind":"condition","time":{},"entity":{},"condition":"{}"}}"#,
            time.units(),
            entity.get(),
            condition_name(condition)
        ),
    }
}

fn condition_name(condition: Condition) -> &'static str {
    match condition {
        Condition::Unhurt => "unhurt",
        Condition::Wounded => "wounded",
        Condition::BadlyWounded => "badly_wounded",
        Condition::Dying => "dying",
    }
}

fn outcome_json(outcome: Outcome) -> String {
    let target =
        |kind: &str, target: EntityId| format!(r#"{{"kind":"{kind}","target":{}}}"#, target.get());
    match outcome {
        Outcome::Stepped(direction) => format!(
            r#"{{"kind":"stepped","direction":"{}"}}"#,
            direction_name(direction)
        ),
        Outcome::Waited => r#"{"kind":"waited"}"#.to_owned(),
        Outcome::Bumped(bumped) => target("bumped", bumped),
        Outcome::Hit(hit) => target("hit", hit),
        Outcome::Missed(missed) => target("missed", missed),
    }
}

/// The JSON of each item, comma-separated.
fn list<T>(items: &[T], json: impl Fn(&T) -> String) -> String {
    items.iter().map(json).collect::<Vec<_>>().join(",")
}

#[cfg(test)]
mod tests {
    use ash_protocol::message::TilePosition;

    use super::*;

    #[test]
    fn each_direction_name_encodes_the_move_and_the_attack_it_names() {
        let names = [
            ("north", Direction::North),
            ("northeast", Direction::NorthEast),
            ("east", Direction::East),
            ("southeast", Direction::SouthEast),
            ("south", Direction::South),
            ("southwest", Direction::SouthWest),
            ("west", Direction::West),
            ("northwest", Direction::NorthWest),
        ];
        for (name, direction) in names {
            let bytes = encode_move(7, name).unwrap_or_else(|| panic!("{name} is unknown"));
            assert_eq!(
                ClientMessage::decode(&bytes),
                Ok(ClientMessage::Command {
                    entity: EntityId::new(7),
                    action: Action::Move(direction),
                }),
                "{name}"
            );
            let bytes = encode_attack(7, name).unwrap_or_else(|| panic!("{name} is unknown"));
            assert_eq!(
                ClientMessage::decode(&bytes),
                Ok(ClientMessage::Command {
                    entity: EntityId::new(7),
                    action: Action::Attack(direction),
                }),
                "{name}"
            );
        }
        assert_eq!(encode_move(7, "up"), None);
        assert_eq!(encode_attack(7, "up"), None);
    }

    /// The web client steps a perceived move with its own copy of the
    /// offsets, `STEPS` in `web/src/model.ts`. A copy that drifts would
    /// draw every mover off its true tile.
    #[test]
    fn the_web_client_steps_each_direction_by_the_same_offset() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../web/src/model.ts");
        let model = std::fs::read_to_string(path).expect("the web model is readable");
        let table = model
            .split_once("const STEPS")
            .and_then(|(_, rest)| rest.split_once("};"))
            .map(|(table, _)| table)
            .expect("the web model declares STEPS");
        // One `name: [east, south],` per line.
        let mut steps: Vec<(String, (i64, i64))> = table
            .lines()
            .filter_map(|line| {
                let (name, offset) = line.trim().split_once(": [")?;
                let (east, south) = offset.strip_suffix("],")?.split_once(", ")?;
                Some((name.to_owned(), (east.parse().ok()?, south.parse().ok()?)))
            })
            .collect();
        steps.sort();
        let origin = TilePosition { x: 1, y: 1, z: 0 };
        let mut expected: Vec<(String, (i64, i64))> = Direction::ALL
            .into_iter()
            .map(|direction| {
                let to = origin.step(direction);
                let offset = (
                    i64::from(to.x) - i64::from(origin.x),
                    i64::from(to.y) - i64::from(origin.y),
                );
                (direction_name(direction).to_owned(), offset)
            })
            .collect();
        expected.sort();
        assert_eq!(steps, expected);
    }

    #[test]
    fn hello_carries_the_whole_token_and_count() {
        let token = u128::MAX - 1;
        assert_eq!(
            ClientMessage::decode(&encode_hello(token, u64::MAX - 2)),
            Ok(ClientMessage::Hello {
                token: ResumeToken(token),
                received: u64::MAX - 2,
            })
        );
    }
}
