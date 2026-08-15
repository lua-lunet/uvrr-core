//! The Maelstrom adapter as a black box: the real binary, driven through its
//! stdin/stdout JSON transport exactly as Maelstrom drives it.
//!
//! A scripted transport stands in for Maelstrom itself: the test spawns node
//! processes, relays the lines they emit to the node each line addresses
//! (client replies are collected instead), and injects `init` and lin-kv
//! client operations. What is asserted is the adapter's end of the
//! integration — the init handshake, the bootstrap to a serving primary,
//! replication through the hex-carried binary datagrams, forwarding from a
//! backup to the primary, and the lin-kv reply bodies Knossos later rules
//! on. The nemesis and the linearizability verdict are Maelstrom's own run,
//! not this suite's.
//!
//! These tests need the `maelstrom-lin-kv` binary, which exists only under
//! `--features maelstrom` (the bin target's `required-features`). Without
//! the feature there is nothing to drive, and each test says so and passes
//! vacuously; the verification gate runs `--all-features`, where they are
//! real.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// The binary under test, or `None` when the `maelstrom` feature is off.
fn binary() -> Option<&'static str> {
    option_env!("CARGO_BIN_EXE_maelstrom-lin-kv")
}

/// How long a node may take to answer before the transport is declared
/// broken. The bootstrap fires on the first tick (100 ms), so this is
/// generous.
const REPLY_TIMEOUT: Duration = Duration::from_secs(15);

/// A spawned node process. Lines for the node go to `to_node`; lines the
/// node emits arrive parsed on `out`, which a relay may take over.
struct Node {
    child: Child,
    to_node: Sender<String>,
    out: Option<Receiver<Value>>,
}

impl Node {
    // `Drop` kills and waits the child; the 1.85 zombie-process lint does
    // not see through the struct escape, so it is silenced at the spawn site.
    #[allow(clippy::zombie_processes)]
    fn spawn() -> Node {
        let mut child = Command::new(binary().expect("spawn called with the binary present"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the adapter binary spawns");
        let mut stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        let (to_node, inbound) = channel::<String>();
        std::thread::spawn(move || {
            while let Ok(line) = inbound.recv() {
                if writeln!(stdin, "{line}")
                    .and_then(|()| stdin.flush())
                    .is_err()
                {
                    return;
                }
            }
        });
        let (emitted, out) = channel::<Value>();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                let Ok(value) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if emitted.send(value).is_err() {
                    return;
                }
            }
        });
        Node {
            child,
            to_node,
            out: Some(out),
        }
    }

    fn send(&self, src: &str, dest: &str, body: Value) {
        let line = json!({"src": src, "dest": dest, "body": body});
        self.to_node
            .send(line.to_string())
            .expect("the node's writer thread lives");
    }

    /// The next line the node emitted within the reply timeout.
    fn recv(&self) -> Value {
        self.out
            .as_ref()
            .expect("the node keeps its output unless a relay took it")
            .recv_timeout(REPLY_TIMEOUT)
            .expect("the node answers within the reply timeout")
    }

    /// Hands the node's emitted-line stream to a relay thread.
    fn take_out(&mut self) -> Receiver<Value> {
        self.out.take().expect("the output is taken once")
    }

    fn init(&self, node_id: &str, node_ids: &[&str]) {
        self.send(
            node_id,
            node_id,
            json!({
                "type": "init",
                "msg_id": 1,
                "node_id": node_id,
                "node_ids": node_ids,
            }),
        );
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Asserts the shape of a reply body — `in_reply_to` and the expected
/// `type` — and returns it for further field assertions.
fn assert_reply(reply: &Value, in_reply_to: u64, kind: &str) -> Value {
    let body = reply.get("body").cloned().expect("a reply carries a body");
    assert_eq!(
        body.get("in_reply_to").and_then(Value::as_u64),
        Some(in_reply_to),
        "the reply names the request it answers: {body}"
    );
    assert_eq!(
        body.get("type").and_then(Value::as_str),
        Some(kind),
        "the reply type: {body}"
    );
    body
}

/// The scripted transport for a small cluster: relays each node's emitted
/// lines to the addressee, collecting client replies for assertion.
struct RelayedCluster {
    collected: Receiver<Value>,
    deadline: Instant,
}

impl RelayedCluster {
    /// Starts the relay threads over the nodes' emitted-line streams. Node
    /// ids are `n0`..`n{k-1}` in slice order.
    fn start(nodes: &mut [&mut Node]) -> RelayedCluster {
        let (client_replies, collected) = channel::<Value>();
        let relays: Vec<(String, Receiver<Value>)> = nodes
            .iter_mut()
            .enumerate()
            .map(|(index, node)| (format!("n{index}"), node.take_out()))
            .collect();
        let senders: Vec<(String, Sender<String>)> = nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (format!("n{index}"), node.to_node.clone()))
            .collect();
        for (_, out) in relays {
            let senders = senders.clone();
            let clients = client_replies.clone();
            std::thread::spawn(move || {
                while let Ok(message) = out.recv() {
                    let dest = message
                        .get("dest")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    if dest.starts_with('c') {
                        let _ = clients.send(message);
                    } else if let Some((_, to_node)) =
                        senders.iter().find(|(label, _)| label == dest)
                    {
                        // Peer traffic carries the hex-encoded binary
                        // datagrams.
                        let _ = to_node.send(message.to_string());
                    }
                }
            });
        }
        RelayedCluster {
            collected,
            deadline: Instant::now() + REPLY_TIMEOUT,
        }
    }

    /// The next client reply within the deadline.
    fn recv_client(&self) -> Value {
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        assert!(remaining > Duration::ZERO, "the cluster served in time");
        self.collected
            .recv_timeout(remaining)
            .expect("the cluster answers within the deadline")
    }
}

#[test]
fn init_handshake_answers_init_ok() {
    if binary().is_none() {
        eprintln!("maelstrom feature off; no adapter binary to drive");
        return;
    }
    let node = Node::spawn();
    node.init("n0", &["n0"]);
    assert_reply(&node.recv(), 1, "init_ok");
}

/// A two-node cluster with the client addressing the primary directly: the
/// full propose/PrepareOk/commit/apply/answer pipeline serves write, read,
/// compare-and-set, and the definite-failure replies. (One node is not a
/// serving configuration of the core: the commit cascade is evaluated when
/// a `PrepareOk` lands, and a backup-less cluster never receives one.)
#[test]
fn cluster_serves_kv_operations() {
    if binary().is_none() {
        eprintln!("maelstrom feature off; no adapter binary to drive");
        return;
    }
    let mut n0 = Node::spawn();
    let mut n1 = Node::spawn();
    n0.init("n0", &["n0", "n1"]);
    n1.init("n1", &["n0", "n1"]);
    let cluster = RelayedCluster::start(&mut [&mut n0, &mut n1]);
    // The bootstrap promotion lands on the first tick after init; proposing
    // before it would draw a definite "not the primary" refusal, which is
    // not what this test is about.
    std::thread::sleep(Duration::from_millis(500));

    n0.send(
        "c1",
        "n0",
        json!({"type": "write", "msg_id": 2, "key": 7, "value": 42}),
    );
    assert_reply(&cluster.recv_client(), 2, "write_ok");

    n0.send("c1", "n0", json!({"type": "read", "msg_id": 3, "key": 7}));
    let body = assert_reply(&cluster.recv_client(), 3, "read_ok");
    assert_eq!(body.get("value"), Some(&Value::from(42)));

    n0.send(
        "c1",
        "n0",
        json!({"type": "cas", "msg_id": 4, "key": 7, "from": 42, "to": 5}),
    );
    assert_reply(&cluster.recv_client(), 4, "cas_ok");

    n0.send("c1", "n0", json!({"type": "read", "msg_id": 5, "key": 7}));
    let body = assert_reply(&cluster.recv_client(), 5, "read_ok");
    assert_eq!(body.get("value"), Some(&Value::from(5)));

    n0.send("c1", "n0", json!({"type": "read", "msg_id": 6, "key": 99}));
    let body = assert_reply(&cluster.recv_client(), 6, "error");
    assert_eq!(body.get("code").and_then(Value::as_u64), Some(20));
}

/// Two nodes over the scripted transport: the client addresses the backup,
/// which forwards to the primary; the write replicates through the
/// hex-carried binary datagrams (bootstrap adoption, Prepare/PrepareOk,
/// commit), and the reply hops back through the forwarding node to the
/// client.
#[test]
fn two_node_cluster_replicates_a_forwarded_write() {
    if binary().is_none() {
        eprintln!("maelstrom feature off; no adapter binary to drive");
        return;
    }
    let mut n0 = Node::spawn();
    let mut n1 = Node::spawn();
    n0.init("n0", &["n0", "n1"]);
    n1.init("n1", &["n0", "n1"]);
    let cluster = RelayedCluster::start(&mut [&mut n0, &mut n1]);

    // The bootstrap: n0 promotes on its first tick and announces the view;
    // n1 adopts from the announcement. Then the forwarded write.
    std::thread::sleep(Duration::from_millis(500));
    n1.send(
        "c1",
        "n1",
        json!({"type": "write", "msg_id": 2, "key": 3, "value": 9}),
    );
    let reply = cluster.recv_client();
    assert_eq!(reply.get("dest").and_then(Value::as_str), Some("c1"));
    assert_reply(&reply, 2, "write_ok");

    n0.send("c2", "n0", json!({"type": "read", "msg_id": 7, "key": 3}));
    let reply = cluster.recv_client();
    assert_eq!(reply.get("dest").and_then(Value::as_str), Some("c2"));
    let body = assert_reply(&reply, 7, "read_ok");
    assert_eq!(body.get("value"), Some(&Value::from(9)));
}
