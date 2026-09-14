//! NEW  — zig/uvrr/store.zig
//!
//! Synchronous driver over the VENDORED TB IO subsystem (zig/io/*.zig — the
//! real per-OS direct block-IO layer: io_uring on linux, kqueue+F_NOCACHE on
//! darwin, IOCP on windows) and the vendored superblock quorum logic. The
//! consumer (examples/uvrr-reincarnation) is a sync C-ABI caller, so
//! completions are drained synchronously; every read and write goes through
//! TB's IO (open_data_file: O_DSYNC + F_NOCACHE + flock, F_FULLFSYNC flush on
//! darwin; O_DIRECT on linux).
//!
//! Data-file layout (see vsr.Zone): four superblock copies (superblock zone),
//! a 32 KiB membership ops WAL (8 sector-sized slots), one 4 KiB checkpoint
//! block.
//!
//! THE MARKER TRANSITION MACHINE (§5.1 of `docs/vrr-durability-model.md`;
//! the pure Rust twin `src/replica/reincarnation.rs`, whose `Marker`,
//! `classify` and `restart` this store mirrors over real IO). uVRR performs
//! no disk flushes on the normal path: the superblock markers are an ordered
//! transition system, each state naming the transition that must have
//! completed for it to exist —
//!
//! ```text
//! Running ──stop──> Stopping ──drain──> Stopped ──boot, 2-of-4──> Restarting
//!                   (4x write)  flush     (4x write)              (4x write)
//!                               WALs + grids
//! Running ──crash──> (markers unchanged) ──boot, no 2-of-4 Stopped──> Joining
//!                                                        (bump, 4x write)
//! ```
//!
//! T1 — the stop command (`begin_stop`) writes `stopping` 4x; the drain —
//! the HOST flushes WALs and grids — sits STRICTLY between the two marker
//! writes; `finish_stop` writes `stopped` 4x. The marker order IS the
//! drain's proof: a `stopped` copy vouches for the WAL under it. No
//! `started` state is written: no safety logic looks for `started`, it
//! looks for `stopped` — the extra superblock write buys no safety and is
//! elided. T2 — the boot (`open`) reads the working quorum at the open
//! threshold (2-of-4) and its winning cohort's verdict: ≥2 `stopped` is
//! the clean stop, so the node continues under the same identity and
//! writes `restarting` 4x. T3 — anything else — a crash, a torn marker
//! set, or death mid-join — means the identity is dead: the node bumps it
//! and writes `joining` 4x. The word is reincarnation. Marker writes are
//! startup/shutdown-only, durable-on-write through the IO layer's sync
//! path, and are the only disk traffic outside the stop path; the boot's
//! uniform 4x write is also the repair (every disagreeing copy rewritten).

const std = @import("std");
const assert = std.debug.assert;
const mem = std.mem;
const posix = std.posix;

const IO = @import("../io.zig").IO;
const constants = @import("../constants.zig");
const vsr = @import("../vsr.zig");
const superblock_mod = @import("../vsr/superblock.zig");
const membership = @import("membership.zig");
const stdx = @import("stdx");

pub const SuperBlockHeader = superblock_mod.SuperBlockHeader;
pub const superblock_copies = constants.superblock_copies;
pub const copy_size = superblock_mod.superblock_copy_size;
pub const data_file_size_min = superblock_mod.data_file_size_min;

/// The open threshold: 2 of 4 (the vendored TB quorum table).
pub const open_threshold: u8 = superblock_mod.Quorums.Threshold.open.count();

const io_entries_max: u12 = 64;

/// One superblock copy's marker (§5.1; the Rust twin's `Marker`): one state
/// of the ordered marker transition system, as written by T1/T2/T3. Each
/// state names the transition that must have completed for it to exist:
///
/// ```text
/// Running ──stop──> Stopping ──drain──> Stopped ──boot, 2-of-4──> Restarting
/// Running ──crash──> (markers unchanged) ──boot, no 2-of-4 Stopped──> Joining
/// ```
///
/// No `started` state is written: no safety logic looks for `started`, it
/// looks for `stopped` — the extra superblock write buys no safety and is
/// elided.
///
/// The unnamed values (`_`) are READ-PATH ONLY, never written: the marker
/// byte sits under the checksum mask (a torn marker write must not fork the
/// checksum), so a byte that is not one of the four written states — bit
/// rot — still arrives with a valid checksum. Such a copy votes in its
/// identity cohort (its `uvrr_incarnation` is checksummed) but never as
/// `stopped`: it can vouch for no transition.
pub const Marker = enum(u8) {
    /// T1's first half: the stop command was received (`Running ──stop──>
    /// stopping`, 4x). The node has stopped sending; the drain — the host
    /// flushes WALs and grids — has not yet been proven, so this state
    /// vouches for nothing.
    stopping = 0,
    /// T1's second half: the `stopping ──drain──> stopped` transition
    /// completed (4x). The drain happened strictly between the `stopping`
    /// write and this one, so this state vouches for the WAL under it:
    /// there is no amnesiac risk under a `stopped` marker.
    stopped = 1,
    /// T2: the `stopped ──boot, 2-of-4──> restarting` transition completed
    /// (4x). The node restarted a CONTROLLED shutdown under the same
    /// identity: a member with complete state, no amnesia, ticking the
    /// full protocol — suspecting a silent primary like any backup.
    restarting = 2,
    /// T3: the `crash ──boot, no 2-of-4 stopped──> joining` transition
    /// completed (bump, 4x). The identity was reincarnated under a bumped
    /// incarnation: NOT a member — it neither votes nor drives view
    /// change — until the forced sequence seats it.
    joining = 3,
    _,
};

/// One superblock copy's boot view: the identity the copy vouches for and
/// its marker (the Rust twin's `CopyState`).
pub const CopyState = struct {
    /// The identity this copy vouches for.
    identity: u64,
    /// The copy's marker.
    marker: Marker,
};

/// The boot's cohort view of the four copies: `null` where the copy is not
/// a member of the working copyset (a torn sector's checksum failed, or a
/// stale sequence), so it cannot vote. The Rust twin's four copies are
/// always whole — it has no sequence hash-chain; this store's view is the
/// same rule over the working copyset the sequence chain selects.
pub const CohortView = [superblock_copies]?CopyState;

/// The quorum read's verdict (§5.1; the Rust twin's `RestartClass`): did the
/// `stopping ──drain──> stopped` transition complete?
pub const RestartClass = enum {
    /// The winning cohort holds ≥2 `stopped` copies: the transition
    /// completed, the drain is proven, there is no amnesiac risk — the
    /// clean stop.
    stopped,
    /// No stopped quorum — a crash, a torn marker set, or death mid-join:
    /// no controlled shutdown. This identity is dead.
    not_stopped,
};

/// What a restart decided (§5.1's decision table; the Rust twin's
/// `RestartDecision`).
pub const RestartDecision = union(enum) {
    /// A stopped quorum: continue under the same identity — a member with
    /// complete state, no amnesia — and write `restarting` to all four
    /// copies (the boot of a controlled shutdown).
    continued: u64,
    /// No stopped quorum: the identity is dead. Bump it — exactly one past
    /// the quorum-resolved identity — and write `joining` to all four
    /// copies. The pair IS the commitment: the wire phase always follows.
    bumped: struct { old: u64, new: u64 },
};

/// Why a restart refused (§5.1; the Rust twin's `RestartRefusal`).
pub const RestartRefusal = error{
    /// No identity cohort reaches the open threshold — 2 of 4: the marker
    /// set is torn beyond the quorum read.
    QuorumLost,
    /// The bump would wrap the identity space, so it refuses instead: a
    /// wrapped identity would make a superseded one indistinguishable from
    /// a current one. The identity that could not be bumped is the
    /// quorum-resolved one the view carried.
    IdentityExhausted,
};

/// The quorum read's answer: the verdict and the quorum-resolved identity.
pub const Verdict = struct {
    class: RestartClass,
    /// The identity the winning cohort carries — the identity the boot
    /// continues (T2) or supersedes (T3).
    identity: u64,
};

/// What a restart decided and the 4x marker write the decision leaves on
/// disk (§5.1's decision table).
pub const Restart = struct {
    decision: RestartDecision,
    /// The uniform 4x write: `(identity, restarting)` for a stopped quorum,
    /// `(new, joining)` for a bump. This IS the repair — every copy that
    /// disagreed with the working quorum is rewritten from the decision,
    /// 4-of-4, satisfying the repair-to-≥3-of-4 behaviour.
    written: [superblock_copies]CopyState,
};

/// The quorum read (§5.1; the Rust twin's `classify`). The boot question is
/// *did the transition complete?*, answered at the open threshold — 2 of 4.
/// The copies agreeing on one identity form a cohort; a cohort reaching the
/// open threshold is a working cohort; the winner is the HIGHEST-identity
/// working cohort — higher-identity-wins INSIDE the working quorum, never
/// the highest identity observed across all copies, which a lone stale or
/// superseded copy cannot impose. The verdict reads the winner's cohort:
/// `stopped` ⟺ it holds ≥2 `stopped` copies; anything else is not stopped.
///
/// Over the machine's written states (uniform 4x writes per transition) the
/// winner cohort is all four copies, so the read is exactly "2-of-4 copies
/// hold `stopped`"; the cohort rule is what the torn cases need. Returns
/// `null` when no cohort reaches the threshold — the torn marker set with
/// no quorum (the store's `error.QuorumLost`).
pub fn classify(view: CohortView) ?Verdict {
    // Per identity: how many copies carry it, and how many of those hold
    // `stopped` (the state right of the stopping──drain──>stopped
    // transition).
    var cohorts: [superblock_copies]struct { identity: u64, count: u8, stopped: u8 } = undefined;
    var cohort_count: usize = 0;
    for (view) |slot| {
        const copy = slot orelse continue;
        for (cohorts[0..cohort_count]) |*cohort| {
            if (cohort.identity != copy.identity) continue;
            cohort.count += 1;
            cohort.stopped += @intFromBool(copy.marker == .stopped);
            break;
        } else {
            cohorts[cohort_count] = .{
                .identity = copy.identity,
                .count = 1,
                .stopped = @intFromBool(copy.marker == .stopped),
            };
            cohort_count += 1;
        }
    }
    // The winner: the highest-identity cohort that reaches the open
    // threshold. A lone stale or superseded copy cannot impose its
    // identity.
    var winner: ?usize = null;
    for (cohorts[0..cohort_count], 0..) |cohort, index| {
        if (cohort.count < open_threshold) continue;
        if (winner == null or cohort.identity > cohorts[winner.?].identity) winner = index;
    }
    const w = winner orelse return null;
    return .{
        .class = if (cohorts[w].stopped >= open_threshold) .stopped else .not_stopped,
        .identity = cohorts[w].identity,
    };
}

/// A restart: the quorum read, the decision, and the 4x marker write the
/// decision leaves on disk (§5.1's decision table; the Rust twin's
/// `restart`).
///
/// * Stopped quorum → `.continued`: continue under the same identity — a
///   member with complete state — and write `(identity, restarting)` 4x.
/// * No stopped quorum → `.bumped`: the identity is dead; bump it —
///   exactly one past the quorum-resolved identity, checked: an identity
///   one bump from `u64` exhaustion refuses rather than wraps — and write
///   `(new, joining)` 4x. A node dying mid-join reads no stopped quorum
///   and reincarnates again — the table makes that fall out.
pub fn restart(view: CohortView) RestartRefusal!Restart {
    const verdict = classify(view) orelse return error.QuorumLost;
    switch (verdict.class) {
        .stopped => return .{
            .decision = .{ .continued = verdict.identity },
            .written = @splat(.{ .identity = verdict.identity, .marker = .restarting }),
        },
        .not_stopped => {
            // The bump is the sole constructor of a higher identity, and
            // the identity it returns is strictly greater than the one it
            // supersedes — the superseded band never re-enters circulation.
            const new = std.math.add(u64, verdict.identity, 1) catch return error.IdentityExhausted;
            assert(new > verdict.identity);
            return .{
                .decision = .{ .bumped = .{ .old = verdict.identity, .new = new } },
                .written = @splat(.{ .identity = new, .marker = .joining }),
            };
        },
    }
}

/// What the boot (`Store.open`) decided and now runs under.
pub const Info = struct {
    /// The quorum-resolved identity: the one the boot continued (T2) or
    /// superseded (T3).
    identity: u64,
    /// The identity the node now runs under: `identity` (T2) or one past
    /// it (T3).
    incarnation: u64,
    /// The boot's marker write: `.restarting` (T2) or `.joining` (T3).
    marker: Marker,
    /// The replayed membership state.
    member_count: u32,
};

pub const Store = struct {
    io: IO,
    dir_fd: IO.fd_t,
    fd: IO.fd_t = IO.INVALID_FILE,
    file_name: [:0]const u8,
    /// Working superblock header (TB's "working" header).
    working: SuperBlockHeader = undefined,
    /// Checkpoint-interval knob (host concern; 0 = only before WAL wrap).
    checkpoint_interval: u64 = constants.membership_checkpoint_interval_default,
    /// Replayed membership state.
    state: membership.State = .{},
    wal_last_checksum: u128 = 0,
    wal_last_sequence: u64 = 0,
    wal_count: u64 = 0,

    pub fn init(dir_path: []const u8, file_name: [:0]const u8) !Store {
        comptime {
            assert(@sizeOf(SuperBlockHeader) == copy_size);
        }
        var self: Store = .{
            .io = try IO.init(io_entries_max, 0),
            .dir_fd = undefined,
            .file_name = file_name,
        };
        errdefer self.io.deinit();

        self.dir_fd = try IO.open_dir(dir_path);
        errdefer posix.close(self.dir_fd);
        return self;
    }

    pub fn deinit(self: *Store) void {
        posix.close(self.fd);
        posix.close(self.dir_fd);
        self.io.deinit();
    }

    /// Format a fresh data file: four copies of the first superblock
    /// header — the pristine DRAINED copyset, marker `stopped` (everything
    /// format wrote is already on disk through the sync path, so the empty
    /// WAL is vouchsafed and the first boot's 2-of-4 `stopped` read is the
    /// clean stop) — a zeroed WAL, and the empty checkpoint block.
    pub fn format(self: *Store, cluster: u128, incarnation: u64) !void {
        var header = SuperBlockHeader{
            .version = superblock_mod.SuperBlockVersion,
            .release_format = vsr.Release.minimum,
            .sequence = 0,
            .cluster = cluster,
            .parent = 0,
            .uvrr_incarnation = incarnation,
            .uvrr_marker = @intFromEnum(Marker.stopped),
            .vsr_state = VSRStateForCluster(cluster),
            .view_headers_all = @splat(std.mem.zeroes(vsr.Header.Prepare)),
        };
        header.set_checksum();

        self.fd = try IO.open_data_file(
            &self.io,
            self.dir_fd,
            self.file_name,
            data_file_size_min,
            .format,
            .direct_io_required,
        );

        try self.write_all_copies(&header);
        try self.checkpoint();
        self.working = header;
        self.wal_last_checksum = 0;
        self.wal_last_sequence = 0;
        self.wal_count = 0;
    }

    /// THE BOOT (§5.1's decision table): read all four copies, resolve the
    /// working quorum at the open threshold (2-of-4, over the sequence
    /// hash-chain — every identity bump is a new parent-chained sequence,
    /// so the working copyset carries the higher-identity-wins
    /// resolution), read the verdict off the winning cohort's markers
    /// (2-of-4 `stopped` ⟺ the clean stop), then write the decision 4x:
    /// `.restarting` under the same identity (T2) or `.joining` under a
    /// bumped one (T3). The uniform 4x write is the repair — every copy
    /// that disagreed with the working quorum is rewritten, 4-of-4,
    /// satisfying the repair-to-≥3-of-4 behaviour. Finally replay
    /// checkpoint + WAL suffix into the membership state: a `stopped`
    /// marker vouches for the WAL under it (T1's drain proof), and a
    /// bumped identity still replays its past-life state — the
    /// announcement carries it.
    pub fn open(self: *Store) !Info {
        // The store owns one data-file fd; opening replaces it.
        if (self.fd != IO.INVALID_FILE) {
            posix.close(self.fd);
            self.fd = IO.INVALID_FILE;
        }
        self.fd = try IO.open_data_file(
            &self.io,
            self.dir_fd,
            self.file_name,
            data_file_size_min,
            .open,
            .direct_io_required,
        );

        var copies: [superblock_copies]SuperBlockHeader = undefined;
        for (&copies, 0..) |*copy, i| {
            copy.* = try self.read_copy(@intCast(i));
            copy.copy = @intCast(i); // checksum excludes `copy` (TB design)
        }

        // The working quorum at the open threshold: 2 of 4.
        var quorums = superblock_mod.Quorums{};
        const working = try quorums.working(&copies, .open);

        // The cohort view: the working copyset's (identity, marker) pairs
        // — the Rust twin's `SuperblockCopies`. Copies outside the working
        // copyset (torn sectors, stale sequences) do not vote.
        var view: CohortView = @splat(null);
        for (&copies, 0..) |*copy, i| {
            if (copy.valid_checksum() and copy.sequence == working.header.sequence) {
                view[i] = .{
                    .identity = copy.uvrr_incarnation,
                    // The marker byte sits under the checksum mask, so a
                    // byte that is not a written state still decodes; it
                    // votes in its cohort but never as `stopped`.
                    .marker = @enumFromInt(copy.uvrr_marker),
                };
            }
        }

        const outcome = try restart(view);
        const quorum_identity = switch (outcome.decision) {
            .continued => |identity| identity,
            .bumped => |pair| pair.old,
        };
        const boot_marker: Marker = switch (outcome.decision) {
            .continued => .restarting,
            .bumped => .joining,
        };

        // The boot write: a new copyset — sequence advanced, parent
        // hash-chained — carrying the decision. Durable-on-write through
        // the IO layer's sync path, and the only disk traffic the boot
        // produces.
        var header = working.header.*;
        header.sequence = working.header.sequence + 1;
        header.parent = working.header.checksum;
        header.uvrr_incarnation = switch (outcome.decision) {
            .continued => quorum_identity,
            .bumped => |pair| pair.new,
        };
        header.uvrr_marker = @intFromEnum(boot_marker);
        header.set_checksum();
        try self.write_all_copies(&header);
        self.working = header;

        try self.load_membership();

        return .{
            .identity = quorum_identity,
            .incarnation = header.uvrr_incarnation,
            .marker = boot_marker,
            .member_count = self.state.count,
        };
    }

    /// T1's first half (§5.1; the Rust twin's `begin_stop`): the stop
    /// command — `Running ──stop──> stopping`, written to all four copies.
    /// The node has stopped sending; no disk flush sits on the protocol's
    /// hot path. The drain — flushes of the WALs and grids — is the HOST's
    /// and sits strictly between this write and `finish_stop`.
    pub fn begin_stop(self: *Store) !void {
        try self.advance_marker(.stopping);
    }

    /// T1's second half (§5.1; the Rust twin's `finish_stop`): the drain's
    /// proof — `stopping ──drain──> stopped`, written to all four copies,
    /// callable only AFTER the host drain completed. The marker order IS
    /// the drain's proof: the drain happened strictly between the
    /// `stopping` write and this one, so a `stopped` copy vouches for the
    /// WAL under it, and 2-of-4 `stopped` at the next boot proves the
    /// clean stop with no amnesiac risk. A stop that dies partway still
    /// reads clean on the surviving quorum, correctly: the flush had
    /// already completed before the first `stopped` write.
    pub fn finish_stop(self: *Store) !void {
        try self.advance_marker(.stopped);
    }

    /// One marker transition: a new copyset — sequence advanced, parent
    /// hash-chained — carrying `marker` under the live identity, written
    /// to all four copies through the IO layer's sync path
    /// (durable-on-write).
    fn advance_marker(self: *Store, marker: Marker) !void {
        var header = self.working;
        header.sequence = self.working.sequence + 1;
        header.parent = self.working.checksum;
        header.uvrr_marker = @intFromEnum(marker);
        header.set_checksum();
        try self.write_all_copies(&header);
        self.working = header;
    }

    /// Apply one membership op: append to the ops WAL (hash-chained), then
    /// apply to the in-memory state. Checkpoint policy runs BEFORE the WAL
    /// wraps (live entries are never overwritten; the WAL restarts at slot 0
    /// after each checkpoint) or every `checkpoint_interval` ops.
    pub fn op(self: *Store, wal_op: membership.Op, identity: u128, weight: u16, learner: bool) !void {
        if (self.wal_count == membership.wal_slot_count) {
            try self.checkpoint();
        } else if (self.checkpoint_interval > 0 and
            self.wal_count > 0 and
            self.wal_count % self.checkpoint_interval == 0)
        {
            try self.checkpoint();
        }

        var slot = membership.WALSlot{
            .parent = self.wal_last_checksum,
            .sequence = self.wal_last_sequence + 1,
            .member_identity = identity,
            .weight = weight,
            .learner = @intFromBool(learner),
            .op = wal_op,
        };
        try self.state.apply(wal_op, identity, weight, learner);
        slot.set_checksum();

        const offset = vsr.Zone.start(.membership_wal) +
            (self.wal_count % membership.wal_slot_count) * membership.wal_slot_size;
        try self.write_sectors(&slot, offset);

        self.wal_last_checksum = slot.checksum;
        self.wal_last_sequence = slot.sequence;
        self.wal_count += 1;
    }

    /// Flush the FULL membership state into the ONE 4KiB checkpoint block and
    /// restart the WAL at slot 0, chained from the checkpoint.
    pub fn checkpoint(self: *Store) !void {
        const block = membership.Checkpoint.encode(
            &self.state,
            self.wal_last_checksum,
            self.wal_last_sequence,
        );
        try self.write_sectors(&block, vsr.Zone.start(.checkpoint));
        self.wal_count = 0;
    }

    fn load_membership(self: *Store) !void {
        var block: membership.Checkpoint align(16) = std.mem.zeroes(membership.Checkpoint);
        try self.read_sectors(std.mem.asBytes(&block), vsr.Zone.start(.checkpoint));
        const decoded = membership.Checkpoint.decode(&block) catch return error.QuorumLost;

        var slots: [membership.wal_slot_count]membership.WALSlot align(16) =
            std.mem.zeroes([membership.wal_slot_count]membership.WALSlot);
        try self.read_sectors(std.mem.asBytes(&slots), vsr.Zone.start(.membership_wal));

        const replayed = membership.replay(
            decoded.state,
            decoded.wal_last_checksum,
            decoded.wal_last_sequence,
            slots[0..],
        ) catch return error.QuorumLost;

        self.state = replayed.state;
        self.wal_last_checksum = replayed.wal_last_checksum;
        self.wal_last_sequence = replayed.wal_last_sequence;
        self.wal_count = replayed.wal_count;
    }

    fn write_all_copies(self: *Store, header: *SuperBlockHeader) !void {
        var copies: [superblock_copies]u8 = undefined;
        for (&copies, 0..) |*c, i| c.* = @intCast(i);
        try self.write_copies(header, copies[0..]);
    }

    fn write_copies(self: *Store, header: *SuperBlockHeader, copies: []const u8) !void {
        for (copies) |copy| {
            var staged = header.*;
            staged.copy = copy;
            // The checksum excludes `copy` (TB design): one checksum covers
            // all four copies.
            try self.write_sectors(&staged, superblock_mod.copy_offset(copy));
        }
        // The marker machine's writes are uniform 4x (the boot write and
        // T1's pair), which repairs every disagreeing copy; a partial
        // write is the torn-write case the boot's cohort read resolves.
        assert(copies.len <= superblock_copies);
    }

    fn read_copy(self: *Store, copy: u32) !SuperBlockHeader {
        var header: SuperBlockHeader align(16) = undefined;
        try self.read_sectors(std.mem.asBytes(&header), superblock_mod.copy_offset(copy));
        return header;
    }

    fn write_sectors(self: *Store, buffer: anytype, offset: u64) !void {
        const bytes = std.mem.asBytes(buffer);
        assert(bytes.len % constants.sector_size == 0);
        var ctx = IoContext{};
        var completion: IO.Completion = undefined;
        self.io.write(
            *IoContext,
            &ctx,
            write_callback,
            &completion,
            self.fd,
            bytes,
            offset,
        );
        try self.drive(&ctx);
        if (ctx.err) return error.IoWriteFailed;
    }

    fn read_sectors(self: *Store, buffer: []u8, offset: u64) !void {
        assert(buffer.len % constants.sector_size == 0);
        var ctx = IoContext{};
        var completion: IO.Completion = undefined;
        self.io.read(
            *IoContext,
            &ctx,
            read_callback,
            &completion,
            self.fd,
            buffer,
            offset,
        );
        try self.drive(&ctx);
        if (ctx.err) return error.IoReadFailed;
    }

    const IoContext = struct {
        done: bool = false,
        err: bool = false,
    };

    fn write_callback(
        ctx: *IoContext,
        completion: *IO.Completion,
        result: IO.WriteError!usize,
    ) void {
        _ = completion;
        if (result) |_| {
            ctx.done = true;
        } else |_| {
            ctx.err = true;
            ctx.done = true;
        }
    }

    fn read_callback(
        ctx: *IoContext,
        completion: *IO.Completion,
        result: IO.ReadError!usize,
    ) void {
        _ = completion;
        if (result) |_| {
            ctx.done = true;
        } else |_| {
            ctx.err = true;
            ctx.done = true;
        }
    }

    fn drive(self: *Store, ctx: *IoContext) !void {
        var attempts: usize = 0;
        while (!ctx.done) {
            attempts += 1;
            if (attempts > 1_000_000) return error.IoTimeout;
            try self.io.run_for_ns(1_000);
        }
    }
};

/// The VSRState for our demo cluster (root members derived from the
/// cluster id).
fn VSRStateForCluster(cluster: u128) superblock_mod.VSRState {
    const members = vsr.root_members(cluster);
    return superblock_mod.VSRState.root(.{
        .cluster = cluster,
        .replica_id = members[0],
        .members = members,
        .replica_count = @min(4, constants.replicas_max),
        .view = 0,
    });
}

// ---------------------------------------------------------------------------
// Tests (the Zig convention: with the module). The pure machine is driven
// over the closed 4-copy domain exactly as the Rust twin's exhaustive
// assignment corpus (tests/reincarnation.rs, class D); the disk paths pin
// T1/T2/T3 through the real IO layer.
// ---------------------------------------------------------------------------

const testing = std.testing;

const test_file_name: [:0]const u8 = "uvrr-store-test.bin";
const test_cluster: u128 = 0xBEEF;

/// A uniform copyset at one identity, `marks` in slot order.
fn copiesOf(marks: [superblock_copies]Marker, identity: u64) CohortView {
    var view: CohortView = @splat(null);
    for (&view, marks) |*slot, mark| slot.* = .{ .identity = identity, .marker = mark };
    return view;
}

/// The four written marker states, in tag order (the closed domain).
const marker_states = [_]Marker{ .stopping, .stopped, .restarting, .joining };

test "the closed 4-copy domain is exhaustive: 2-of-4 stopped is the clean stop, every other assignment bumps" {
    // Every assignment of the four marker states to four copies — 4⁴ =
    // 256, all cheap, no IO. For each assignment: the verdict is `stopped`
    // ⟺ ≥2 copies hold `stopped`; the quorum-resolved identity is the
    // single written identity; a stopped quorum continues under it and
    // writes `restarting` 4x; every non-clean assignment bumps — a crash,
    // a torn marker set, and death mid-join all reincarnate — and writes
    // `joining` 4x.
    for (0..256) |bits| {
        const marks = [superblock_copies]Marker{
            marker_states[bits & 3],
            marker_states[(bits >> 2) & 3],
            marker_states[(bits >> 4) & 3],
            marker_states[(bits >> 6) & 3],
        };
        const stored = copiesOf(marks, 7);
        var stopped_count: u8 = 0;
        for (marks) |mark| stopped_count += @intFromBool(mark == .stopped);
        const clean = stopped_count >= 2;
        const found = classify(stored) orelse return error.TestUnexpectedResult;
        try testing.expectEqual(@as(u64, 7), found.identity);
        try testing.expectEqual(clean, found.class == .stopped);
        const outcome = try restart(stored);
        switch (outcome.decision) {
            .continued => |identity| {
                try testing.expectEqual(@as(u64, 7), identity);
                try testing.expect(clean);
                for (outcome.written) |copy| {
                    try testing.expectEqual(@as(u64, 7), copy.identity);
                    try testing.expectEqual(Marker.restarting, copy.marker);
                }
            },
            .bumped => |pair| {
                try testing.expect(!clean);
                try testing.expectEqual(@as(u64, 7), pair.old);
                try testing.expectEqual(@as(u64, 8), pair.new);
                for (outcome.written) |copy| {
                    try testing.expectEqual(@as(u64, 8), copy.identity);
                    try testing.expectEqual(Marker.joining, copy.marker);
                }
            },
        }
    }

    // Death mid-join: all-`joining` reads no stopped quorum, bumps, writes
    // `joining` again — and the rewritten set reincarnates AGAIN; the
    // commitment never wedges.
    const mid_join = copiesOf(.{ .joining, .joining, .joining, .joining }, 7);
    const first = try restart(mid_join);
    try testing.expectEqual(@as(u64, 7), first.decision.bumped.old);
    try testing.expectEqual(@as(u64, 8), first.decision.bumped.new);
    const second = try restart(copiesOf(
        .{ .joining, .joining, .joining, .joining },
        first.decision.bumped.new,
    ));
    try testing.expectEqual(@as(u64, 8), second.decision.bumped.old);
    try testing.expectEqual(@as(u64, 9), second.decision.bumped.new);
    for (second.written) |copy| {
        try testing.expectEqual(@as(u64, 9), copy.identity);
        try testing.expectEqual(Marker.joining, copy.marker);
    }
}

test "the identity is resolved inside the working quorum: a lone higher copy cannot impose" {
    // The highest identity whose cohort reaches the open threshold wins;
    // a copy whose cohort falls short cannot impose it — never
    // highest-observed-across-all.
    var higher_cohort = copiesOf(.{ .joining, .joining, .joining, .joining }, 7);
    higher_cohort[0] = .{ .identity = 9, .marker = .joining };
    higher_cohort[1] = .{ .identity = 9, .marker = .joining };
    const found = classify(higher_cohort) orelse return error.TestUnexpectedResult;
    try testing.expectEqual(RestartClass.not_stopped, found.class);
    try testing.expectEqual(@as(u64, 9), found.identity);
    const outcome = try restart(higher_cohort);
    try testing.expectEqual(@as(u64, 9), outcome.decision.bumped.old);
    try testing.expectEqual(@as(u64, 10), outcome.decision.bumped.new);

    var lone_higher = copiesOf(.{ .joining, .joining, .joining, .joining }, 7);
    lone_higher[2] = .{ .identity = 12, .marker = .joining };
    const found_lone = classify(lone_higher) orelse return error.TestUnexpectedResult;
    try testing.expectEqual(@as(u64, 7), found_lone.identity);
    const lone = try restart(lone_higher);
    try testing.expectEqual(@as(u64, 7), lone.decision.bumped.old);
    try testing.expectEqual(@as(u64, 8), lone.decision.bumped.new);

    // A marker byte that is not a written state votes in its identity
    // cohort but never as `stopped`: the rotted copy still carries its
    // checksummed identity, so the higher band cannot silently lose its
    // quorum and re-enter circulation.
    var rotted = copiesOf(.{ .stopped, .stopped, .stopped, .stopped }, 7);
    rotted[0] = .{ .identity = 9, .marker = @enumFromInt(77) };
    rotted[1] = .{ .identity = 9, .marker = .stopped };
    const found_rotted = classify(rotted) orelse return error.TestUnexpectedResult;
    try testing.expectEqual(@as(u64, 9), found_rotted.identity);
    try testing.expectEqual(RestartClass.not_stopped, found_rotted.class);
    const bumped = try restart(rotted);
    try testing.expectEqual(@as(u64, 9), bumped.decision.bumped.old);
    try testing.expectEqual(@as(u64, 10), bumped.decision.bumped.new);
}

test "no cohort reaching the open threshold is quorum lost" {
    // Four copies, four identities: no cohort reaches 2-of-4.
    const torn: CohortView = .{
        .{ .identity = 1, .marker = .stopped },
        .{ .identity = 2, .marker = .stopped },
        .{ .identity = 3, .marker = .stopped },
        .{ .identity = 4, .marker = .stopped },
    };
    try testing.expect(classify(torn) == null);
    try testing.expectError(error.QuorumLost, restart(torn));

    // No copies at all: nothing reaches the threshold.
    const empty: CohortView = @splat(null);
    try testing.expect(classify(empty) == null);
    try testing.expectError(error.QuorumLost, restart(empty));
}

test "the identity space is checked: exhaustion refuses rather than wraps" {
    const stored = copiesOf(.{ .joining, .joining, .joining, .joining }, std.math.maxInt(u64));
    try testing.expectError(error.IdentityExhausted, restart(stored));
    // A clean stop at the top of the space still continues — no bump.
    const clean = copiesOf(.{ .stopped, .stopped, .restarting, .joining }, std.math.maxInt(u64));
    const outcome = try restart(clean);
    try testing.expectEqual(std.math.maxInt(u64), outcome.decision.continued);
}

test "continuation commitment: the identity only moves forward" {
    // No stopped quorum: bump; bump again — the identity never regresses.
    const marks = [superblock_copies]Marker{ .restarting, .restarting, .restarting, .stopping };
    const first = try restart(copiesOf(marks, 7));
    const new = first.decision.bumped.new;
    const second = try restart(copiesOf(marks, new));
    try testing.expectEqual(new, second.decision.bumped.old);
    try testing.expectEqual(new + 1, second.decision.bumped.new);
    // A clean stop then a restart continues the committed identity — never
    // a reversion to anything lower.
    const stopped = copiesOf(.{ .stopped, .stopped, .stopped, .stopped }, new + 1);
    const third = try restart(stopped);
    try testing.expectEqual(new + 1, third.decision.continued);
    for (third.written) |copy| try testing.expectEqual(Marker.restarting, copy.marker);
}

/// A store over a fresh testing directory, with the data file formatted at
/// `incarnation` (the pristine drained copyset). The directory path is
/// carried by value, not borrowed: each reopen opens the same path by
/// name through a fresh `Store.init`.
const TestStore = struct {
    tmp: testing.TmpDir,
    dir_path_buf: [std.fs.max_path_bytes]u8,
    dir_path_len: usize,
    store: Store,

    fn deinit(self: *TestStore) void {
        self.store.deinit();
        self.tmp.cleanup();
    }

    fn dirPath(self: *const TestStore) []const u8 {
        return self.dir_path_buf[0..self.dir_path_len];
    }

    /// Close the current store and open the same data file again (the
    /// directory's flock is per-fd, so the old store must close first).
    fn reopen(self: *TestStore) !Store {
        self.store.deinit();
        return Store.init(self.dirPath(), test_file_name);
    }
};

fn formattedStore(incarnation: u64) !TestStore {
    var tmp = testing.tmpDir(.{});
    errdefer tmp.cleanup();
    var buf: [std.fs.max_path_bytes]u8 = undefined;
    const dir_path = try tmp.dir.realpath(".", &buf);
    var store = try Store.init(dir_path, test_file_name);
    errdefer store.deinit();
    try store.format(test_cluster, incarnation);
    return .{
        .tmp = tmp,
        .dir_path_buf = buf,
        .dir_path_len = dir_path.len,
        .store = store,
    };
}

test "the boot continues a stopped store under the same identity and writes restarting" {
    // T2 on disk: the pristine `stopped` copyset (and the stopped copyset
    // T1 leaves) reads clean at 2-of-4, so the node continues under the
    // same identity and writes `restarting` 4x.
    var ts = try formattedStore(1);
    defer ts.deinit();

    const info = try ts.store.open();
    try testing.expectEqual(@as(u64, 1), info.identity);
    try testing.expectEqual(@as(u64, 1), info.incarnation);
    try testing.expectEqual(Marker.restarting, info.marker);
    for (0..superblock_copies) |i| {
        const copy = try ts.store.read_copy(@intCast(i));
        try testing.expect(copy.valid_checksum());
        try testing.expectEqual(@intFromEnum(Marker.restarting), copy.uvrr_marker);
        try testing.expectEqual(@as(u64, 1), copy.uvrr_incarnation);
    }

    // The stop path T1: `stopping` 4x, then — the drain is the host's —
    // `stopped` 4x, each a new parent-chained copyset.
    try ts.store.begin_stop();
    const stopping = ts.store.working;
    try testing.expectEqual(@intFromEnum(Marker.stopping), stopping.uvrr_marker);
    for (0..superblock_copies) |i| {
        const copy = try ts.store.read_copy(@intCast(i));
        try testing.expectEqual(@intFromEnum(Marker.stopping), copy.uvrr_marker);
    }
    try ts.store.finish_stop();
    try testing.expectEqual(@intFromEnum(Marker.stopped), ts.store.working.uvrr_marker);
    try testing.expect(ts.store.working.sequence == stopping.sequence + 1);
    try testing.expect(ts.store.working.parent == stopping.checksum);
    for (0..superblock_copies) |i| {
        const copy = try ts.store.read_copy(@intCast(i));
        try testing.expectEqual(@intFromEnum(Marker.stopped), copy.uvrr_marker);
    }

    // The controlled restart boots clean and continues the identity.
    ts.store = try ts.reopen();
    const again = try ts.store.open();
    try testing.expectEqual(@as(u64, 1), again.identity);
    try testing.expectEqual(@as(u64, 1), again.incarnation);
    try testing.expectEqual(Marker.restarting, again.marker);
}

test "a crash while running bumps the identity and writes joining" {
    // T3 on disk: a node whose markers hold `restarting` (the running
    // state's boot write) proves no controlled shutdown, so the identity
    // is dead — bump, `joining` 4x — and death mid-join reincarnates
    // again, the identity only moving forward.
    var ts = try formattedStore(1);
    defer ts.deinit();

    _ = try ts.store.open(); // the running state's boot write: restarting 4x
    ts.store = try ts.reopen(); // the crash: no stop, markers unchanged
    const first = try ts.store.open();
    try testing.expectEqual(@as(u64, 1), first.identity);
    try testing.expectEqual(@as(u64, 2), first.incarnation);
    try testing.expectEqual(Marker.joining, first.marker);
    for (0..superblock_copies) |i| {
        const copy = try ts.store.read_copy(@intCast(i));
        try testing.expectEqual(@intFromEnum(Marker.joining), copy.uvrr_marker);
        try testing.expectEqual(@as(u64, 2), copy.uvrr_incarnation);
    }

    // Death mid-join: the joining copyset still proves no controlled
    // shutdown, so the node reincarnates again.
    ts.store = try ts.reopen();
    const second = try ts.store.open();
    try testing.expectEqual(@as(u64, 2), second.identity);
    try testing.expectEqual(@as(u64, 3), second.incarnation);
    try testing.expectEqual(Marker.joining, second.marker);

    // The reincarnated node stops cleanly and its next boot continues the
    // bumped identity.
    try ts.store.begin_stop();
    try ts.store.finish_stop();
    ts.store = try ts.reopen();
    const clean = try ts.store.open();
    try testing.expectEqual(@as(u64, 3), clean.identity);
    try testing.expectEqual(@as(u64, 3), clean.incarnation);
    try testing.expectEqual(Marker.restarting, clean.marker);
}

test "a torn stop reads clean on the surviving quorum" {
    // 2-of-4 on disk: `finish_stop` torn after two copies leaves a
    // stopped quorum, and the boot continues the same identity — the
    // flush had already completed before the first `stopped` write.
    var ts = try formattedStore(1);
    defer ts.deinit();

    _ = try ts.store.open();
    try ts.store.begin_stop();
    // T1's second half lands on only two copies (the crash tore it):
    var torn = ts.store.working;
    torn.sequence = ts.store.working.sequence + 1;
    torn.parent = ts.store.working.checksum;
    torn.uvrr_marker = @intFromEnum(Marker.stopped);
    torn.set_checksum();
    try ts.store.write_copies(&torn, &[_]u8{ 0, 1 });

    ts.store = try ts.reopen();
    const info = try ts.store.open();
    try testing.expectEqual(@as(u64, 1), info.identity);
    try testing.expectEqual(@as(u64, 1), info.incarnation);
    try testing.expectEqual(Marker.restarting, info.marker);
    // The boot's uniform 4x write repaired the torn pair.
    for (0..superblock_copies) |i| {
        const copy = try ts.store.read_copy(@intCast(i));
        try testing.expectEqual(@intFromEnum(Marker.restarting), copy.uvrr_marker);
    }
}

test "a stop that dies before the drain bumps" {
    // The crash tore T1 after `begin_stop` and before the drain: the
    // `stopping` copyset vouches for nothing, so the boot bumps.
    var ts = try formattedStore(1);
    defer ts.deinit();

    _ = try ts.store.open();
    try ts.store.begin_stop();
    ts.store = try ts.reopen(); // the crash: no drain, no `stopped` write

    const info = try ts.store.open();
    try testing.expectEqual(@as(u64, 1), info.identity);
    try testing.expectEqual(@as(u64, 2), info.incarnation);
    try testing.expectEqual(Marker.joining, info.marker);
}

test "a marker set torn beyond the open threshold refuses with quorum lost" {
    // One valid copy cannot reach 2-of-4: the boot refuses rather than
    // deciding on an inadequate quorum.
    var ts = try formattedStore(1);
    defer ts.deinit();

    var garbage: SuperBlockHeader align(16) = std.mem.zeroes(SuperBlockHeader);
    for (1..superblock_copies) |i| {
        try ts.store.write_sectors(&garbage, superblock_mod.copy_offset(@intCast(i)));
    }

    ts.store = try ts.reopen();
    try testing.expectError(error.QuorumLost, ts.store.open());
}

test "membership replays across the boot" {
    // The WAL under a `stopped` marker is vouchsafed (T1's drain proof):
    // the ops replay after the controlled restart, and after a bump the
    // past-life state still replays — the announcement carries it.
    var ts = try formattedStore(1);
    defer ts.deinit();

    const info = try ts.store.open();
    try testing.expectEqual(@as(u32, 0), info.member_count);
    try ts.store.op(.add_one, 1, 1, false);
    try ts.store.op(.add_one, 2, 2, false);
    try testing.expectEqual(@as(u32, 2), ts.store.state.count);
    try ts.store.begin_stop();
    try ts.store.finish_stop();

    ts.store = try ts.reopen();
    const clean = try ts.store.open();
    try testing.expectEqual(@as(u32, 2), clean.member_count);

    ts.store = try ts.reopen(); // the crash: the state survives, the identity does not
    const bumped = try ts.store.open();
    try testing.expectEqual(@as(u64, 2), bumped.incarnation);
    try testing.expectEqual(Marker.joining, bumped.marker);
    try testing.expectEqual(@as(u32, 2), bumped.member_count);
}
