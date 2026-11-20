//! One client connection: the controller of one world's character.

use std::net::TcpStream;

use ash_core::entity::EntityId;
use ash_protocol::message::{Action, ClientMessage, ServerMessage};
use ash_sim::world::World;
use tungstenite::protocol::Role;
use tungstenite::{Message, WebSocket};

use crate::registry::{Controller, HostedWorld, Registry};
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
    let Ok(controller) = Controller::new(&stream) else {
        return;
    };
    if let Ok(mut socket) = tungstenite::accept(stream)
        && let Some(ClientMessage::Hello { token }) = receive(&mut socket)
    {
        let hosted = registry.world(token);
        match control(&mut socket, &hosted, &controller) {
            End::Left => {}
            End::Quarantined => farewell(socket, ServerMessage::Quarantined),
            End::ControllerTaken => farewell(socket, ServerMessage::ControllerTaken),
        }
    }
    controller.close();
}

fn control(socket: &mut Socket, hosted: &HostedWorld, controller: &Controller) -> End {
    let greeting = hosted.run(|controlled| {
        controlled.take(controller);
        let mut greeting = vec![perception::snapshot(&controlled.world)];
        greeting.extend(perception::awaited(&controlled.world, hosted.character));
        greeting
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
