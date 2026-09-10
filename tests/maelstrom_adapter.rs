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
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
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

/// How long a whole scripted cluster may keep serving client traffic.
/// The membership lifecycle serializes its verbs through the
/// stop-the-world gate (§8.7.8), so each verb polls through the era
/// transition in flight — roughly `PRIMARY_TIMEOUT_TICKS` ticks per verb.
const CLUSTER_TIMEOUT: Duration = Duration::from_secs(90);

/// A spawned node process. Lines for the node go to `to_node`; lines the
/// node emits arrive parsed on `out`, which a relay may take over. Stderr
/// is either discarded (the default lanes) or relayed onto a channel the
/// test polls for the host's named lifecycle diagnostics.
struct Node {
    child: Child,
    to_node: Option<Sender<String>>,
    out: Option<Receiver<Value>>,
    stderr: Option<Receiver<String>>,
}

impl Node {
    // `Drop` kills and waits the child; the 1.85 zombie-process lint does
    // not see through the struct escape, so it is silenced at the spawn site.
    #[allow(clippy::zombie_processes)]
    fn spawn() -> Node {
        Node::spawn_with(None)
    }

    // `Drop` kills and waits the child; see `spawn`.
    #[allow(clippy::zombie_processes)]
    fn spawn_with(state_dir: Option<&str>) -> Node {
        let mut command = Command::new(binary().expect("spawn called with the binary present"));
        command.stdin(Stdio::piped()).stdout(Stdio::piped());
        match state_dir {
            Some(dir) => {
                command.env("MAELSTROM_VRR_STATE_DIR", dir);
                command.stderr(Stdio::piped());
            }
            None => {
                command.stderr(Stdio::null());
            }
        }
        let mut child = command.spawn().expect("the adapter binary spawns");
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
        let stderr = state_dir.map(|_| {
            let pipe = child.stderr.take().expect("piped stderr");
            let (lines, stderr) = channel::<String>();
            std::thread::spawn(move || {
                for line in BufReader::new(pipe).lines() {
                    let Ok(line) = line else { break };
                    if lines.send(line).is_err() {
                        return;
                    }
                }
            });
            stderr
        });
        Node {
            child,
            to_node: Some(to_node),
            out: Some(out),
            stderr,
        }
    }

    /// A kill the way the nemesis does it: SIGKILL, stdin closed, the
    /// process reaped — the volatile state is lost, the state dir is not.
    fn kill(&mut self) {
        self.to_node = None;
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    /// A clean shutdown: stdin closes, the node's EOF arm writes the
    /// flushed checkpoint, and the process exits on its own. Panics when
    /// the exit is not clean or does not come.
    fn eof(self) {
        let mut node = self;
        node.to_node = None;
        let deadline = Instant::now() + REPLY_TIMEOUT;
        while Instant::now() < deadline {
            match node.child.try_wait() {
                Ok(Some(status)) => {
                    assert!(
                        status.success(),
                        "the node exited cleanly on stdin EOF: {status}"
                    );
                    return;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(reason) => panic!("the node's exit status is unreadable: {reason}"),
            }
        }
        let _ = node.child.kill();
        let _ = node.child.wait();
        panic!("the node did not exit on stdin EOF");
    }

    /// The first stderr line containing `needle`, within the reply
    /// timeout. The host's lifecycle diagnostics are the test-visible
    /// evidence of how an init decided (provision / clean reopen / dirty
    /// bump).
    fn stderr_containing(&self, needle: &str) -> String {
        let stderr = self.stderr.as_ref().expect("this node pipes stderr");
        let deadline = Instant::now() + REPLY_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(remaining > Duration::ZERO, "stderr never said {needle:?}");
            match stderr.recv_timeout(remaining) {
                Ok(line) if line.contains(needle) => return line,
                Ok(_) => {}
                Err(reason) => panic!("stderr never said {needle:?}: {reason}"),
            }
        }
    }

    /// Waits for the process to exit and returns its status. The refusal
    /// path exits nonzero right after answering `error`.
    fn wait_exit(&mut self, timeout: Duration) -> std::process::ExitStatus {
        let deadline = Instant::now() + timeout;
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => return status,
                Ok(None) => {
                    assert!(Instant::now() < deadline, "the node did not exit in time");
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(reason) => panic!("the node's exit status is unreadable: {reason}"),
            }
        }
    }

    fn send(&self, src: &str, dest: &str, body: Value) {
        let line = json!({"src": src, "dest": dest, "body": body});
        self.to_node
            .as_ref()
            .expect("the node accepts input until it is killed")
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
        // Closing stdin first gives a live node its clean EOF path (§2's
        // flushed checkpoint); a dead process cannot take it, and the kill
        // below reaps whatever remains.
        self.to_node = None;
        for _ in 0..20 {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
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

/// One peer's input channel: the live end of a spawned node's stdin, and
/// the label the relay routes peer traffic by.
type PeerSender = (String, Sender<String>);

/// The scripted transport for a small cluster: relays each node's emitted
/// lines to the addressee, collecting client replies for assertion. A node
/// the nemesis killed rejoins the transport through [`RelayedCluster::relay`]
/// under the same label, exactly as Maelstrom re-attaches a restarted node.
struct RelayedCluster {
    collected: Receiver<Value>,
    collector: Sender<Value>,
    senders: Arc<Mutex<Vec<PeerSender>>>,
    deadline: Instant,
}

impl RelayedCluster {
    /// Starts the relay threads over the nodes' emitted-line streams. Node
    /// ids are `n0`..`n{k-1}` in slice order.
    fn start(nodes: &mut [&mut Node]) -> RelayedCluster {
        let (collector, collected) = channel::<Value>();
        let senders = Arc::new(Mutex::new(
            nodes
                .iter()
                .enumerate()
                .map(|(index, node)| {
                    (
                        format!("n{index}"),
                        node.to_node.clone().expect("a live node accepts input"),
                    )
                })
                .collect::<Vec<_>>(),
        ));
        let cluster = RelayedCluster {
            collected,
            collector,
            senders,
            deadline: Instant::now() + CLUSTER_TIMEOUT,
        };
        for (index, node) in nodes.iter_mut().enumerate() {
            cluster.relay(&format!("n{index}"), node);
        }
        cluster
    }

    /// One relay thread for a (re)spawned node: its emitted lines go to
    /// the addressee, and client-addressed lines join the shared
    /// collector. Peer traffic carries the hex-encoded binary datagrams.
    fn relay(&self, label: &str, node: &mut Node) {
        let out = node.take_out();
        let to_node = node.to_node.clone().expect("a live node accepts input");
        let senders = Arc::clone(&self.senders);
        let clients = self.collector.clone();
        let mut map = senders.lock().expect("the peer map is not poisoned");
        map.retain(|(peer, _)| peer != label);
        map.push((label.to_owned(), to_node));
        drop(map);
        std::thread::spawn(move || {
            while let Ok(message) = out.recv() {
                let dest = message
                    .get("dest")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if dest.starts_with('c') {
                    let _ = clients.send(message);
                } else {
                    let sender = senders
                        .lock()
                        .expect("the peer map is not poisoned")
                        .iter()
                        .find(|(peer, _)| peer == dest)
                        .map(|(_, sender)| sender.clone());
                    if let Some(sender) = sender {
                        let _ = sender.send(message.to_string());
                    }
                }
            }
        });
    }

    /// Removes a node from the peer map, so its stdin closes (the relay's
    /// cloned sender was the last one holding it open) and the process
    /// sees EOF — the clean-shutdown path's trigger.
    fn detach(&self, label: &str) {
        self.senders
            .lock()
            .expect("the peer map is not poisoned")
            .retain(|(peer, _)| peer != label);
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

/// The host's membership pre-gate runs before the core is touched: a join
/// naming an id outside the bench roster (fixed by `node_ids`) is a
/// definite malformed request, answered with Maelstrom's own
/// `malformed-request` code — the core is never touched.
#[test]
fn join_refuses_an_id_outside_the_bench_roster() {
    if binary().is_none() {
        eprintln!("maelstrom feature off; no adapter binary to drive");
        return;
    }
    let node = Node::spawn();
    node.init("n0", &["n0"]);
    assert_reply(&node.recv(), 1, "init_ok");
    node.send(
        "c1",
        "n0",
        json!({"type": "join", "msg_id": 2, "node_id": "n9"}),
    );
    let body = assert_reply(&node.recv(), 2, "error");
    assert_eq!(
        body.get("code").and_then(Value::as_u64),
        Some(12),
        "the refusal is the roster gate, not the core: {body}"
    );
    // Every membership reply echoes the node's configuration view.
    assert!(
        body.get("config").and_then(Value::as_object).is_some(),
        "the refusal echoes the configuration view: {body}"
    );
}

/// The member order and weights of an echoed configuration view.
fn members_of(body: &Value) -> Vec<(String, u64)> {
    body.get("config")
        .and_then(|config| config.get("members"))
        .and_then(Value::as_array)
        .expect("every membership reply echoes a configuration view")
        .iter()
        .map(|member| {
            (
                member
                    .get("node")
                    .and_then(Value::as_str)
                    .expect("a member names its node")
                    .to_string(),
                member
                    .get("weight")
                    .and_then(Value::as_u64)
                    .expect("a member carries its weight"),
            )
        })
        .collect()
}

/// One membership RPC, retried until the cluster answers `*_ok`. A
/// refusal is definite — the core's gates ran before the proposal, the
/// operation never entered the log — so a retry after one is honest, and
/// it is also how the stop-the-world gate (§8.7.4, §8.7.8: one era
/// transition outstanding at a time, the next verb waits for the view
/// change into the established era) is driven. An unanswered verb whose
/// establishing operation died uncommitted is the honest indeterminate;
/// the deadline asserts only the healthy path.
fn membership_ok(
    cluster: &RelayedCluster,
    node: &Node,
    client: &str,
    msg_id: &mut u64,
    kind: &str,
    target: &str,
) -> Value {
    let deadline = Instant::now() + CLUSTER_TIMEOUT;
    loop {
        assert!(
            Instant::now() < deadline,
            "{kind} {target} committed within the cluster timeout"
        );
        *msg_id += 1;
        let request = *msg_id;
        node.send(
            client,
            "n0",
            json!({"type": kind, "msg_id": request, "node_id": target}),
        );
        let body = loop {
            let reply = cluster.recv_client();
            let seen = reply
                .get("body")
                .and_then(|body| body.get("in_reply_to"))
                .and_then(Value::as_u64);
            if seen == Some(request) {
                break reply.get("body").cloned().expect("a reply carries a body");
            }
        };
        if body.get("type").and_then(Value::as_str) == Some(&format!("{kind}_ok")) {
            return body;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// The full membership lifecycle over the scripted transport: demote a
/// voter to a learner, leave it out of the configuration entirely, join
/// it back at weight 0, and promote it to a voter again. Every `*_ok`
/// reply echoes the answering primary's current configuration view, and
/// the echoes are the membership-convergence evidence the checker later
/// rules on.
#[test]
fn membership_verbs_make_and_unmake_a_member() {
    if binary().is_none() {
        eprintln!("maelstrom feature off; no adapter binary to drive");
        return;
    }
    let mut n0 = Node::spawn();
    let mut n1 = Node::spawn();
    let mut n2 = Node::spawn();
    n0.init("n0", &["n0", "n1", "n2"]);
    n1.init("n1", &["n0", "n1", "n2"]);
    n2.init("n2", &["n0", "n1", "n2"]);
    let cluster = RelayedCluster::start(&mut [&mut n0, &mut n1, &mut n2]);
    std::thread::sleep(Duration::from_millis(500));

    let mut msg_id = 1u64;
    let demoted = membership_ok(&cluster, &n0, "c1", &mut msg_id, "demote", "n2");
    assert!(
        members_of(&demoted).contains(&("n2".into(), 0)),
        "the demoted member is a learner: {demoted}"
    );
    let left = membership_ok(&cluster, &n0, "c1", &mut msg_id, "leave", "n2");
    assert!(
        !members_of(&left).iter().any(|(name, _)| name == "n2"),
        "the left member is out of the configuration: {left}"
    );
    let joined = membership_ok(&cluster, &n0, "c1", &mut msg_id, "join", "n2");
    assert_eq!(
        members_of(&joined),
        vec![("n0".into(), 1), ("n1".into(), 1), ("n2".into(), 0)],
        "the join appends the learner: {joined}"
    );
    let promoted = membership_ok(&cluster, &n0, "c1", &mut msg_id, "promote", "n2");
    assert!(
        members_of(&promoted).contains(&("n2".into(), 1)),
        "the promoted member votes again: {promoted}"
    );
}

// ---------------------------------------------------------------------------
// Persistence: the state dir, the write-through barrier, and the restart
// decision (§2 of docs/uvrr-reincarnation.md). A kill-restart must become
// an honest `Node::reopen` — the bumped identity re-enters through the
// core's reincarnation machinery — and a fresh or cleanly shut-down node
// must behave exactly as its §2 classification says.
// ---------------------------------------------------------------------------

/// A fresh, uniquely named state directory for one test.
fn state_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("uvrr-maelstrom-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the state dir creates");
    dir
}

/// The configuration view a definite membership refusal echoes, polled
/// until it names exactly `wanted`. `join` of a member the configuration
/// already holds never enters the log, so the reply is immediate and
/// carries the answering node's current view — the convergence evidence
/// of a reincarnation cycle, read through the public surface.
fn converged_members(
    cluster: &RelayedCluster,
    node: &Node,
    wanted: Vec<(String, u64)>,
) -> Vec<(String, u64)> {
    let deadline = Instant::now() + CLUSTER_TIMEOUT;
    let mut msg_id = 10_000u64;
    loop {
        assert!(
            Instant::now() < deadline,
            "the configuration converged to {wanted:?} in time"
        );
        msg_id += 1;
        let request = msg_id;
        node.send(
            "c9",
            "n0",
            json!({"type": "join", "msg_id": request, "node_id": "n1"}),
        );
        let reply = cluster.recv_client();
        let body = reply.get("body").cloned().expect("a reply carries a body");
        if body.get("in_reply_to").and_then(Value::as_u64) != Some(request) {
            continue;
        }
        if body.get("type").and_then(Value::as_str) != Some("error") {
            continue;
        }
        if let Some(config) = body.get("config").cloned() {
            let members = members_of(&json!({"config": config}));
            if members == wanted {
                return members;
            }
        }
    }
}

/// One lin-kv client op retried until the cluster answers `ok_type`. A
/// refusal is definite — the operation provably never entered the log —
/// so a retry after one is honest, and it is also how a reopener still
/// catching up (fenced, or naming a stale view) is driven. The deadline
/// asserts only the healthy path.
fn client_ok(
    cluster: &RelayedCluster,
    node: &Node,
    client: &str,
    target: &str,
    kind: &str,
    fields: Value,
) -> Value {
    let deadline = Instant::now() + CLUSTER_TIMEOUT;
    let mut msg_id = 20_000u64;
    loop {
        assert!(
            Instant::now() < deadline,
            "{kind} answered within the cluster timeout"
        );
        msg_id += 1;
        let request = msg_id;
        let mut body = json!({"type": kind, "msg_id": request});
        if let (Some(target), Some(extra)) = (body.as_object_mut(), fields.as_object()) {
            for (name, value) in extra {
                target.insert(name.clone(), value.clone());
            }
        }
        node.send(client, target, body);
        let body = loop {
            let reply = cluster.recv_client();
            let seen = reply
                .get("body")
                .and_then(|body| body.get("in_reply_to"))
                .and_then(Value::as_u64);
            if seen == Some(request) {
                break reply.get("body").cloned().expect("a reply carries a body");
            }
        };
        if body.get("type").and_then(Value::as_str) == Some(&format!("{kind}_ok")) {
            return body;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// Three nodes, a fresh state dir, a committed write, and then the nemesis:
/// `n2` is killed mid-life and respawned against the SAME state dir. The
/// restart must be a dirty `Node::reopen` (the stderr diagnostic names the
/// bump — reopen, not provision), the core's reincarnation machinery must
/// walk the forced sequence until the bumped identity is a voter again
/// (the announcement, the forced batches, the era transitions between
/// them), the superseded identity must be gone, and the committed history
/// must serve through the reopener.
#[test]
fn committed_state_survives_a_kill_restart_with_the_same_state_dir() {
    if binary().is_none() {
        eprintln!("maelstrom feature off; no adapter binary to drive");
        return;
    }
    let dir = state_dir("kill-restart");
    let mut n0 = Node::spawn_with(Some(dir.to_str().expect("a utf-8 path")));
    let mut n1 = Node::spawn_with(Some(&dir.to_string_lossy()));
    let mut n2 = Node::spawn_with(Some(&dir.to_string_lossy()));
    n0.init("n0", &["n0", "n1", "n2"]);
    n1.init("n1", &["n0", "n1", "n2"]);
    n2.init("n2", &["n0", "n1", "n2"]);
    n0.stderr_containing("provisions a fresh identity");
    n1.stderr_containing("provisions a fresh identity");
    n2.stderr_containing("provisions a fresh identity");
    let cluster = RelayedCluster::start(&mut [&mut n0, &mut n1, &mut n2]);
    std::thread::sleep(Duration::from_millis(500));

    // The write commits (quorum 2 of 3) and is answered — from this
    // moment it is durable in every node's state file, written through
    // before the reply left the proposing node.
    n0.send(
        "c1",
        "n0",
        json!({"type": "write", "msg_id": 2, "key": 7, "value": 42}),
    );
    assert_reply(&cluster.recv_client(), 2, "write_ok");
    n1.send("c1", "n1", json!({"type": "read", "msg_id": 3, "key": 7}));
    assert_reply(&cluster.recv_client(), 3, "read_ok");

    // The kill: n2 dies mid-life, the way the nemesis kills it.
    n2.kill();

    // The cluster keeps serving inside its fault bound (2 of 3 survive).
    n0.send(
        "c1",
        "n0",
        json!({"type": "write", "msg_id": 4, "key": 8, "value": 64}),
    );
    assert_reply(&cluster.recv_client(), 4, "write_ok");

    // The restart against the same state dir. The node was operating when
    // it died, so the file holds the running sentinel: the dirty path.
    // The stderr diagnostic is the test-visible reopen evidence — the
    // bumped identity, announced, the forced sequence to follow.
    let mut n2 = Node::spawn_with(Some(&dir.to_string_lossy()));
    n2.init("n2", &["n0", "n1", "n2"]);
    let diagnostic = n2.stderr_containing("reopens dirty");
    cluster.relay("n2", &mut n2);
    assert!(
        !diagnostic.contains("provisions"),
        "a kill-restart with a state dir reopens, it does not provision: {diagnostic}"
    );

    // The reincarnation completes on ticks: the forced batches commit one
    // era each, the era transitions ride the suspicion timeout, and the
    // configuration ends at three voters with the bumped identity in
    // n2's seat and the superseded identity evicted.
    let members = converged_members(
        &cluster,
        &n0,
        vec![("n0".into(), 1), ("n1".into(), 1), ("n2".into(), 1)],
    );
    assert_eq!(
        members,
        vec![("n0".into(), 1), ("n1".into(), 1), ("n2".into(), 1)],
        "the reincarnated identity votes again"
    );

    // The committed history serves through the reopener, and the lane
    // accepts fresh traffic through it: the re-established member
    // participates. The reopener catches up (its own view may trail the
    // eras the forced sequence committed), so the ops retry past the
    // definite refusals its stale view produces.
    let body = client_ok(&cluster, &n2, "c2", "n2", "read", json!({"key": 7}));
    assert_eq!(
        body.get("value"),
        Some(&Value::from(42)),
        "the write committed before the kill survives the restart: {body}"
    );
    client_ok(
        &cluster,
        &n2,
        "c2",
        "n2",
        "write",
        json!({"key": 7, "value": 43}),
    );
}

/// A state dir that holds no state file for the node: the first life. The
/// provision path is exactly the unpersisted host's — the same fenced
/// `Recovering` start, the same bootstrap adoption, the same serving —
/// and the lifecycle diagnostic names it.
#[test]
fn a_fresh_state_dir_provisions_as_today() {
    if binary().is_none() {
        eprintln!("maelstrom feature off; no adapter binary to drive");
        return;
    }
    let dir = state_dir("fresh");
    let mut n0 = Node::spawn_with(Some(&dir.to_string_lossy()));
    let mut n1 = Node::spawn_with(Some(&dir.to_string_lossy()));
    n0.init("n0", &["n0", "n1"]);
    n1.init("n1", &["n0", "n1"]);
    n0.stderr_containing("provisions a fresh identity");
    n1.stderr_containing("provisions a fresh identity");
    let cluster = RelayedCluster::start(&mut [&mut n0, &mut n1]);
    std::thread::sleep(Duration::from_millis(500));
    client_ok(
        &cluster,
        &n0,
        "c1",
        "n0",
        "write",
        json!({"key": 3, "value": 9}),
    );
    let body = client_ok(&cluster, &n1, "c1", "n1", "read", json!({"key": 3}));
    assert_eq!(
        body.get("value"),
        Some(&Value::from(9)),
        "the fresh lane serves the value it wrote: {body}"
    );
}

/// A clean shutdown (stdin EOF) writes the flushed checkpoint: the next
/// init under the same state dir reopens CLEANLY, under the same identity
/// — the §2 clean path, no bump, no announcement. The committed history
/// survives, and the reopener rejoins the live cluster's serving.
#[test]
fn a_clean_shutoff_reopens_under_the_same_identity() {
    if binary().is_none() {
        eprintln!("maelstrom feature off; no adapter binary to drive");
        return;
    }
    let dir = state_dir("clean-restart");
    let mut n0 = Node::spawn_with(Some(&dir.to_string_lossy()));
    let mut n1 = Node::spawn_with(Some(&dir.to_string_lossy()));
    let mut n2 = Node::spawn_with(Some(&dir.to_string_lossy()));
    n0.init("n0", &["n0", "n1", "n2"]);
    n1.init("n1", &["n0", "n1", "n2"]);
    n2.init("n2", &["n0", "n1", "n2"]);
    n0.stderr_containing("provisions a fresh identity");
    n1.stderr_containing("provisions a fresh identity");
    n2.stderr_containing("provisions a fresh identity");
    let cluster = RelayedCluster::start(&mut [&mut n0, &mut n1, &mut n2]);
    std::thread::sleep(Duration::from_millis(500));
    n0.send(
        "c1",
        "n0",
        json!({"type": "write", "msg_id": 2, "key": 5, "value": 7}),
    );
    assert_reply(&cluster.recv_client(), 2, "write_ok");

    // The clean shutoff: stdin closes (the relay's sender is detached
    // first — it was the last one holding the pipe open), the node's EOF
    // arm flushes the checkpoint, and the process exits on its own.
    cluster.detach("n2");
    n2.eof();

    // The respawn reopens the flushed state under the same identity — the
    // clean path, no bump — and rejoins the live cluster's serving view.
    let mut n2 = Node::spawn_with(Some(&dir.to_string_lossy()));
    n2.init("n2", &["n0", "n1", "n2"]);
    let diagnostic = n2.stderr_containing("reopens cleanly");
    cluster.relay("n2", &mut n2);
    assert!(
        !diagnostic.contains("bumps"),
        "a flushed state file continues under the same identity: {diagnostic}"
    );
    let body = client_ok(&cluster, &n2, "c2", "n2", "read", json!({"key": 5}));
    assert_eq!(
        body.get("value"),
        Some(&Value::from(7)),
        "the flushed history serves the committed read: {body}"
    );
}

/// A torn state file is a crash artifact, not an input: the host refuses
/// the reopen by name, answers `error`, and exits nonzero so Jepsen
/// restarts the node — never a panic.
#[test]
fn a_corrupt_state_file_refuses_to_start() {
    if binary().is_none() {
        eprintln!("maelstrom feature off; no adapter binary to drive");
        return;
    }
    let dir = state_dir("corrupt");
    let n0 = Node::spawn_with(Some(&dir.to_string_lossy()));
    n0.init("n0", &["n0"]);
    assert_reply(&n0.recv(), 1, "init_ok");
    let file = dir.join("n0.uvrr-state");
    let bytes = std::fs::read(&file).expect("the state file is written at init");
    std::fs::write(&file, &bytes[..bytes.len() / 2]).expect("the file truncates");

    // The respawn against the torn file: the named refusal, then the
    // nonzero exit.
    let mut n0 = Node::spawn_with(Some(&dir.to_string_lossy()));
    n0.init("n0", &["n0"]);
    let reply = n0.recv();
    let body = assert_reply(&reply, 1, "error");
    assert_eq!(
        body.get("code").and_then(Value::as_u64),
        Some(11),
        "the refusal is a definite error, not a crash: {body}"
    );
    let status = n0.wait_exit(REPLY_TIMEOUT);
    assert!(
        !status.success(),
        "the refused node exits nonzero: {status}"
    );
}
