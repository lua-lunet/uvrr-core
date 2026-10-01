//! The conformance host's transport: a loopback HTTP/1.1 server and the
//! client its gates drive it with, in the standard library only.
//!
//! The host is a dev tool and the transport is its whole reason to exist,
//! so it carries no dependency. An HTTP framework here would be a
//! lifelong support tax on a crate whose whole claim is a bare sans-I/O
//! core; `std::net` is enough for four routes, and the parsing is short
//! enough to read.
//!
//! # The loopback rule
//!
//! The listener binds `127.0.0.1` and nothing else. The host drives a
//! cluster whose datagrams, journal and boot-gate markers are all
//! observable, so it is not a thing to expose to a network: it binds
//! loopback or it does not bind at all. The port is `0`, so the kernel
//! names it and the caller learns it from `local_addr`; two test targets
//! running in parallel never collide on a fixed port.
//!
//! # One connection, one request
//!
//! Every response carries `Connection: close` and the server closes the
//! socket after it. Hurl opens a connection per entry, so keep-alive
//! would buy nothing and its framing is the one thing in HTTP/1.1 that
//! is easy to get subtly wrong. The cost is one handshake per request on
//! a loopback socket, which is noise next to a replayed case.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// One parsed request: the method, the path without its query, and the
/// body as text. A JSON body is UTF-8 by construction, so the body is
/// text and the transport never guesses an encoding.
pub struct Request {
    /// The method, upper case as the request line carried it.
    pub method: String,
    /// The path without its query.
    pub path: String,
    /// The body, as text; a JSON body is UTF-8 by construction.
    pub body: String,
}

/// One response: a status, and a JSON body.
pub struct Response {
    /// The status code.
    pub status: u16,
    /// The JSON body.
    pub body: String,
}

impl Response {
    /// A JSON response.
    /// A JSON response serialised from a value.
    pub fn json(status: u16, body: serde_json::Value) -> Response {
        Response {
            status,
            // A serialisable value never fails to serialise; a Value is
            // already a tree, so this cannot panic in practice.
            body: serde_json::to_string(&body).unwrap_or_else(|_| "null".to_string()),
        }
    }

    /// A JSON response carrying one named reason, the shape every refusal
    /// takes so a client reads one field whichever way it went wrong.
    /// A JSON response carrying one named reason, the shape every refusal
    /// takes so a client reads one field whichever way it went wrong.
    pub fn reason(status: u16, field: &str, reason: &str) -> Response {
        Response::json(status, serde_json::json!({ field: reason }))
    }
}

/// The reason phrase for a status, so the status line is well formed
/// without carrying a status-line table.
fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Internal Server Error",
    }
}

/// Reads one request: the request line, the headers, and exactly as many
/// body bytes as `Content-Length` names. `Ok(None)` is a closed
/// connection with nothing sent, which is a peer going away rather than a
/// malformed request.
fn read_request(reader: &mut BufReader<TcpStream>) -> std::io::Result<Option<Request>> {
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(None);
    }
    let mut words = line.trim_end().split(' ');
    let method = words.next().unwrap_or_default().to_string();
    // The query is not part of the route: no endpoint takes a parameter
    // by query, so dropping it here cannot silently change a route.
    let target = words.next().unwrap_or_default();
    let path = target.split('?').next().unwrap_or_default().to_string();

    let mut length = 0usize;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            break;
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().unwrap_or(0);
        }
    }

    let mut body = vec![0u8; length];
    reader.read_exact(&mut body)?;
    Ok(Some(Request {
        method,
        path,
        body: String::from_utf8_lossy(&body).into_owned(),
    }))
}

/// Writes one response and closes the socket.
fn write_response(mut stream: TcpStream, response: &Response) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status,
        reason_phrase(response.status),
        response.body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(response.body.as_bytes())?;
    stream.flush()
}

/// Serves connections until `stop` is set, one request each, in arrival
/// order. The loop is single threaded and the routing closure is the only
/// state, so a session begun by one connection is visible to the next:
/// determinism survives the transport, and a case replayed over HTTP
/// lands on the same state it lands on in process.
///
/// # Why the stop flag, and how it is observed
///
/// The routing state is a cluster of replicas, and the harness holds the
/// boot gate's operation log in an `Rc`, so the host is not `Send` and
/// cannot be moved to a server thread. The host therefore runs on the
/// caller's thread and the client moves. `accept` blocks, so the flag
/// alone would only be read after the next connection arrives; the
/// client sets the flag and then opens one connection to wake the
/// listener, which is observed and answered with nothing. No poll, no
/// sleep, no wall-clock wait in the loop.
/// Serves connections on `listener` until `stop` is set, routing each
/// request through `route` in arrival order.
pub fn serve<F>(listener: TcpListener, mut route: F, stop: Arc<AtomicBool>)
where
    F: FnMut(Request) -> Response,
{
    loop {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        let Ok((stream, _)) = listener.accept() else {
            return;
        };
        if stop.load(Ordering::SeqCst) {
            // The wake-up connection. Nothing is owed on it.
            return;
        }
        let Ok(write_half) = stream.try_clone() else {
            continue;
        };
        let mut reader = BufReader::new(stream);
        let response = match read_request(&mut reader) {
            Ok(Some(request)) => route(request),
            Ok(None) => continue,
            Err(_) => Response::reason(400, "error", "the request did not parse"),
        };
        let _ = write_response(write_half, &response);
    }
}

/// One request from the client side, and the response's status and body.
/// The transport's own round trip, used by the gates that assert the
/// served capture equals the in-process capture.
pub fn request(
    address: &str,
    method: &str,
    path: &str,
    body: &str,
) -> std::io::Result<(u16, String)> {
    let mut stream = TcpStream::connect(address)?;
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    stream.flush()?;

    // The server closes after answering, so reading to end of stream is
    // the whole body and needs no content-length arithmetic.
    let mut raw = String::new();
    stream.read_to_string(&mut raw)?;
    let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((raw.as_str(), ""));
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    Ok((status, body.to_string()))
}
