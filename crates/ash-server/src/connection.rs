//! One client connection: the controller of one world's character.

use std::net::TcpStream;

use ash_core::entity::EntityId;
use ash_protocol::message::{Action, ClientMessage, ServerMessage};
use ash_sim::world::World;
use tungstenite::{Message, WebSocket};

use crate::perception;
use crate::registry::{HostedWorld, Registry};

type Socket = WebSocket<TcpStream>;

/// The connection ends: the client left, broke the protocol, or its world
/// hit a violated invariant.
struct Ended;

/// Serves one client until the connection ends. The world keeps its state
/// (R19).
pub fn run(stream: TcpStream, registry: &Registry) {
    let _ = control(stream, registry);
}

fn control(stream: TcpStream, registry: &Registry) -> Result<(), Ended> {
    let mut socket = tungstenite::accept(stream).map_err(|_| Ended)?;
    let ClientMessage::Hello { token } = receive(&mut socket)? else {
        return Err(Ended);
    };
    let hosted = registry.world(token);
    let greeting = {
        let world = lock(&hosted)?;
        let mut greeting = vec![perception::snapshot(&world)];
        greeting.extend(perception::awaited(&world, hosted.character));
        greeting
    };
    send(&mut socket, greeting)?;
    loop {
        let ClientMessage::Command { entity, action } = receive(&mut socket)? else {
            return Err(Ended);
        };
        // The world lock is held for one command only, never across a read.
        let replies = step(&mut *lock(&hosted)?, hosted.character, entity, action);
        send(&mut socket, replies)?;
    }
}

/// `entity` takes `action` for the controller of `character`. Returns the
/// replies in order: the perceived changes, then the next turn of
/// `character` (R17, R9), or a refusal that advanced nothing (R10).
fn step(
    world: &mut World,
    character: EntityId,
    entity: EntityId,
    action: Action,
) -> Vec<ServerMessage> {
    // A controller drives only its own character, and only on its turn.
    if entity != character || world.awaited() != character {
        return vec![ServerMessage::Refused];
    }
    let seen = world.perceived_changes().len();
    if world.submit(sim_action(action)).is_err() {
        return vec![ServerMessage::Refused];
    }
    let mut replies: Vec<_> = perception::changes_since(world, seen).collect();
    replies.extend(perception::awaited(world, character));
    replies
}

fn sim_action(action: Action) -> ash_sim::action::Action {
    match action {
        Action::Move(direction) => ash_sim::action::Action::Move(direction),
        Action::Wait => ash_sim::action::Action::Wait,
    }
}

/// A poisoned lock means the world panicked on a violated invariant: it is
/// never stepped again.
fn lock(hosted: &HostedWorld) -> Result<std::sync::MutexGuard<'_, World>, Ended> {
    hosted.world.lock().map_err(|_| Ended)
}

fn receive(socket: &mut Socket) -> Result<ClientMessage, Ended> {
    loop {
        match socket.read().map_err(|_| Ended)? {
            Message::Binary(bytes) => return ClientMessage::decode(&bytes).map_err(|_| Ended),
            // tungstenite answers pings itself.
            Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {}
            Message::Text(_) | Message::Close(_) => return Err(Ended),
        }
    }
}

fn send(socket: &mut Socket, messages: Vec<ServerMessage>) -> Result<(), Ended> {
    for message in messages {
        socket
            .write(Message::binary(message.encode()))
            .map_err(|_| Ended)?;
    }
    socket.flush().map_err(|_| Ended)
}
