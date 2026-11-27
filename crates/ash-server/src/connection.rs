//! One client connection: the controller of one world's character.

use std::net::TcpStream;

use ash_core::entity::EntityId;
use ash_protocol::message::{Action, ClientMessage, ServerMessage};
use ash_sim::world::World;
use tungstenite::protocol::Role;
use tungstenite::{Message, WebSocket};

use crate::registry::{HostedWorld, PlayerConnection, Registry};
use crate::{action, perception};

type Socket = WebSocket<TcpStream>;

/// Why a connection ends.
enum End {
    /// The client left or broke the protocol.
    Left,
    /// Its world hit a violated invariant.
    Quarantined,
    /// A newer connection took the controller (R20).
    ControllerTaken,
}

/// Serves one client until the connection ends. The world keeps its state
/// (R19).
pub fn run(stream: TcpStream, registry: &Registry) {
    let Ok(controller) = PlayerConnection::new(&stream) else {
        return;
    };
    if let Ok(mut socket) = tungstenite::accept(stream)
        && let Some(ClientMessage::Hello { token, received }) = receive(&mut socket)
    {
        let hosted = registry.world(token);
        // A count past every change sends none.
        let received = usize::try_from(received).unwrap_or(usize::MAX);
        match control(&mut socket, &hosted, &controller, received) {
            End::Left => {}
            End::Quarantined => farewell(socket, ServerMessage::Quarantined),
            End::ControllerTaken => farewell(socket, ServerMessage::ControllerTaken),
        }
    }
    // The registry may still hold a clone of the stream: dropping the socket
    // alone would leave the connection open.
    controller.close();
}

/// Greets the client, then serves its commands. Only the client knows what
/// it `received`: a reply the server wrote may never have been read (R17).
fn control(
    socket: &mut Socket,
    hosted: &HostedWorld,
    controller: &PlayerConnection,
    received: usize,
) -> End {
    let greeting = hosted.run(|controlled| {
        controlled.take(controller);
        greeting(&controlled.world, hosted.character, received)
    });
    let Ok(greeting) = greeting else {
        return End::Quarantined;
    };
    if send(socket, greeting).is_err() {
        return End::Left;
    }
    loop {
        let received = receive(socket);
        // The world lock is held for one command only, never across a read.
        // A read the takeover interrupted ends here too.
        let replies = hosted.run(|controlled| {
            if !controlled.is_controlled_by(controller) {
                return Err(End::ControllerTaken);
            }
            let Some(ClientMessage::Command { entity, action }) = received else {
                return Err(End::Left);
            };
            Ok(step(&mut controlled.world, hosted, entity, action))
        });
        match replies {
            Ok(Ok(replies)) => {
                if send(socket, replies).is_err() {
                    return End::Left;
                }
            }
            Ok(Err(end)) => return end,
            Err(_) => return End::Quarantined,
        }
    }
}

/// What the controller of `character` gets on connection: the changes after
/// the first `received`, then the snapshot of `character` and its pending
/// turn (R17, R16, R9).
fn greeting(world: &World, character: EntityId, received: usize) -> Vec<ServerMessage> {
    let mut greeting: Vec<_> = perception::changes_since(world, received).collect();
    greeting.push(perception::snapshot(world, character));
    greeting.extend(perception::awaited(world, character));
    greeting
}

/// `entity` takes `action` for the controller of `hosted`'s character.
/// Returns the replies in order: the perceived changes, then the next turn
/// of the character (R17, R9), or a refusal that advanced nothing (R10).
fn step(
    world: &mut World,
    hosted: &HostedWorld,
    entity: EntityId,
    action: Action,
) -> Vec<ServerMessage> {
    let character = hosted.character;
    // A controller drives only its own character, and only on its turn.
    if entity != character || world.awaited() != character {
        return vec![ServerMessage::Refused];
    }
    let seen = world.perceived_changes().len();
    if world.submit(action::to_sim(action)).is_err() {
        return vec![ServerMessage::Refused];
    }
    if hosted.faulty {
        ash_core::invariant!("the test fault of this world fired");
    }
    let mut replies: Vec<_> = perception::changes_since(world, seen).collect();
    replies.extend(perception::awaited(world, character));
    replies
}

/// Sends `message`, then closes the WebSocket. It writes through a new
/// WebSocket over the same stream: a read the takeover interrupted leaves
/// the first one unable to write.
fn farewell(socket: Socket, message: ServerMessage) {
    let mut socket = WebSocket::from_raw_socket(socket.into_inner(), Role::Server, None);
    if send(&mut socket, vec![message]).is_ok() {
        let _ = socket.close(None);
    }
}

/// The next message; `None` when the connection ended or the bytes do not
/// decode.
fn receive(socket: &mut Socket) -> Option<ClientMessage> {
    loop {
        match socket.read().ok()? {
            Message::Binary(bytes) => return ClientMessage::decode(&bytes).ok(),
            // tungstenite answers pings itself.
            Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {}
            Message::Text(_) | Message::Close(_) => return None,
        }
    }
}

/// The client left.
struct Gone;

fn send(socket: &mut Socket, messages: Vec<ServerMessage>) -> Result<(), Gone> {
    for message in messages {
        socket
            .write(Message::binary(message.encode()))
            .map_err(|_| Gone)?;
    }
    socket.flush().map_err(|_| Gone)
}

#[cfg(test)]
mod tests {
    use ash_protocol::message::{PerceivedHealth, ResumeToken};

    use super::*;
    use crate::perception::tests::world_awaiting_another_player;

    #[test]
    fn a_command_while_the_world_awaits_another_entity_is_refused() {
        let (mut world, character, _) = world_awaiting_another_player();
        let hosted = Registry::default().world(ResumeToken(1));
        assert_eq!(hosted.character, character);
        let before = world.clone();

        let replies = step(&mut world, &hosted, character, Action::Wait);

        assert_eq!(replies, [ServerMessage::Refused]);
        assert_eq!(world, before);
    }

    /// A9, R18: the snapshot is for the character of the connection, also
    /// when the world awaits another one. No hosted world does yet.
    #[test]
    fn the_greeting_gives_hit_points_to_its_character_while_the_world_awaits_another() {
        let (world, character, _) = world_awaiting_another_player();
        let received = world.perceived_changes().len();

        let greeting = greeting(&world, character, received);

        let [ServerMessage::Snapshot(snapshot)] = &greeting[..] else {
            panic!("expected a snapshot alone, got {greeting:?}");
        };
        let given: Vec<EntityId> = snapshot
            .entities
            .iter()
            .filter(|perceived| matches!(perceived.health, PerceivedHealth::HitPoints { .. }))
            .map(|perceived| perceived.entity)
            .collect();
        assert_eq!(given, [character]);
    }
}
