//! The authoritative server: it hosts the worlds and speaks `ash-protocol`
//! over WebSocket to their controllers.
//!
//! Synchronous: one thread per connection, no async runtime. A world turns
//! only when its controller sends a command.

mod connection;
mod perception;
mod registry;

use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

pub use registry::create_world;

use registry::Registry;

/// Serves every connection `listener` accepts, each on its own thread.
/// Never returns.
pub fn serve(listener: TcpListener) {
    let registry = Arc::new(Registry::default());
    for stream in listener.incoming() {
        // A failed accept concerns that one connection only.
        let Ok(stream) = stream else { continue };
        let registry = Arc::clone(&registry);
        thread::spawn(move || connection::run(stream, &registry));
    }
}
