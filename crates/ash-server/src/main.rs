//! `ash-server <address>`: serves the worlds over WebSocket on `address`
//! until the process is killed. `cargo xtask play` starts it.

use std::net::TcpListener;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [address] = args.as_slice() else {
        eprintln!("usage: ash-server <address>");
        return ExitCode::from(2);
    };
    match TcpListener::bind(address) {
        Ok(listener) => {
            println!("ash-server: listening on ws://{address}");
            ash_server::serve(listener);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("ash-server: cannot listen on {address}: {error}");
            ExitCode::FAILURE
        }
    }
}
