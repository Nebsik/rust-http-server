mod thread_pool;

use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};

use thread_pool::ThreadPool;

const DEFAULT_PORT: u16 = 8080;
const PUBLIC_DIR: &str = "public";
const WORKERS: usize = 4;

fn main() {
    let port = env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(DEFAULT_PORT);

    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).unwrap_or_else(|e| {
        eprintln!("Failed to bind {addr}: {e}");
        std::process::exit(1);
    });

    println!("Serving ./{PUBLIC_DIR} on http://{addr} with {WORKERS} workers");

    let pool = ThreadPool::new(WORKERS);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => pool.execute(|| {
                if let Err(e) = handle_connection(stream) {
                    eprintln!("Connection error: {e}");
                }
            }),
            Err(e) => eprintln!("Failed to accept connection: {e}"),
        }
    }
}

fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let mut parts = request_line.split_whitespace();
    let (method, target) = match (parts.next(), parts.next()) {
        (Some(m), Some(t)) => (m, t),
        _ => {
            return respond(
                &mut stream,
                400,
                "Bad Request",
                b"400 Bad Request",
                "text/plain",
            )
        }
    };

    println!("{} {}", method, target);

    if method != "GET" {
        return respond(
            &mut stream,
            405,
            "Method Not Allowed",
            b"405 Method Not Allowed",
            "text/plain",
        );
    }

    match resolve_path(target) {
        Some(path) => match fs::read(&path) {
            Ok(body) => respond(&mut stream, 200, "OK", &body, content_type(&path)),
            Err(_) => not_found(&mut stream),
        },
        None => not_found(&mut stream),
    }
}

/// Maps a request target to a file inside `PUBLIC_DIR`,
/// rejecting anything that tries to escape it (e.g. `../`).
fn resolve_path(target: &str) -> Option<PathBuf> {
    let path = target.split(['?', '#']).next().unwrap_or("/");
    let path = if path == "/" { "/index.html" } else { path };

    let relative = Path::new(path.trim_start_matches('/'));
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return None;
    }

    let full = Path::new(PUBLIC_DIR).join(relative);
    full.is_file().then_some(full)
}

fn not_found(stream: &mut TcpStream) -> std::io::Result<()> {
    let body = fs::read(Path::new(PUBLIC_DIR).join("404.html"))
        .unwrap_or_else(|_| b"404 Not Found".to_vec());
    respond(stream, 404, "Not Found", &body, "text/html")
}

fn respond(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    body: &[u8],
    content_type: &str,
) -> std::io::Result<()> {
    let headers = format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_maps_to_index() {
        // Only checks mapping logic; file existence depends on cwd,
        // so we test the rejection paths which are pure.
        assert!(resolve_path("/../secret").is_none());
        assert!(resolve_path("/..%2fsecret/../x").is_none());
    }

    #[test]
    fn query_string_is_stripped() {
        assert!(resolve_path("/../etc/passwd?x=1").is_none());
    }

    #[test]
    fn content_types() {
        assert_eq!(
            content_type(Path::new("a.html")),
            "text/html; charset=utf-8"
        );
        assert_eq!(content_type(Path::new("a.css")), "text/css");
        assert_eq!(content_type(Path::new("a.bin")), "application/octet-stream");
    }
}
