//! The bench host's state file: the durable evidence a restart reopens
//! under (`docs/uvrr-reincarnation.md` §2), carried in the host's own
//! store.
//!
//! One file per node under the state dir, keyed by the node's Maelstrom
//! id: the four-superblock copies of §2, the genesis roster, the §5
//! persisted progress record, and the journal's retained history. The
//! format is bench-internal (no cross-version compatibility), and its
//! journal content uses the core's own wire codec — [`LogEntry`] packs
//! and unpacks through `vrr::wire`, so the host invents no second codec
//! for history it did not define. The rest of the record is host-owned
//! state the core hands back verbatim at [`vrr::replica::Replica::reopen`],
//! encoded with the core's scalar codec and stated word tables (the
//! core's `Status::to_word` convention: a `match`, never a cast).
//!
//! Write-through: the file is rewritten — temp file, fsync, atomic
//! rename, fsync of the directory, the §7 `Forced` barrier shape — after
//! every published transition and before any released effect is routed,
//! so a kill loses at most the in-flight input. A torn or corrupt file is
//! a crash artifact and is answered with the core's reopen refusal path —
//! a named refusal and a nonzero exit, never a panic. Absence is the
//! provisioning case: no file, first life.

use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use vrr::ids::{Era, Fault, Slot, View, ViewId};
use vrr::journal::{JournalView, LogEntry, RangeOutcome};
use vrr::progress::Status;
use vrr::replica::{Incarnation, Marker, PersistedProgress, SuperblockCopies};
use vrr::wire::{Pack, PackWriter, Unpack, UnpackCursor, UnpackError};

/// File magic, as a big-endian word. A file not opening with it is not a
/// state file.
const MAGIC: u32 = u32::from_be_bytes(*b"UVRS");
/// Format version. The format is bench-internal: an unknown version is a
/// refusal, not a compatibility surface.
const VERSION: u32 = 1;

/// Marker words, stated as a table so reordering this source cannot
/// renumber the file (the core's `Status::to_word` discipline).
const MARKER_FLUSHED: u8 = 0;
const MARKER_UNFLUSHED: u8 = 1;

/// Fault-word table, same discipline. `0` is "no fault"; the variants are
/// numbered in declaration order of [`vrr::ids::Fault`].
const FAULT_NONE: u8 = 0;

/// One node's persistence home: the directory the state file lives in.
pub struct Store {
    path: PathBuf,
}

impl Store {
    /// The per-node file path under `dir`, keyed by the node's Maelstrom id.
    ///
    /// # Errors
    ///
    /// The directory could not be created — the persistence home is
    /// unusable, and the node must not claim durability it cannot deliver.
    pub fn open(dir: &Path, node: &str) -> io::Result<Store> {
        std::fs::create_dir_all(dir)?;
        Ok(Store {
            path: dir.join(format!("{node}.uvrr-state")),
        })
    }

    /// Writes the full durable snapshot: temp file, fsync, atomic rename,
    /// fsync of the directory. The rename is part of the barrier — a
    /// directory entry that never reaches disk loses the file that carries
    /// the data.
    ///
    /// # Errors
    ///
    /// Any storage failure. A failed temp write or rename leaves the
    /// previous file content in place (or the temp file aside, unread by
    /// any loader), so the durable evidence never names a state the node
    /// did not reach.
    pub fn write(&self, state: &NodeState) -> io::Result<()> {
        let bytes = encode(state);
        let temp = self.path.with_extension("uvrr-tmp");
        {
            let mut file = File::create(&temp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
        }
        std::fs::rename(&temp, &self.path)?;
        if let Some(parent) = self.path.parent() {
            File::open(parent)?.sync_all()?;
        }
        Ok(())
    }

    /// Reads the node's state file. Any decode failure — magic, version,
    /// short read, malformed field, trailing bytes — is a named refusal:
    /// the file is evidence the node cannot vouch for.
    ///
    /// # Errors
    ///
    /// [`StoreError::Absent`] when there is no file (first life),
    /// [`StoreError::Corrupt`] for every other reason.
    pub fn load(&self) -> Result<NodeState, StoreError> {
        let bytes = match std::fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(StoreError::Absent);
            }
            Err(error) => return Err(StoreError::Corrupt(format!("unreadable: {error}"))),
        };
        decode(&bytes).map_err(|error| StoreError::Corrupt(error.to_string()))
    }
}

/// Why the reopen decision could not read the durable evidence.
#[derive(Debug)]
pub enum StoreError {
    /// No state file: a first life. The host provisions.
    Absent,
    /// The file exists but is not a self-consistent record — a crash
    /// artifact, bit rot, or an unreadable path. The core's reopen
    /// refusal path, never a panic.
    Corrupt(String),
}

/// A decode failure: the core codec's own error, or a stated refusal.
#[derive(Debug)]
enum DecodeError {
    Wire(UnpackError),
    Refusal(String),
}

impl From<UnpackError> for DecodeError {
    fn from(error: UnpackError) -> DecodeError {
        DecodeError::Wire(error)
    }
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DecodeError::Wire(error) => write!(f, "{error:?}"),
            DecodeError::Refusal(reason) => write!(f, "{reason}"),
        }
    }
}

/// What a node's state file holds: everything `Node::reopen` needs, plus
/// the §2 restart model.
pub struct NodeState {
    /// The four-superblock copies (§2): the restart classification and
    /// the incarnation.
    pub copies: SuperblockCopies,
    /// The sorted genesis roster the journal's `Init` folds; the file's
    /// roster must equal the init's, or the durable evidence names a
    /// different cluster.
    pub roster: Vec<String>,
    /// The persisted progress record (§5), evidence about the past.
    pub progress: PersistedProgress,
    /// The journal's retained history, contiguous, first slot first.
    pub entries: Vec<LogEntry>,
}

impl NodeState {
    /// Snapshots the live node's durable state for writing. The journal's
    /// retained window is whole by construction: the bench host never
    /// reclaims (`S1` allows it), so a short copy is a defect in this host
    /// and is reported rather than written as an inconsistent snapshot.
    ///
    /// # Errors
    ///
    /// The retained window is not physically whole — a host defect; the
    /// persist is refused.
    pub fn snapshot(
        copies: SuperblockCopies,
        roster: Vec<String>,
        progress: PersistedProgress,
        journal_view: &impl JournalView,
    ) -> Result<NodeState, String> {
        let mut entries = Vec::new();
        let outcome = match journal_view.accepted() {
            Some(frontier) => {
                let (base, _) = journal_view.retained();
                journal_view.copy_out(base, frontier, &mut entries)
            }
            None => RangeOutcome::Complete,
        };
        match outcome {
            RangeOutcome::Complete => {}
            other => return Err(format!("the retained window is not whole: {other:?}")),
        }
        Ok(NodeState {
            copies,
            roster,
            progress,
            entries,
        })
    }
}

/// The exact encoded size, then the bytes — the W3 discipline: budget
/// with the length function, write once.
fn encode(state: &NodeState) -> Vec<u8> {
    let roster_len: usize = state.roster.iter().map(|name| 4 + name.len()).sum();
    let entries_len: usize = state.entries.iter().map(|e| 4 + e.packed_len()).sum();
    let total = 4                       // magic
        + 4                             // version
        + 4 * 9                         // four copies: (identity u64, marker u8)
        + 4                             // roster count
        + roster_len                    // each name: u32 prefix + bytes
        + 4 + 4 + 4 + 4 + 4             // current era/view, retained era/view, status word
        + 8 * 4                         // accepted, committed, applied, checkpoint
        + 8                             // revision
        + 1                             // fault word
        + 4                             // entry count
        + entries_len; // each entry: u32 prefix + packed body
    let mut buf = vec![0u8; total];
    let mut w = match PackWriter::new(&mut buf, total) {
        Ok(w) => w,
        Err(_) => unreachable!("the buffer was sized for the exact requirement"),
    };
    w.u32(MAGIC);
    w.u32(VERSION);
    for copy in &state.copies.copies {
        w.u64(copy.identity.0);
        w.u8(match copy.marker {
            Marker::Flushed => MARKER_FLUSHED,
            Marker::Unflushed => MARKER_UNFLUSHED,
        });
    }
    w.u32(u32::try_from(state.roster.len()).expect("roster fits u32"));
    for member in &state.roster {
        w.opaque(member.as_bytes());
    }
    w.u32(state.progress.current.era.0);
    w.u32(state.progress.current.view.0);
    w.u32(state.progress.retained.era.0);
    w.u32(state.progress.retained.view.0);
    w.u32(state.progress.status.to_word());
    w.u64(state.progress.accepted.0);
    w.u64(state.progress.committed.0);
    w.u64(state.progress.applied.0);
    w.u64(state.progress.checkpoint.0);
    w.u64(state.progress.revision);
    match state.progress.fault {
        None => w.u8(FAULT_NONE),
        Some(fault) => w.u8(fault_word(fault)),
    }
    w.u32(u32::try_from(state.entries.len()).expect("entry count fits u32"));
    for entry in &state.entries {
        let mut body = vec![0u8; entry.packed_len()];
        match entry.pack_into(&mut body) {
            Ok(_) => w.opaque(&body),
            Err(_) => unreachable!("the scratch buffer was sized by packed_len"),
        }
    }
    debug_assert_eq!(w.written(), total, "the budget is exact (W3)");
    buf
}

/// Decodes a state file. Total over every input (the core codec's own
/// rule): a short or malformed file is a named refusal, never a panic.
fn decode(bytes: &[u8]) -> Result<NodeState, DecodeError> {
    let mut c = UnpackCursor::new(bytes);
    let magic = c.u32()?;
    if magic != MAGIC {
        return Err(DecodeError::Refusal(format!(
            "not a state file (magic {magic:#010x})"
        )));
    }
    let version = c.u32()?;
    if version != VERSION {
        return Err(DecodeError::Refusal(format!(
            "unsupported state format version {version}"
        )));
    }
    let copies = decode_copies(&mut c)?;
    let count = c.u32()?;
    let mut roster = Vec::new();
    for _ in 0..count {
        let bytes = c.opaque()?;
        let name = String::from_utf8(bytes.to_vec())
            .map_err(|error| DecodeError::Refusal(format!("roster name is not UTF-8: {error}")))?;
        roster.push(name_checked(&name, &roster)?);
    }
    let current = read_view(&mut c)?;
    let retained = read_view(&mut c)?;
    let status_word = c.u32()?;
    let accepted = c.u64()?;
    let committed = c.u64()?;
    let applied = c.u64()?;
    let checkpoint = c.u64()?;
    let revision = c.u64()?;
    let fault = match c.u8()? {
        FAULT_NONE => None,
        word => Some(fault_from_word(word)?),
    };
    let count = c.u32()?;
    let mut entries = Vec::new();
    for _ in 0..count {
        let frame = c.opaque()?;
        let entry = LogEntry::unpack_from(frame).map_err(|error| {
            DecodeError::Refusal(format!("journal entry does not decode: {error:?}"))
        })?;
        entries.push(entry);
    }
    if c.finish().is_err() {
        return Err(DecodeError::Refusal(
            "trailing bytes after the journal".into(),
        ));
    }
    let status = Status::from_word(status_word)
        .ok_or_else(|| DecodeError::Refusal(format!("unknown status word {status_word}")))?;
    Ok(NodeState {
        copies,
        roster,
        progress: PersistedProgress {
            current,
            retained,
            status,
            accepted: Slot(accepted),
            committed: Slot(committed),
            applied: Slot(applied),
            checkpoint: Slot(checkpoint),
            revision,
            fault,
        },
        entries,
    })
}

/// Four superblock copies, each an incarnation and a marker word.
fn decode_copies(c: &mut UnpackCursor<'_>) -> Result<SuperblockCopies, DecodeError> {
    let mut copies = Vec::new();
    for _ in 0..4 {
        let identity = c.u64()?;
        let marker = match c.u8()? {
            MARKER_FLUSHED => Marker::Flushed,
            MARKER_UNFLUSHED => Marker::Unflushed,
            word => {
                return Err(DecodeError::Refusal(format!(
                    "unknown superblock marker word {word}"
                )));
            }
        };
        copies.push(vrr::replica::CopyState {
            identity: Incarnation(identity),
            marker,
        });
    }
    let [a, b, d, e] = copies.try_into().expect("four copies were read");
    Ok(SuperblockCopies {
        copies: [a, b, d, e],
    })
}

/// `era` then `view`, the wire field order (W1).
fn read_view(c: &mut UnpackCursor<'_>) -> Result<ViewId, DecodeError> {
    let era = c.u32()?;
    let view = c.u32()?;
    Ok(ViewId {
        era: Era(era),
        view: View(view),
    })
}

/// The fault-word table's inverse. A word outside the table is a refusal.
fn fault_from_word(word: u8) -> Result<Fault, DecodeError> {
    match word {
        1 => Ok(Fault::IllegalTransition),
        2 => Ok(Fault::IndeterminatePersistence),
        3 => Ok(Fault::QuorumObligation),
        4 => Ok(Fault::ProgressJournalDivergence),
        5 => Ok(Fault::HostDeclared),
        _ => Err(DecodeError::Refusal(format!("unknown fault word {word}"))),
    }
}

/// The fault-word table's forward direction, stated as a `match` in one
/// place (the core's `Status::to_word` discipline).
fn fault_word(fault: Fault) -> u8 {
    match fault {
        Fault::IllegalTransition => 1,
        Fault::IndeterminatePersistence => 2,
        Fault::QuorumObligation => 3,
        Fault::ProgressJournalDivergence => 4,
        Fault::HostDeclared => 5,
    }
}

/// A roster name must be printable ASCII (a Maelstrom id) and distinct:
/// the genesis order is a set, and a duplicate name would fold an illegal
/// configuration at reopen that the Q1 gate would then refuse — the
/// refusal belongs at the boundary that read the evidence.
fn name_checked(name: &str, seen: &[String]) -> Result<String, DecodeError> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b' ')
    {
        return Err(DecodeError::Refusal(format!(
            "roster name {name:?} is not a Maelstrom id"
        )));
    }
    if seen.iter().any(|other| other == name) {
        return Err(DecodeError::Refusal(format!("roster names {name:?} twice")));
    }
    Ok(name.to_string())
}
