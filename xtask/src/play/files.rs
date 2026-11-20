//! Serves the files of `web/` over HTTP, read-only, for the local browser.
//! GET only, one thread per connection, every response closes it.

use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::thread;

/// Serves `web` to every connection `listener` accepts. Never returns.
pub fn serve(listener: TcpListener, web: PathBuf) {
    let web = Arc::new(web);
    for stream in listener.incoming() {
        // A failed accept concerns that one connection only.
        let Ok(stream) = stream else { continue };
        let web = Arc::clone(&web);
        thread::spawn(move || {
            let _ = respond(stream, &web);
        });
    }
}

fn respond(stream: TcpStream, web: &Path) -> io::Result<()> {
    let mut reader = BufReader::new(stream);
    let mut request = String::new();
    reader.read_line(&mut request)?;
    // Read the headers too: closing a socket with unread bytes resets it,
    // and the browser may lose the response.
    let mut header = String::new();
    while reader.read_line(&mut header)? > 2 {
        header.clear();
    }
    let found = requested_file(web, &request).and_then(|path| {
        let body = std::fs::read(&path).ok()?;
        Some((content_type(&path), body))
    });
    let (status, content_type, body) = match found {
        Some((content_type, body)) => ("200 OK", content_type, body),
        None => ("404 Not Found", "text/plain", b"not found\n".to_vec()),
    };
    let mut stream = reader.into_inner();
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(&body)?;
    stream.flush()
}

/// The file below `web` a request line such as `GET /build/main.js HTTP/1.1`
/// names. `None` for any other method, and for a path that could leave
/// `web`.
fn requested_file(web: &Path, request: &str) -> Option<PathBuf> {
    let target = request.strip_prefix("GET ")?.split_whitespace().next()?;
    let path = target.split(['?', '#']).next()?.strip_prefix('/')?;
    let path = if path.is_empty() { "index.html" } else { path };
    let relative = Path::new(path);
    relative
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
        .then(|| web.join(relative))
}

/// A module script with another type does not run, and the WASM loader
/// falls back to a slower path without `application/wasm`.
fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript",
        Some("wasm") => "application/wasm",
        Some("css") => "text/css",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::net::SocketAddr;

    use super::super::super::verify::test_repo::TestRepo;
    use super::*;

    fn start(repo: &TestRepo) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let web = repo.root().join("web");
        thread::spawn(move || serve(listener, web));
        address
    }

    /// The whole response to a GET of `target`.
    fn get(address: SocketAddr, target: &str) -> String {
        let mut stream = TcpStream::connect(address).unwrap();
        write!(stream, "GET {target} HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }

    #[test]
    fn page_module_and_wasm_are_served_with_their_types() {
        let repo = TestRepo::new();
        repo.write("web/index.html", "<p>page</p>");
        repo.write("web/build/main.js", "export {};");
        repo.write("web/pkg/ash_wasm_bg.wasm", "wasm");
        let address = start(&repo);
        for (target, content_type, body) in [
            ("/", "text/html; charset=utf-8", "<p>page</p>"),
            ("/build/main.js?v=1", "text/javascript", "export {};"),
            ("/pkg/ash_wasm_bg.wasm", "application/wasm", "wasm"),
        ] {
            let response = get(address, target);
            assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
            assert!(
                response.contains(&format!("\r\nContent-Type: {content_type}\r\n")),
                "{response}"
            );
            assert!(response.ends_with(&format!("\r\n\r\n{body}")), "{response}");
        }
    }

    #[test]
    fn path_out_of_the_web_directory_is_refused() {
        let repo = TestRepo::new();
        repo.write("secret.txt", "secret");
        repo.write("web/index.html", "<p>page</p>");
        let address = start(&repo);
        for target in ["/../secret.txt", "/build/../../secret.txt", "//secret.txt"] {
            let response = get(address, target);
            assert!(
                response.starts_with("HTTP/1.1 404 "),
                "{target}: {response}"
            );
        }
    }
}
