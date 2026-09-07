//! Uvrr delta over TigerBeetle 0.17.9's superblock (`src/vsr/superblock.zig`).
//!
//! EXTRACTION, not reimplementation. This file imports TigerBeetle's own
//! `SuperBlockHeader`, its checksum scheme (`vsr.checksum` over the header
//! minus the first 34 bytes), its constants, its copy layout
//! (`copy_size * copy_index`) and its open quorum (2 of 4, per the
//! `superblock.zig` header table: "open: Read quorum; verify 2/4 are valid")
//! verbatim, compiled into a synchronous C-ABI static library.
//!
//! What is re-glued rather than extracted: TB's `SuperBlockType(Storage)` is
//! an asynchronous, callback-driven state machine over TB's `Storage`/`IO`
//! interfaces. This wrapper performs the same flow — read all copies, verify
//! each, adopt the highest-identity quorum, write the full copyset —
//! synchronously over one file descriptor, because the consumer is a
//! synchronous C API.
//!
//! Director's delta (the ONLY behavioral change to TB's scheme):
//!   * `SuperBlockHeader.flags` bit 0 = `flushed`. TB reserves `flags` for
//!     "future minor features"; `set_checksum` asserts `flags == 0`, so this
//!     code assigns the checksum via `calculate_checksum()` directly.
//!   * Restart: read all FOUR; if ANY valid copy is unflushed → node is dirty.
//!   * Dirty: bump the incarnation (TB's own identity counter, the header
//!     `sequence`, hash-chained via `parent`), write the new identity +
//!     `flushed` to ALL FOUR copies. Reads adopt the highest identity seen.
//!   * Clean shutdown: flush writes → fsync → mark `flushed` in all four.

const std = @import("std");
const sb = @import("vsr/superblock.zig");
const constants = @import("constants.zig");
const vsr = @import("vsr.zig");

const log = std.log.scoped(.uvrr_sb);

// Satisfies config.zig's `@hasDecl(root, "vsr_options")` build-options probe
// (TB's build.zig would normally generate this via `addOptions`).
pub const vsr_options = struct {
    pub const git_commit: ?[40]u8 = .{ '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0', '0' };
    pub const config_verify: bool = false;
    pub const release: []const u8 = "0.17.9";
    pub const release_client_min: []const u8 = "0.17.9";
};

const SuperBlockHeader = sb.SuperBlockHeader;
const VSRState = SuperBlockHeader.VSRState;
const copies = constants.superblock_copies; // 4
const copy_size = sb.superblock_copy_size;
const zone_size = sb.superblock_zone_size;

/// Director's delta flag, carried in TB's reserved-for-minor-features field.
/// Covered by the header checksum (unlike `copy`), so all copies share one
/// checksum for the same state.
pub const FLAG_FLUSHED: u64 = 1 << 0;

/// Read-out state, returned to the caller (Rust) by value.
pub const UvrrState = extern struct {
    /// The adopted incarnation (TB superblock `sequence`).
    adopted_sequence: u64 = 0,
    /// The adopted header's `flushed` bit.
    flushed: bool = false,
    /// Director's delta: ANY valid copy unflushed ⇒ dirty.
    dirty: bool = false,
    /// Bitset of copies that passed verification.
    valid_copies: u8 = 0,
    /// Bitset of valid copies that were unflushed.
    unflushed_copies: u8 = 0,
};

/// Opaque-to-Rust handle carrying the adopted header.
pub const Handle = extern struct {
    header: SuperBlockHeader,
};

// Error codes.
const err_io: c_int = -1;
const err_no_quorum: c_int = -2;

fn open_rw(path: [*:0]const u8) !std.fs.File {
    return std.fs.cwd().openFile(std.mem.span(path), .{ .mode = .read_write });
}

fn fsync_file(file: std.fs.File) !void {
    try std.posix.fsync(file.handle);
}

/// Write one copy. The checksum excludes the leading 34 bytes (checksum,
/// checksum_padding, copy), so re-tagging `copy` per copy does not change it —
/// exactly TB's "all copies have the same checksum" property.
fn write_copy(file: std.fs.File, header: *const SuperBlockHeader, copy_index: u8) !void {
    var h: SuperBlockHeader = header.*;
    h.copy = copy_index;
    var buffer: [copy_size]u8 align(@alignOf(SuperBlockHeader)) = @splat(0);
    const bytes = std.mem.asBytes(&h);
    @memcpy(buffer[0..bytes.len], bytes);
    try file.pwriteAll(&buffer, copy_size * @as(u64, copy_index));
}

/// Write the full copyset (all four copies) and make it durable.
fn write_copyset(file: std.fs.File, header: *const SuperBlockHeader) !void {
    for (0..copies) |i| try write_copy(file, header, @intCast(i));
    try fsync_file(file);
}

fn make_header(cluster: u128, sequence: u64, parent: u128, flags: u64) SuperBlockHeader {
    var header = std.mem.zeroes(SuperBlockHeader);
    const release = vsr.Release.from(.{ .major = 0, .minor = 17, .patch = 9 });
    const members = vsr.root_members(cluster);
    header.version = sb.SuperBlockVersion;
    header.release_format = release;
    header.sequence = sequence;
    header.cluster = cluster;
    header.parent = parent;
    header.flags = flags;
    header.view_headers_count = 0;
    header.vsr_state = VSRState.root(.{
        .cluster = cluster,
        .replica_id = members[0],
        .members = members,
        .replica_count = copies,
        .release = release,
        .view = 0,
    });
    // Delta note: TB's set_checksum() asserts flags == 0; the flushed bit is
    // this project's sanctioned delta, so the checksum is assigned directly.
    header.checksum = header.calculate_checksum();
    return header;
}

/// Format a fresh superblock zone: four copies at sequence 1, marked flushed
/// (a formatted-but-never-written store is clean).
/// Returns a handle; the caller frees it with uvrr_sb_close.
export fn uvrr_sb_format(
    path: [*:0]const u8,
    cluster: u64,
    out: *?*Handle,
) c_int {
    out.* = null;
    const file = std.fs.cwd().createFile(std.mem.span(path), .{
        .truncate = true,
        .read = true,
    }) catch |e| {
        log.err("format: open failed: {s} ({s})", .{ path, @errorName(e) });
        return err_io;
    };
    defer file.close();
    std.posix.ftruncate(file.handle, zone_size) catch |e| {
        log.err("format: ftruncate failed ({s})", .{@errorName(e)});
        return err_io;
    };
    const header = make_header(cluster, 1, 0, FLAG_FLUSHED);
    write_copyset(file, &header) catch |e| {
        log.err("format: write failed ({s})", .{@errorName(e)});
        return err_io;
    };
    const handle = std.heap.c_allocator.create(Handle) catch return err_io;
    handle.* = .{ .header = header };
    out.* = handle;
    log.info("format: cluster={x} sequence=1 flushed=true copies={}", .{ cluster, copies });
    return 0;
}

/// TB's startup flow: read ALL copies, verify each (misdirected-read tag,
/// version, checksum), adopt the highest sequence that a quorum of valid
/// copies agrees on (open quorum: 2 of 4). Reports the Director's dirty
/// verdict: ANY valid copy unflushed ⇒ dirty.
export fn uvrr_sb_open(
    path: [*:0]const u8,
    out: *?*Handle,
    state: *UvrrState,
) c_int {
    out.* = null;
    state.* = .{};
    const file = open_rw(path) catch |e| {
        log.err("open: failed: {s} ({s})", .{ path, @errorName(e) });
        return err_io;
    };
    defer file.close();

    const buffer = std.heap.c_allocator.alignedAlloc(
        u8,
        @alignOf(SuperBlockHeader),
        zone_size,
    ) catch return err_io;
    defer std.heap.c_allocator.free(buffer);
    @memset(buffer, 0);

    var valid: [copies]bool = @splat(false);
    var valid_bitset: u8 = 0;
    for (0..copies) |i| {
        const slice = buffer[copy_size * i .. copy_size * (i + 1)];
        const got = file.preadAll(slice, copy_size * @as(u64, @intCast(i))) catch continue;
        if (got != slice.len) continue;
        const h: *const SuperBlockHeader = @ptrCast(@alignCast(slice.ptr));
        const copy_ok = h.copy == i;
        const version_ok = h.version == sb.SuperBlockVersion;
        if (copy_ok and version_ok and h.valid_checksum()) {
            valid[i] = true;
            valid_bitset |= @as(u8, 1) << @intCast(i);
        }
    }

    // Adopt the highest sequence among valid copies (higher identity wins).
    var adopted: ?usize = null;
    for (0..copies) |i| {
        if (!valid[i]) continue;
        const h: *const SuperBlockHeader = @ptrCast(@alignCast(buffer[copy_size * i ..].ptr));
        if (adopted == null or
            h.sequence > adopted_header(buffer, adopted.?).sequence)
        {
            adopted = i;
        }
    }
    if (adopted == null) {
        log.err("open: no valid superblock copy in {s}", .{path});
        return err_io;
    }

    const adopted_h = adopted_header(buffer, adopted.?);
    // Quorum: copies agreeing with the adopted identity (sequence + checksum).
    var agreeing: u8 = 0;
    for (0..copies) |i| {
        if (!valid[i]) continue;
        const h: *const SuperBlockHeader = @ptrCast(@alignCast(buffer[copy_size * i ..].ptr));
        if (h.sequence == adopted_h.sequence and h.checksum == adopted_h.checksum) {
            agreeing += 1;
        }
    }
    // TB open quorum: 2 of 4 ("Read quorum; verify 2/4 are valid").
    if (agreeing < 2) {
        log.err("open: no quorum: agreeing={} valid=0b{b:0>4}", .{ agreeing, valid_bitset });
        return err_no_quorum;
    }

    var unflushed_bitset: u8 = 0;
    var any_unflushed = false;
    for (0..copies) |i| {
        if (!valid[i]) continue;
        const h: *const SuperBlockHeader = @ptrCast(@alignCast(buffer[copy_size * i ..].ptr));
        if (h.flags & FLAG_FLUSHED == 0) {
            any_unflushed = true;
            unflushed_bitset |= @as(u8, 1) << @intCast(i);
        }
    }

    state.adopted_sequence = adopted_h.sequence;
    state.flushed = adopted_h.flags & FLAG_FLUSHED != 0;
    state.dirty = any_unflushed;
    state.valid_copies = valid_bitset;
    state.unflushed_copies = unflushed_bitset;

    const handle = std.heap.c_allocator.create(Handle) catch return err_io;
    handle.* = .{ .header = adopted_h.* };
    out.* = handle;
    log.info("open: sequence={} flushed={} dirty={} valid=0b{b:0>4} unflushed=0b{b:0>4}", .{
        state.adopted_sequence,
        state.flushed,
        state.dirty,
        valid_bitset,
        unflushed_bitset,
    });
    return 0;
}

fn adopted_header(buffer: []align(@alignOf(SuperBlockHeader)) u8, index: usize) *const SuperBlockHeader {
    return @ptrCast(@alignCast(buffer[copy_size * index ..].ptr));
}

/// Director's dirty path: bump the incarnation. The header `sequence` is TB's
/// own monotonic superblock identity; `parent` hash-chains to the previous
/// state, exactly as TB's checkpoint()/view_change() do. Writes the new
/// identity + `flushed` to ALL FOUR copies and fsyncs.
export fn uvrr_sb_bump(handle: *Handle, path: [*:0]const u8) c_int {
    const old = handle.header;
    const old_sequence = old.sequence;
    const new = make_header(old.cluster, old.sequence + 1, old.checksum, FLAG_FLUSHED);
    const file = open_rw(path) catch |e| {
        log.err("bump: open failed ({s})", .{@errorName(e)});
        return err_io;
    };
    defer file.close();
    write_copyset(file, &new) catch |e| {
        log.err("bump: write failed ({s})", .{@errorName(e)});
        return err_io;
    };
    handle.header = new;
    log.info("bump: reincarnation {} -> {} (parent={x:0>32})", .{
        old_sequence,
        new.sequence,
        new.parent,
    });
    return 0;
}

/// Director's shutdown semantics. `flushed=true` (clean shutdown): flush
/// writes → fsync → mark `flushed` in all four copies. `flushed=false`
/// simulates in-flight (unflushed) state, e.g. a crash mid-write.
export fn uvrr_sb_set_flushed(handle: *Handle, path: [*:0]const u8, flushed: bool) c_int {
    var h = handle.header;
    if (flushed) {
        h.flags |= FLAG_FLUSHED;
    } else {
        h.flags &= ~FLAG_FLUSHED;
    }
    h.checksum = h.calculate_checksum();
    const file = open_rw(path) catch |e| {
        log.err("set_flushed: open failed ({s})", .{@errorName(e)});
        return err_io;
    };
    defer file.close();
    write_copyset(file, &h) catch |e| {
        log.err("set_flushed: write failed ({s})", .{@errorName(e)});
        return err_io;
    };
    handle.header = h;
    log.info("set_flushed: sequence={} flushed={} (fsynced)", .{ h.sequence, flushed });
    return 0;
}

export fn uvrr_sb_close(handle: *Handle) void {
    std.heap.c_allocator.destroy(handle);
}

export fn uvrr_sb_copy_size() u64 {
    return copy_size;
}

comptime {
    _ = copies;
    _ = zone_size;
}
