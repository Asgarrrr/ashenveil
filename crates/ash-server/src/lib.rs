//! The authoritative server: it hosts the worlds and speaks `ash-protocol`
//! over WebSocket to their controllers.
//!
//! Synchronous: one thread per connection, no async runtime. A world turns
//! only when its controller sends a command.

mod action;
mod connection;
mod perception;
mod quarantine;
mod registry;

use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

use ash_protocol::message::ResumeToken;

pub use registry::world_seed;

use registry::Registry;

/// Serves every connection `listener` accepts, each on its own thread.
/// Never returns.
pub fn serve(listener: TcpListener) {
    serve_registry(listener, Registry::default());
}

/// `serve`, except that the world of `token` violates an invariant once it
/// accepts its first command. Tests use it to prove the quarantine.
///
/// The fault cannot reach production. `serve` never sets it, and no message
/// can. `#[cfg(test)]` would hide this function from the integration tests.
/// A Cargo feature would need a self dev-dependency to reach them, and the
/// dependency matrix, part of the constitution, allows no such edge.
#[doc(hidden)]
pub fn serve_with_fault(listener: TcpListener, token: ResumeToken) {
    serve_registry(listener, Registry::with_fault(token));
}

/// `serve`, except that the world of `token` holds two characters side by
/// side and no creature. The client drives the first. Whenever the world
/// awaits the second, the server makes it attack toward the start tile of
/// the first, or wait when that attack is not offered. Tests use it to
/// strike the character of a client.
///
/// It cannot reach production, like `serve_with_fault`: `serve` never sets
/// it, and no message can.
#[doc(hidden)]
pub fn serve_with_struck_character(listener: TcpListener, token: ResumeToken) {
    serve_registry(listener, Registry::with_struck_character(token));
}

fn serve_registry(listener: TcpListener, registry: Registry) {
    let registry = Arc::new(registry);
    for stream in listener.incoming() {
        // A failed accept concerns that one connection only.
        let Ok(stream) = stream else { continue };
        let registry = Arc::clone(&registry);
        thread::spawn(move || connection::run(stream, &registry));
    }
}
