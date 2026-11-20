//! One client connection: the controller of one world's character.

use std::net::TcpStream;

pub fn run(stream: TcpStream) {
    let _ = tungstenite::accept(stream);
}
