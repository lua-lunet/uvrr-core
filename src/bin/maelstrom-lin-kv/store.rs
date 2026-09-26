//! The bench host's state file: the durable evidence a restart reopens
//! under (`docs/uvrr-reincarnation.md` §2), carried in the host's own
//! store.
//!
//! # Shape (format 3, bench-internal; no cross-version compatibility)
//!
//! One file per node under the state dir, keyed by the node's Maelstrom
//! id:
//!
//! ```text
//! [ header slot A ][ header slot B ][ journal entries, appended in slot order ]
//! ```
//!
//! The journal region is append-only: each newly journaled entry is
//! encoded once, a `u32` length frame and the core's own wire codec
//! packs the [`LogEntry`], so the host invents no second codec for
//! history it did not define, appended at the active header's byte
//! cursor and fsynced. The header is a fixed-size slot holding exactly
//! what a restart must decide from: the four-superblock copies of §2,
//! the genesis roster, the §5 persisted progress record, and the
//! journal region's base slot, entry count and byte length. Committing
//! a transition writes the INACTIVE slot, a monotone sequence number
//! and a checksum over the slot, and fsyncs it. The slot is the
//! commit point.
//!
//! Two slots, alternating: a torn slot write (a crash between the write
//! and its fsync) fails the slot's checksum and the loader falls back
//! to the other slot, the last committed state, never a torn read and
//! never a bricked node. Bytes the active slot does not count, a torn
//! append, or the dead tail a view-change install replaced, are not
//! durable evidence: the loader reads exactly the counted prefix, and
//! the next append overwrites the rest.
//!
//! # The write-through barrier (§7)
//!
//! Per published transition: the new entries are appended and fsynced
//! FIRST, the header slot is written and fsynced SECOND, and only then
//! does the host route the released effects, durable before
//! observable, so a kill loses at most the in-flight input. A storage
//! failure at either step is determinate: the node stops (exit
//! nonzero, Jepsen restarts it from the last committed slot) rather
//! than serve on a store it cannot vouch for.
//!
//! # The uncommitted tail (§9.2)
//!
//! Acceptances append at the tail. A view-change install REPLACES the
//! divergent uncommitted tail ([`vrr::journal::Journal::install_suffix`]),
//! and an installed history can be shorter than the one it replaces.
//! The committed prefix never changes under the node's feet: every
//! shared slot of an offered suffix is checked, and a disagreement at
//! or below the committed frontier is a fault, never an install (the
//! core's `check_suffix`, §9.1), so the store's per-transition work
//! compares only the uncommitted tail `(committed, frontier]`, entry
//! digests against the durable mirror. Matching slots stand; the
//! lowest mismatch truncates the durable region below it and the
//! replaced tail re-appends. Steady state, the walk is the in-flight
//! window and the write is the new entries, O(new) per transition; an
//! install pays only the tail it actually diverges on.
//!
//! The bench never reclaims (`S1` allows it), so the retained window's
//! base is stable; a base that moves is a named refusal, never a
//! re-basing guess. The first life builds the whole file once, temp
//! file, fsync, atomic rename, fsync of the directory, the §7 `Forced`
//! barrier shape, and the reopen path (§2) reads the active slot and
//! its counted prefix. A file no slot of which vouches for itself, a
//! short journal region, or a malformed frame is a crash artifact,
//! answered with the core's reopen refusal path: a named refusal and a
//! nonzero exit, never a panic. Absence is the provisioning case: no
//! file, first life.

use std::fs::File;
use std::io::{self, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use vrr::ids::NodeId;
use vrr::ids::{Era, Fault, Slot, View, ViewId};
use vrr::journal::{JournalView, LogEntry};
use vrr::lifecycle::{Marker, SuperblockCopies};
use vrr::progress::Status;
use vrr::replica::PersistedProgress;
use vrr::wire::{Pack, Unpack};

/// File magic, as a big-endian word. A slot not opening with it is not
/// a committed header.
const MAGIC: u32 = u32::from_be_bytes(*b"UVRS");
/// Format version. Bench-internal: an unknown version is a refusal,
/// not a compatibility surface. Format 3 renumbers the marker words to
/// the core's four-state marker transition machine; a format-2 file's
/// words would misread, so it refuses rather than decodes.
const VERSION: u32 = 4;

/// Marker words, stated as a table so reordering this source cannot
/// renumber the file (the core's `Status::to_word` discipline). The
/// four states of the core's marker transition machine, numbered in
/// [`vrr::replica::Marker`]'s declaration order.
const MARKER_STOPPING: u8 = 0;
const MARKER_STOPPED: u8 = 1;
const MARKER_RESTARTING: u8 = 2;
const MARKER_JOINING: u8 = 3;

/// Fault-word table, same discipline. `0` is "no fault"; the variants
/// are numbered in declaration order of [`vrr::ids::Fault`].
const FAULT_NONE: u8 = 0;

/// The fixed roster area inside a slot: a `u32` count then each name as
/// a `u32`-prefixed opaque. A roster whose framed size does not fit is
/// refused at creation, the bench's `node_ids` are Maelstrom ids, a
/// handful of short strings.
const ROSTER_AREA: usize = 4096;

// Slot field offsets, stated once so encode and decode cannot drift.
const AT_SEQ: usize = 8;
const AT_COPIES: usize = AT_SEQ + 8;
const AT_ROSTER: usize = AT_COPIES + 4 * 5;
const AT_CURRENT: usize = AT_ROSTER + ROSTER_AREA;
const AT_RETAINED: usize = AT_CURRENT + 8;
const AT_STATUS: usize = AT_RETAINED + 8;
const AT_ACCEPTED: usize = AT_STATUS + 4;
const AT_COMMITTED: usize = AT_ACCEPTED + 8;
const AT_APPLIED: usize = AT_COMMITTED + 8;
const AT_CHECKPOINT: usize = AT_APPLIED + 8;
const AT_REVISION: usize = AT_CHECKPOINT + 8;
const AT_FAULT: usize = AT_REVISION + 8;
const AT_BASE: usize = AT_FAULT + 1;
const AT_COUNT: usize = AT_BASE + 8;
const AT_BYTES: usize = AT_COUNT + 8;
const AT_CHECKSUM: usize = AT_BYTES + 8;
/// One header slot's exact size.
const HEADER_SIZE: usize = AT_CHECKSUM + 8;
/// Where the journal region starts: past both slots.
const ENTRIES_BASE: u64 = 2 * HEADER_SIZE as u64;

/// One node's persistence home: the state file's path and, once a life
/// has armed it, the open file plus the durable mirror the incremental
/// commits walk.
pub struct Store {
    path: PathBuf,
    /// The armed mirror: `None` until a successful [`Store::load`] (a
    /// reopen) or [`Store::create`] (a first life).
    log: Option<LogFile>,
}

/// The open state file and the in-memory mirror of its committed
/// content.
struct LogFile {
    /// Opened read-write without append mode: commits seek to the
    /// inactive slot's offset and to the journal cursor.
    file: File,
    /// Which slot the last commit landed in.
    side: Side,
    /// The active slot's sequence number: one per commit, monotone.
    seq: u64,
    /// The genesis roster, fixed at creation; every commit re-records
    /// it and a reopen refuses a roster the init did not name.
    roster: Vec<String>,
    /// The first durable entry's slot: the retained window's base. The
    /// bench never reclaims, so this never moves.
    base: Slot,
    /// Per durable entry, oldest first: its byte offset from the
    /// journal region's base, its framed length, and the digest of its
    /// packed body. O(history) memory, the same order as the journal
    /// itself.
    entries: Vec<DurableEntry>,
    /// The active slot's `entries_bytes`: where the next append
    /// writes.
    bytes: u64,
}

/// One durable entry's mirror record.
struct DurableEntry {
    offset: u64,
    len: u32,
    digest: u64,
}

/// The alternating header slots.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    A,
    B,
}

impl Side {
    /// The slot's byte offset in the file.
    fn at(self) -> u64 {
        match self {
            Side::A => 0,
            Side::B => HEADER_SIZE as u64,
        }
    }

    /// The other slot.
    fn flip(self) -> Side {
        match self {
            Side::A => Side::B,
            Side::B => Side::A,
        }
    }
}

impl Store {
    /// The per-node file path under `dir`, keyed by the node's
    /// Maelstrom id.
    ///
    /// # Errors
    ///
    /// The directory could not be created, the persistence home is
    /// unusable, and the node must not claim durability it cannot
    /// deliver.
    pub fn open(dir: &Path, node: &str) -> io::Result<Store> {
        std::fs::create_dir_all(dir)?;
        Ok(Store {
            path: dir.join(format!("{node}.uvrr-state")),
            log: None,
        })
    }

    /// Whether a life has armed the mirror: `load` returned a state or
    /// `create` built the first one. The volatile mode never arms it.
    pub fn armed(&self) -> bool {
        self.log.is_some()
    }

    /// Reads the node's state file, the valid slot with the higher
    /// sequence number, then exactly its counted entry prefix, and
    /// arms the mirror for the commits that follow. Any decode failure
    ///, magic, version, checksum, short read, malformed field, is a
    /// named refusal: the file is evidence the node cannot vouch for.
    ///
    /// # Errors
    ///
    /// [`StoreError::Absent`] when there is no file (first life),
    /// [`StoreError::Corrupt`] for every other reason.
    pub fn load(&mut self) -> Result<NodeState, StoreError> {
        let bytes = match std::fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(StoreError::Absent);
            }
            Err(error) => return Err(StoreError::Corrupt(format!("unreadable: {error}"))),
        };
        if bytes.len() < 2 * HEADER_SIZE {
            return Err(StoreError::Corrupt(
                "the file is shorter than its header pair".into(),
            ));
        }
        let slot_a = SlotBytes::decode(&bytes[..HEADER_SIZE]);
        let slot_b = SlotBytes::decode(&bytes[HEADER_SIZE..2 * HEADER_SIZE]);
        let (slot, side) = match (slot_a, slot_b) {
            (Some(a), Some(b)) => {
                if a.seq >= b.seq {
                    (a, Side::A)
                } else {
                    (b, Side::B)
                }
            }
            (Some(a), None) => (a, Side::A),
            (None, Some(b)) => (b, Side::B),
            (None, None) => {
                return Err(StoreError::Corrupt("no slot vouches for itself".into()));
            }
        };
        let end = ENTRIES_BASE
            .checked_add(slot.bytes)
            .ok_or_else(|| StoreError::Corrupt("the journal region's length overflows".into()))?;
        let end = usize::try_from(end)
            .ok()
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| {
                StoreError::Corrupt("the journal region is short of the header's count".into())
            })?;
        let (entries, mirror) =
            decode_entries(&bytes[ENTRIES_BASE as usize..end], slot.count, slot.base)
                .map_err(StoreError::Corrupt)?;
        let file = File::options()
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|error| StoreError::Corrupt(format!("cannot open for append: {error}")))?;
        self.log = Some(LogFile {
            file,
            side,
            seq: slot.seq,
            roster: slot.roster.clone(),
            base: slot.base,
            entries: mirror,
            bytes: slot.bytes,
        });
        Ok(NodeState {
            copies: slot.copies,
            roster: slot.roster,
            progress: slot.progress,
            entries,
        })
    }

    /// The first life: builds the whole file, slot A committed at
    /// sequence 1, slot B zeroed (it fails the magic until the first
    /// commit flips to it), the journal region, through a temp file,
    /// fsync, atomic rename, fsync of the directory, then arms the
    /// mirror. Once per life, not per transition.
    ///
    /// # Errors
    ///
    /// A named refusal, the store's own defect class, or any storage
    /// failure. A failed temp write or rename leaves no file (or the
    /// temp file aside, unread by any loader), so the durable evidence
    /// never names a state the node did not reach.
    pub fn create(&mut self, state: &NodeState) -> Result<(), String> {
        if self.log.is_some() {
            return Err("create on an armed store".into());
        }
        let Some(base) = state.entries.first().map(|entry| entry.slot) else {
            return Err("the first life holds no genesis to persist".into());
        };
        let mut region = Vec::new();
        let mut mirror = Vec::new();
        let mut offset = 0u64;
        for entry in &state.entries {
            let framed = frame(entry)?;
            let digest = digest_of(entry)?;
            mirror.push(DurableEntry {
                offset,
                len: u32::try_from(framed.len()).map_err(|_| "an entry cannot frame")?,
                digest,
            });
            offset += framed.len() as u64;
            region.extend_from_slice(&framed);
        }
        let slot = SlotBytes {
            seq: 1,
            copies: state.copies,
            roster: state.roster.clone(),
            progress: state.progress,
            base,
            count: state.entries.len() as u64,
            bytes: offset,
        };
        let mut file_bytes = vec![0u8; 2 * HEADER_SIZE + region.len()];
        slot.encode(&mut file_bytes[..HEADER_SIZE])?;
        file_bytes[ENTRIES_BASE as usize..].copy_from_slice(&region);
        let temp = self.path.with_extension("uvrr-tmp");
        {
            let mut file = File::create(&temp)
                .map_err(|error| format!("the temp state file is unwritable: {error}"))?;
            file.write_all(&file_bytes)
                .and_then(|_| file.sync_all())
                .map_err(|error| format!("the temp state file is unwritable: {error}"))?;
        }
        std::fs::rename(&temp, &self.path)
            .map_err(|error| format!("the state file cannot land: {error}"))?;
        if let Some(parent) = self.path.parent() {
            File::open(parent)
                .and_then(|dir| dir.sync_all())
                .map_err(|error| format!("the state dir cannot fsync: {error}"))?;
        }
        let file = File::options()
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|error| format!("the state file cannot reopen: {error}"))?;
        self.log = Some(LogFile {
            file,
            side: Side::A,
            seq: 1,
            roster: state.roster.clone(),
            base,
            entries: mirror,
            bytes: offset,
        });
        Ok(())
    }

    /// One published transition, written through: the journal's new
    /// entries append (fsynced), then the inactive header slot commits
    /// the counts, the progress record and the superblock copies
    /// (fsynced). The caller routes released effects only after this
    /// returns.
    ///
    /// The delta against the mirror: slots at or below the committed
    /// frontier are quorum-identical and never recompared (§9.2, the
    /// core's `check_suffix` faults on a committed-slot disagreement,
    /// so an install cannot alter them); the uncommitted tail
    /// `(committed, frontier]` is digest-walked, matching slots
    /// stand, the lowest mismatch truncates the durable region below
    /// it, and the journal's entries from there to the accepted
    /// frontier append. A journal that no longer holds a slot the
    /// mirror holds is a host defect, refused loudly.
    ///
    /// # Errors
    ///
    /// A named refusal, the store's own defect class, or any storage
    /// failure. Both are the caller's determinate stop.
    pub fn commit(
        &mut self,
        copies: SuperblockCopies,
        progress: PersistedProgress,
        journal: &impl JournalView,
    ) -> Result<(), String> {
        let log = self.log.as_mut().ok_or("commit on an unarmed store")?;
        let frontier = journal
            .accepted()
            .ok_or("the journal holds nothing durable")?;
        let (base, _) = journal.retained();
        if base != log.base {
            return Err(format!(
                "the retained window moved: slot {base:?} is not the durable base {:?}",
                log.base
            ));
        }
        if log.entries.is_empty() {
            return Err("the durable mirror is empty".into());
        }
        let mirror_frontier = Slot(log.base.0 + log.entries.len() as u64 - 1);
        // The standing prefix: the lowest mismatch in the uncommitted
        // tail, else the whole matching tail.
        let hi = Slot(std::cmp::min(mirror_frontier.0, frontier.0));
        let mut stand = hi;
        let mut slot = hi.0;
        while slot > progress.committed.0 && slot > log.base.0 {
            let entry = journal
                .get(Slot(slot))
                .ok_or_else(|| format!("the journal no longer holds the durable slot {slot}"))?;
            let index = (slot - log.base.0) as usize;
            if digest_of(entry)? != log.entries[index].digest {
                stand = Slot(slot - 1);
            }
            slot -= 1;
        }
        if stand.0 + 1 < log.base.0 {
            return Err(format!(
                "the standing prefix {stand:?} is below the durable base {:?}",
                log.base
            ));
        }
        // The standing prefix's byte end: where the append writes.
        log.entries.truncate((stand.0 - log.base.0 + 1) as usize);
        let write_at = log
            .entries
            .last()
            .map(|entry| entry.offset + entry.len as u64)
            .unwrap_or(0);
        let mut cursor = write_at;
        let mut region = Vec::new();
        for number in (stand.0 + 1)..=frontier.0 {
            let entry = journal
                .get(Slot(number))
                .ok_or_else(|| format!("the journal no longer holds the durable slot {number}"))?;
            let framed = frame(entry)?;
            let digest = digest_of(entry)?;
            log.entries.push(DurableEntry {
                offset: cursor,
                len: u32::try_from(framed.len()).map_err(|_| "an entry cannot frame")?,
                digest,
            });
            cursor += framed.len() as u64;
            region.extend_from_slice(&framed);
        }
        // The barrier: the entries are durable before the slot that
        // counts them, and the slot is durable before the caller routes
        // anything. Nothing to append still commits the header, the
        // progress record moved (a tick's revision included).
        if !region.is_empty() {
            log.file
                .seek(SeekFrom::Start(ENTRIES_BASE + write_at))
                .and_then(|_| log.file.write_all(&region))
                .and_then(|_| log.file.sync_all())
                .map_err(|error| format!("the journal region cannot sync: {error}"))?;
        }
        let side = log.side.flip();
        let slot_bytes = SlotBytes {
            seq: log.seq + 1,
            copies,
            roster: log.roster.clone(),
            progress,
            base: log.base,
            count: log.entries.len() as u64,
            bytes: cursor,
        };
        let mut buf = [0u8; HEADER_SIZE];
        slot_bytes.encode(&mut buf)?;
        log.file
            .seek(SeekFrom::Start(side.at()))
            .and_then(|_| log.file.write_all(&buf))
            .and_then(|_| log.file.sync_all())
            .map_err(|error| format!("the header slot cannot commit: {error}"))?;
        log.side = side;
        log.seq += 1;
        log.bytes = cursor;
        Ok(())
    }

    /// The clean stop's drain (the marker machine's T1: the host flushes
    /// WALs and grids strictly between the `Stopping` and `Stopped`
    /// writes). This store's WAL is the journal region and its grid is
    /// the header slot, one file, and every commit already fsyncs both,
    /// so the drain is the file's own fsync: the flush that lets a
    /// `Stopped` copy vouch for the state under it.
    ///
    /// # Errors
    ///
    /// The file cannot be flushed, the stop cannot prove its drain, so
    /// the caller must not write `Stopped`.
    pub fn drain(&mut self) -> Result<(), String> {
        let log = self.log.as_mut().ok_or("drain on an unarmed store")?;
        log.file
            .sync_all()
            .map_err(|error| format!("the state file cannot drain: {error}"))
    }
}

/// Why the reopen decision could not read the durable evidence.
#[derive(Debug)]
pub enum StoreError {
    /// No state file: a first life. The host provisions.
    Absent,
    /// The file exists but is not a self-consistent record, a crash
    /// artifact, bit rot, or an unreadable path. The core's reopen
    /// refusal path, never a panic.
    Corrupt(String),
}

/// What a node's state file holds: everything `Node::reopen` needs, plus
/// the §2 restart model. The first life's `create` takes it whole; the
/// steady state commits deltas.
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
    /// Snapshots the live node's durable state for the first life. The
    /// journal's retained window is whole by construction: the bench
    /// host never reclaims (`S1` allows it), so a short copy is a defect
    /// in this host and is reported rather than written as an
    /// inconsistent snapshot.
    ///
    /// # Errors
    ///
    /// The retained window is not physically whole, a host defect; the
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
            None => vrr::journal::RangeOutcome::Complete,
        };
        match outcome {
            vrr::journal::RangeOutcome::Complete => {}
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

/// One decoded header slot. Every field is fixed-size; the checksum
/// covers the bytes before it.
struct SlotBytes {
    seq: u64,
    copies: SuperblockCopies,
    roster: Vec<String>,
    progress: PersistedProgress,
    base: Slot,
    count: u64,
    bytes: u64,
}

impl SlotBytes {
    /// Writes the slot: the fields, then the checksum over them.
    fn encode(&self, buf: &mut [u8]) -> Result<(), String> {
        let framed = 4 + self.roster.iter().map(|name| 4 + name.len()).sum::<usize>();
        if framed > ROSTER_AREA {
            return Err(format!(
                "the roster's framed size {framed} exceeds the slot's fixed area {ROSTER_AREA}"
            ));
        }
        buf.fill(0);
        put_u32(buf, 0, MAGIC);
        put_u32(buf, 4, VERSION);
        put_u64(buf, AT_SEQ, self.seq);
        for (index, copy) in self.copies.copies.iter().enumerate() {
            put_u32(buf, AT_COPIES + index * 5, copy.identity.0);
            buf[AT_COPIES + index * 5 + 4] = match copy.marker {
                Marker::Stopping => MARKER_STOPPING,
                Marker::Stopped => MARKER_STOPPED,
                Marker::Restarting => MARKER_RESTARTING,
                Marker::Joining => MARKER_JOINING,
            };
        }
        put_u32(buf, AT_ROSTER, self.roster.len() as u32);
        let mut at = AT_ROSTER + 4;
        for name in &self.roster {
            let len = u32::try_from(name.len()).map_err(|_| "a roster name cannot frame")?;
            put_u32(buf, at, len);
            buf[at + 4..at + 4 + name.len()].copy_from_slice(name.as_bytes());
            at += 4 + name.len();
        }
        put_u32(buf, AT_CURRENT, self.progress.current.era.0);
        put_u32(buf, AT_CURRENT + 4, self.progress.current.view.0);
        put_u32(buf, AT_RETAINED, self.progress.retained.era.0);
        put_u32(buf, AT_RETAINED + 4, self.progress.retained.view.0);
        put_u32(buf, AT_STATUS, self.progress.status.to_word());
        put_u64(buf, AT_ACCEPTED, self.progress.accepted.0);
        put_u64(buf, AT_COMMITTED, self.progress.committed.0);
        put_u64(buf, AT_APPLIED, self.progress.applied.0);
        put_u64(buf, AT_CHECKPOINT, self.progress.checkpoint.0);
        put_u64(buf, AT_REVISION, self.progress.revision);
        buf[AT_FAULT] = match self.progress.fault {
            None => FAULT_NONE,
            Some(fault) => fault_word(fault),
        };
        put_u64(buf, AT_BASE, self.base.0);
        put_u64(buf, AT_COUNT, self.count);
        put_u64(buf, AT_BYTES, self.bytes);
        put_u64(buf, AT_CHECKSUM, fnv1a(&buf[..AT_CHECKSUM]));
        Ok(())
    }

    /// Reads one slot. `None` is an uncommitted or torn slot, not an
    /// error: the other slot may vouch for the file.
    fn decode(buf: &[u8]) -> Option<SlotBytes> {
        if get_u32(buf, 0) != MAGIC || get_u32(buf, 4) != VERSION {
            return None;
        }
        if get_u64(buf, AT_CHECKSUM) != fnv1a(&buf[..AT_CHECKSUM]) {
            return None;
        }
        let mut copies = Vec::new();
        for index in 0..4 {
            let identity = NodeId(get_u32(buf, AT_COPIES + index * 5));
            let marker = match buf[AT_COPIES + index * 5 + 4] {
                MARKER_STOPPING => Marker::Stopping,
                MARKER_STOPPED => Marker::Stopped,
                MARKER_RESTARTING => Marker::Restarting,
                MARKER_JOINING => Marker::Joining,
                _ => return None,
            };
            copies.push(vrr::lifecycle::CopyState { identity, marker });
        }
        let [a, b, d, e] = copies.try_into().expect("four copies were read");
        let count = get_u32(buf, AT_ROSTER) as usize;
        let mut roster = Vec::new();
        let mut at = AT_ROSTER + 4;
        for _ in 0..count {
            let len = get_u32(buf, at) as usize;
            at = at.checked_add(4)?.checked_add(len)?;
            if at > AT_ROSTER + ROSTER_AREA {
                return None;
            }
            let name = String::from_utf8(buf[at - len..at].to_vec()).ok()?;
            roster.push(name_checked(&name, &roster)?);
        }
        let status = Status::from_word(get_u32(buf, AT_STATUS))?;
        let fault = match buf[AT_FAULT] {
            FAULT_NONE => None,
            word => Some(fault_from_word(word)?),
        };
        Some(SlotBytes {
            seq: get_u64(buf, AT_SEQ),
            copies: SuperblockCopies {
                copies: [a, b, d, e],
            },
            roster,
            progress: PersistedProgress {
                current: ViewId {
                    era: Era(get_u32(buf, AT_CURRENT)),
                    view: View(get_u32(buf, AT_CURRENT + 4)),
                },
                retained: ViewId {
                    era: Era(get_u32(buf, AT_RETAINED)),
                    view: View(get_u32(buf, AT_RETAINED + 4)),
                },
                status,
                accepted: Slot(get_u64(buf, AT_ACCEPTED)),
                committed: Slot(get_u64(buf, AT_COMMITTED)),
                applied: Slot(get_u64(buf, AT_APPLIED)),
                checkpoint: Slot(get_u64(buf, AT_CHECKPOINT)),
                revision: get_u64(buf, AT_REVISION),
                fault,
            },
            base: Slot(get_u64(buf, AT_BASE)),
            count: get_u64(buf, AT_COUNT),
            bytes: get_u64(buf, AT_BYTES),
        })
    }
}

/// Decodes the journal region: `count` frames must consume the region
/// exactly, sit contiguous from `base`, and each body must decode
/// through the core's own codec. Returns the entries and the mirror.
fn decode_entries(
    region: &[u8],
    count: u64,
    base: Slot,
) -> Result<(Vec<LogEntry>, Vec<DurableEntry>), String> {
    let mut entries = Vec::new();
    let mut mirror = Vec::new();
    let mut at = 0usize;
    for number in 0..count {
        if at + 4 > region.len() {
            return Err(format!("the journal region is short at entry {number}"));
        }
        let len = u32::from_be_bytes(region[at..at + 4].try_into().expect("four bytes")) as usize;
        at += 4;
        if at.checked_add(len).is_none_or(|end| end > region.len()) {
            return Err(format!("the journal region is short at entry {number}"));
        }
        let entry = LogEntry::unpack_from(&region[at..at + len])
            .map_err(|error| format!("journal entry {number} does not decode: {error:?}"))?;
        if entry.slot.0 != base.0 + number {
            return Err(format!(
                "journal entry {number} sits at slot {}, not the contiguous {}",
                entry.slot.0,
                base.0 + number
            ));
        }
        mirror.push(DurableEntry {
            offset: (at - 4) as u64,
            len: (len + 4) as u32,
            digest: fnv1a(&region[at..at + len]),
        });
        at += len;
        entries.push(entry);
    }
    if at != region.len() {
        return Err("the journal region holds bytes beyond the counted entries".into());
    }
    Ok((entries, mirror))
}

/// One entry's frame: a `u32` length, then the packed body.
fn frame(entry: &LogEntry) -> Result<Vec<u8>, String> {
    let len = u32::try_from(entry.packed_len()).map_err(|_| "an entry cannot frame")?;
    let mut framed = vec![0u8; 4 + entry.packed_len()];
    framed[..4].copy_from_slice(&len.to_be_bytes());
    entry
        .pack_into(&mut framed[4..])
        .map_err(|error| format!("the entry does not pack: {error:?}"))?;
    Ok(framed)
}

/// The digest of an entry's packed body: the mirror's comparison key.
fn digest_of(entry: &LogEntry) -> Result<u64, String> {
    let mut body = vec![0u8; entry.packed_len()];
    entry
        .pack_into(&mut body)
        .map_err(|error| format!("the entry does not pack: {error:?}"))?;
    Ok(fnv1a(&body))
}

/// FNV-1a, 64-bit: a checksum small enough to state in this file, with
/// no dependency to add for the sake of one.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn put_u32(buf: &mut [u8], at: usize, value: u32) {
    buf[at..at + 4].copy_from_slice(&value.to_be_bytes());
}

fn put_u64(buf: &mut [u8], at: usize, value: u64) {
    buf[at..at + 8].copy_from_slice(&value.to_be_bytes());
}

fn get_u32(buf: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(buf[at..at + 4].try_into().expect("four bytes"))
}

fn get_u64(buf: &[u8], at: usize) -> u64 {
    u64::from_be_bytes(buf[at..at + 8].try_into().expect("eight bytes"))
}

/// The fault-word table's inverse. A word outside the table is a
/// refusal.
fn fault_from_word(word: u8) -> Option<Fault> {
    match word {
        1 => Some(Fault::IllegalTransition),
        2 => Some(Fault::IndeterminatePersistence),
        3 => Some(Fault::QuorumObligation),
        4 => Some(Fault::ProgressJournalDivergence),
        5 => Some(Fault::HostDeclared),
        _ => None,
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
/// configuration at reopen that the Q1 gate would then refuse, the
/// refusal belongs at the boundary that read the evidence.
fn name_checked(name: &str, seen: &[String]) -> Option<String> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b' ')
    {
        return None;
    }
    if seen.iter().any(|other| other == name) {
        return None;
    }
    Some(name.to_string())
}
