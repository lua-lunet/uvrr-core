//! NEW  — zig/uvrr/capi.zig
//!
//! C-ABI entry points for the vendored UVRR stack (TB IO + superblock +
//! membership WAL). `pub export` + `callconv(.C)`; the Rust demo binds these
//! via `extern "C"` with C types on both sides. One store handle (single
//! open data file; not thread-safe by design).

const std = @import("std");
const membership = @import("membership.zig");
const store_mod = @import("store.zig");
const constants = @import("../constants.zig");
const superblock_mod = @import("../vsr/superblock.zig");
const vsr = @import("../vsr.zig");

const Store = store_mod.Store;
const Op = membership.Op;

/// The global handle: the demo drives one store at a time.
var store: ?Store = null;

pub export fn uvrr_data_file_size() callconv(.C) u64 {
    return store_mod.data_file_size_min;
}

pub export fn uvrr_copy_size() callconv(.C) u64 {
    return store_mod.copy_size;
}

pub export fn uvrr_superblock_copies() callconv(.C) u32 {
    return constants.superblock_copies;
}

/// Format a fresh data file: the pristine drained copyset (marker
/// `stopped`) at `incarnation`, which seeds the reincarnation identity.
/// Returns 0 on success, -1 on error.
pub export fn uvrr_format(
    dir_path: [*:0]const u8,
    file_name: [*:0]const u8,
    cluster_lo: u64,
    cluster_hi: u64,
    incarnation: u64,
) callconv(.C) i32 {
    close();
    store = Store.init(std.mem.span(dir_path), std.mem.span(file_name)) catch return -1;
    const s = &store.?;
    s.format(u128From(cluster_lo, cluster_hi), incarnation) catch return -1;
    return 0;
}

/// Open an existing data file: THE BOOT (§5.1 of
/// `docs/vrr-durability-model.md`). The working quorum at the open
/// threshold (2-of-4) resolves the identity; 2-of-4 `stopped` is the
/// clean stop and continues under the same identity, writing `restarting`
/// 4x; anything else bumps the identity and writes `joining` 4x. Returns
/// 0 on success.
pub export fn uvrr_open(
    dir_path: [*:0]const u8,
    file_name: [*:0]const u8,
    cluster_lo: u64,
    cluster_hi: u64,
) callconv(.C) i32 {
    close();
    store = Store.init(std.mem.span(dir_path), std.mem.span(file_name)) catch return -1;
    const s = &store.?;
    const cluster = u128From(cluster_lo, cluster_hi);
    _ = s.open() catch return -1;
    if (s.working.cluster != cluster) return -1;
    return 0;
}

/// T1's first half: the stop command — `stopping` 4x. The host drain
/// (flush WALs and grids) sits strictly between this and
/// `uvrr_finish_stop`.
pub export fn uvrr_begin_stop() callconv(.C) i32 {
    const s = &store.?;
    s.begin_stop() catch return -1;
    return 0;
}

/// T1's second half: the drain's proof — `stopped` 4x, callable only
/// after the host drain completed. The marker order is the drain's proof:
/// a `stopped` copy vouches for the WAL under it.
pub export fn uvrr_finish_stop() callconv(.C) i32 {
    const s = &store.?;
    s.finish_stop() catch return -1;
    return 0;
}

/// Membership ops (the voting-weights moves).
pub export fn uvrr_op(
    op: i32,
    identity_lo: u64,
    identity_hi: u64,
    weight: u16,
    learner: i32,
) callconv(.C) i32 {
    const s = &store.?;
    const wal_op: Op = switch (op) {
        0 => .add_one,
        1 => .remove_one,
        2 => .double,
        3 => .halve,
        else => return -1,
    };
    s.op(wal_op, u128From(identity_lo, identity_hi), weight, learner != 0) catch return -1;
    return 0;
}

/// Force a checkpoint (the N-ops policy knob is host-side).
pub export fn uvrr_checkpoint() callconv(.C) i32 {
    const s = &store.?;
    s.checkpoint() catch return -1;
    return 0;
}

/// Inspect the store state.
pub export fn uvrr_sequence() callconv(.C) u64 {
    return (store orelse return 0).working.sequence;
}

pub export fn uvrr_incarnation() callconv(.C) u64 {
    return (store orelse return 0).working.uvrr_incarnation;
}

/// The working copyset's marker tag (§5.1): 0 `stopping`, 1 `stopped`,
/// 2 `restarting`, 3 `joining`.
pub export fn uvrr_marker() callconv(.C) i32 {
    const s = &store.?;
    return @as(i32, s.working.uvrr_marker);
}

pub export fn uvrr_member_count() callconv(.C) u32 {
    return (store orelse return 0).state.count;
}

/// Read member i: identity (lo/hi), weight, learner. Returns 0 on success.
pub export fn uvrr_member(
    index: u32,
    identity_lo: *u64,
    identity_hi: *u64,
    weight: *u16,
    learner: *i32,
) callconv(.C) i32 {
    const s = &store.?;
    if (index >= s.state.count) return -1;
    const m = s.state.members[index];
    const lo: u64 = @truncate(m.identity);
    const hi: u64 = @truncate(m.identity >> 64);
    identity_lo.* = lo;
    identity_hi.* = hi;
    weight.* = m.weight;
    learner.* = @intFromBool(m.learner);
    return 0;
}

pub export fn uvrr_close() callconv(.C) void {
    close();
}

fn close() void {
    if (store) |*s| {
        s.deinit();
    }
    store = null;
}

inline fn u128From(lo: u64, hi: u64) u128 {
    return (@as(u128, hi) << 64) | lo;
}
