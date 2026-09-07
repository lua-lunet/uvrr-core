//! NEW (item31) — zig/uvrr/store.zig
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
//! an 8 KiB membership ops WAL (16 sector-sized slots), one 4 KiB checkpoint
//! block.

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
pub const checksum = vsr.checksum;

pub const Info = struct {
    sequence: u64,
    incarnation: u64,
    flushed: bool,
    dirty: bool,
    member_count: u32,
};

const io_entries_max: u12 = 64;

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
    /// Set when the latest open() observed any unflushed or missing copy.
    dirty: bool = false,

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

    /// Format a fresh data file: four copies of the first (empty, flushed)
    /// superblock header, a zeroed WAL, and the empty checkpoint block.
    pub fn format(self: *Store, cluster: u128, incarnation: u64) !void {
        var header = SuperBlockHeader{
            .version = superblock_mod.SuperBlockVersion,
            .release_format = vsr.Release.minimum,
            .sequence = 0,
            .cluster = cluster,
            .parent = 0,
            .uvrr_incarnation = incarnation,
            .uvrr_flushed = 1,
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
        self.dirty = false;
    }

    /// Open an existing data file: read all four copies, apply the
    /// higher-identity-wins rule within the highest-sequence quorum, repair
    /// the remaining copies (TB's open flow: read quorum 2/4, repair to 3/4+),
    /// then replay checkpoint + WAL suffix into the membership state.
    pub fn open(self: *Store) !Info {
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

        var any_unflushed = false;
        for (copies) |copy| {
            if (copy.valid_checksum() and copy.uvrr_flushed == 0) any_unflushed = true;
        }

        // TB's vendored quorum logic: open threshold = 2 of 4.
        var quorums = superblock_mod.Quorums{};
        const working = try quorums.working(&copies, .open);

        // PATCH (higher-identity-wins): among valid copies at the working
        // sequence, the highest uvrr_incarnation wins; the winner must itself
        // reach the open quorum.
        var best: ?SuperBlockHeader = null;
        for (copies) |copy| {
            if (!copy.valid_checksum()) continue;
            if (copy.sequence != working.header.sequence) continue;
            if (best == null or copy.uvrr_incarnation > best.?.uvrr_incarnation or
                (copy.uvrr_incarnation == best.?.uvrr_incarnation and
                    copy.uvrr_flushed > best.?.uvrr_flushed))
            {
                best = copy;
            }
        }
        if (best == null) return error.QuorumLost;

        var matched: u8 = 0;
        for (copies) |copy| {
            if (copy.valid_checksum() and
                copy.sequence == best.?.sequence and
                copy.uvrr_incarnation == best.?.uvrr_incarnation)
            {
                matched += 1;
            }
        }
        if (matched < superblock_mod.Quorums.Threshold.open.count()) return error.QuorumLost;

        self.working = best.?;
        self.dirty = any_unflushed;

        try self.load_membership();

        // Repair any broken/missing copy from the winning header (3/4+):
        var repaired: [superblock_copies]u8 = undefined;
        var repaired_count: usize = 0;
        for (&copies, 0..) |*copy, i| {
            if (!copy.valid_checksum() or
                copy.sequence != self.working.sequence or
                copy.uvrr_incarnation != self.working.uvrr_incarnation)
            {
                repaired[repaired_count] = @intCast(i);
                repaired_count += 1;
            }
        }
        try self.write_copies(&self.working, repaired[0..repaired_count]);

        return .{
            .sequence = self.working.sequence,
            .incarnation = self.working.uvrr_incarnation,
            .flushed = self.working.uvrr_flushed == 1,
            .dirty = self.dirty,
            .member_count = self.state.count,
        };
    }

    /// Demo helper: rewrite copy `index` with a consistent unflushed header.
    pub fn simulate_dirty_copy(self: *Store, index: u32) !void {
        var header = try self.read_copy(index);
        if (!header.valid_checksum()) return error.CorruptCopy;
        header.uvrr_flushed = 0;
        header.set_checksum();
        var copies: [1]u8 = .{@intCast(index)};
        try self.write_copies(&header, copies[0..]);
    }

    /// Dirty restart: bump the incarnation, clear the flushed mark, advance
    /// the sequence hash-chain, rewrite ALL FOUR copies.
    pub fn dirty_restart(self: *Store) !void {
        var header = self.working;
        header.sequence += 1;
        header.parent = self.working.checksum;
        header.uvrr_incarnation += 1;
        header.uvrr_flushed = 0;
        header.set_checksum();

        try self.write_all_copies(&header);
        self.working = header;
        self.dirty = true;
    }

    /// Clean shutdown: advance the sequence hash-chain, set the flushed mark,
    /// rewrite all four copies (the write quorum 3/4 must persist), sync.
    pub fn clean_shutdown(self: *Store) !void {
        var header = self.working;
        header.sequence += 1;
        header.parent = self.working.checksum;
        header.uvrr_flushed = 1;
        header.set_checksum();

        try self.write_all_copies(&header);
        self.working = header;
        self.dirty = false;
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
        // Repair writes only the broken/missing copies; the on-disk quorum was
        // already >= the open threshold when this flow was reached. Format
        // writes the full copyset.
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

/// PATCH: the VSRState for our demo cluster (root members derived from the
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
