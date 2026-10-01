//! The conformance host binary: the committed compliance corpus served
//! over loopback HTTP (`docs/uvrr-host-compliance.md` §8), for a client
//! such as [`hurl`] to drive as a process that is not this crate.
//!
//! ```text
//! cargo run --features conformance_host --bin uvrr-conformance
//! hurl --test --variable base_url=http://127.0.0.1:8099 tests/hurl
//! ```
//!
//! The port is `8099` on `127.0.0.1` and nothing else. A named `--port`
//! overrides it, because a fixed default is what a client suite can rely
//! on and a flag is what a busy port needs.

use uvrr::conformance::{Host, server};

fn main() {
    let mut port: u16 = 8099;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--port" => {
                port = args
                    .next()
                    .and_then(|value| value.parse().ok())
                    .expect("--port names a port number");
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            other => {
                eprintln!("no such argument: {other}\n");
                print_help();
                std::process::exit(2);
            }
        }
    }

    let listener = std::net::TcpListener::bind(("127.0.0.1", port))
        .unwrap_or_else(|error| panic!("the loopback listener binds on 127.0.0.1:{port}: {error}"));
    eprintln!("uvrr-conformance: serving 127.0.0.1:{port} (loopback only)");
    let mut host = Host::new();
    server::serve(listener, move |request| host.route(request), never_stops());
}

/// A stop flag the process never raises: the binary serves until it is
/// killed, which is how a client suite expects a server to behave.
fn never_stops() -> std::sync::Arc<std::sync::atomic::AtomicBool> {
    std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false))
}

fn print_help() {
    eprintln!(
        "uvrr-conformance: the compliance corpus over loopback HTTP\n\
         \n\
         USAGE:\n\
             uvrr-conformance [--port <n>]\n\
         \n\
         OPTIONS:\n\
             --port <n>   the port to serve on [default: 8099]\n\
             -h, --help   this text"
    );
}
