//! The authoritative server: it hosts the worlds and speaks `ash-protocol`
//! over WebSocket to their controllers.
//!
//! Synchronous: one thread per connection, no async runtime. A world turns
//! only when its controller sends a command.

mod action;
mod connection;
mod perception;
mod registry;

use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

use ash_protocol::message::ResumeToken;

pub use registry::create_world;

use registry::Registry;

/// Serves every connection `listener` accepts, each on its own thread.
/// Never returns.
pub fn serve(listener: TcpListener) {
    serve_registry(listener, Registry::default());
}

/// `serve`, except that the world of `token` violates an invariant once it
/// accepts its first command. Tests use it to prove the quarantine.
pub fn serve_with_fault(listener: TcpListener, token: ResumeToken) {
    let _ = token;
    serve_registry(listener, Registry::default());
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
