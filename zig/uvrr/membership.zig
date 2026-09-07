//! NEW (item31) — zig/uvrr/membership.zig
//!
//! Cluster-membership store for tiny data, in TB idiom (modeled on
//! src/aof.zig: append-only hash-chained log, fixed-size extern entries,
//! magic number, checksum discipline):
//!
//!   * Ops WAL: append-only, sector-sized extern entries, magic + checksum
//!     hash-chain (`parent` = previous entry checksum), written synchronously
//!     through the vendored direct-IO layer. Ops: add_one / remove_one /
//!     double / halve (the voting-weights reconfiguration moves).
//!   * Checkpoint: the FULL cluster membership state serialized into ONE
//!     4 KiB block (fixed bytes: magic, version, count, entries
//!     {identity u128, weight u16, learner u8}, checksum). Compile-time
//!     size assert. Scale: ~6 active members, 7 during a node swap, dozens
//!     of learners — fits 4 KiB with members_max=12.
//!   * Checkpoint policy: flush the full state to the checkpoint block BEFORE
//!     the WAL wraps (the WAL restarts at slot 0 after a checkpoint, so live
//!     entries are never overwritten) or every N ops — N is a host concern
//!     (`checkpoint_interval`, 0 = only-before-wrap).
//!   * Replay: open = read the checkpoint block (checksum-verified) + replay
//!     the WAL suffix that continues the checkpoint's hash chain.

const std = @import("std");
const assert = std.debug.assert;
const mem = std.mem;

const constants = @import("../constants.zig");
const vsr = @import("../vsr.zig");
const stdx = @import("stdx");
const vsr_checksum = vsr.checksum;

pub const magic_number: u128 = 0x5f75_7672_726d_656d_6265_7273_6869_7001; // "_uvrrmembership\x01"
pub const checkpoint_magic: u128 = 0x5f75_7672_7263_6b70_7431_5f00_0000_00; // "_uvrrckpt1_"

pub const members_capacity = constants.members_max;
pub const wal_slot_size = constants.sector_size; // one direct-IO sector per slot
pub const wal_slot_count = 8; // 32 KiB WAL zone (vsr.Zone.membership_wal_size = 8 sectors)
pub const checkpoint_block_size = constants.block_size;
comptime {
    assert(wal_slot_size * wal_slot_count == vsr.Zone.membership_wal_size);
    assert(checkpoint_block_size == vsr.Zone.checkpoint_size);
}

/// The voting-weights reconfiguration moves. Exact op enum; nothing else.
pub const Op = enum(u8) {
    add_one,
    remove_one,
    double,
    halve,
};

pub const DecodedCheckpoint = struct {
    state: State,
    wal_last_checksum: u128,
    wal_last_sequence: u64,
};

/// One member of the cluster configuration.
pub const Member = struct {
    identity: u128 = 0,
    weight: u16 = 1,
    learner: bool = false,
};

/// The full membership state, replayed from checkpoint + WAL suffix.
pub const State = struct {
    members: [members_capacity]Member = @splat(.{}),
    count: u32 = 0,

    pub fn find(state: *State, identity: u128) ?*Member {
        for (state.members[0..state.count]) |*m| {
            if (m.identity == identity) return m;
        }
        return null;
    }

    pub fn upsert(state: *State, member: Member) !*Member {
        if (state.find(member.identity)) |m| {
            m.weight = member.weight;
            m.learner = member.learner;
            return m;
        }
        if (state.count == members_capacity) return error.MembersFull;
        state.members[state.count] = member;
        state.count += 1;
        return &state.members[state.count - 1];
    }

    pub fn remove(state: *State, identity: u128) !void {
        const index: usize = for (state.members[0..state.count], 0..) |m, i| {
            if (m.identity == identity) break i;
        } else return error.UnknownMember;
        state.count -= 1;
        if (index != state.count) {
            state.members[index] = state.members[state.count];
        }
        state.members[state.count] = .{};
    }

    pub fn apply(state: *State, op: Op, identity: u128, weight: u16, learner: bool) !void {
        switch (op) {
            .add_one => _ = try state.upsert(.{
                .identity = identity,
                .weight = @max(weight, 1),
                .learner = learner,
            }),
            .remove_one => try state.remove(identity),
            .double => {
                const m = state.find(identity) orelse return error.UnknownMember;
                m.weight = m.weight * 2;
                if (m.weight > std.math.maxInt(u16) / 2) return error.WeightOverflow;
            },
            .halve => {
                const m = state.find(identity) orelse return error.UnknownMember;
                m.weight = m.weight / 2;
                if (m.weight == 0) m.weight = 1;
            },
        }
    }
};

/// One WAL slot: fixed-size extern struct, sector-sized, magic + hash-chain.
pub const WALSlot = extern struct {
    /// Fixed random integer, to allow skipping corrupted entries (aof.zig).
    magic: u128 = magic_number,
    /// Checksum of the entry from `parent` onward (excludes magic/checksum).
    checksum: u128 = 0,
    /// Checksum of the previous entry, or the checkpoint's chain head.
    parent: u128 = 0,
    /// WAL sequence number, monotonically increasing across restarts.
    member_identity: u128 = 0,
    sequence: u64 = 0,
    weight: u16 = 1,
    learner: u8 = 0,
    op: Op,
    reserved: [4020]u8 = @splat(0),

    comptime {
        assert(@sizeOf(WALSlot) == wal_slot_size);
        assert(@alignOf(WALSlot) == 16);
        assert(stdx.no_padding(WALSlot));
    }

    pub fn calculate_checksum(slot: *const WALSlot) u128 {
        const offset = @offsetOf(WALSlot, "parent");
        return vsr_checksum(std.mem.asBytes(slot)[offset..]);
    }

    pub fn set_checksum(slot: *WALSlot) void {
        slot.checksum = slot.calculate_checksum();
    }

    pub fn valid(slot: *const WALSlot) bool {
        return slot.magic == magic_number and
            slot.checksum == slot.calculate_checksum();
    }
};

/// The checkpoint block: FULL membership state, one 4 KiB block of fixed bytes.
pub const Checkpoint = extern struct {
    magic: u128 = checkpoint_magic,
    version: u32 = 1,
    count: u32 = 0,
    padding: [8]u8 = @splat(0),
    /// WAL hash-chain head at checkpoint time; the next WAL entry's `parent`.
    wal_last_checksum: u128 = 0,
    wal_last_sequence: u64 = 0,
    padding2: [8]u8 = @splat(0),
    entries: [members_capacity]CheckpointEntry = @splat(.{}),
    reserved: [3632]u8 = @splat(0),
    checksum: u128 = 0,

    comptime {
        assert(@sizeOf(Checkpoint) == checkpoint_block_size);
        assert(@sizeOf(CheckpointEntry) == 32);
        assert(@alignOf(Checkpoint) == 16);
        assert(stdx.no_padding(Checkpoint));
    }

    pub const CheckpointEntry = extern struct {
        identity: u128 = 0,
        weight: u16 = 0,
        learner: u8 = 0,
        reserved: [13]u8 = @splat(0),

        comptime {
            assert(stdx.no_padding(CheckpointEntry));
        }
    };

    pub fn encode(state: *const State, wal_last_checksum: u128, wal_last_sequence: u64) Checkpoint {
        var block = Checkpoint{
            .count = state.count,
            .wal_last_checksum = wal_last_checksum,
            .wal_last_sequence = wal_last_sequence,
        };
        for (state.members[0..state.count], 0..) |m, i| {
            block.entries[i] = .{
                .identity = m.identity,
                .weight = m.weight,
                .learner = @intFromBool(m.learner),
            };
        }
        block.checksum = vsr_checksum(std.mem.asBytes(&block)[0 .. @sizeOf(Checkpoint) - @sizeOf(u128)]);
        return block;
    }

    pub fn decode(block: *const Checkpoint) !DecodedCheckpoint {
        if (block.magic != checkpoint_magic) return error.NotCheckpoint;
        if (block.version != 1) return error.UnsupportedVersion;
        if (block.count > members_capacity) return error.CountOutOfRange;
        if (block.checksum !=
            vsr_checksum(std.mem.asBytes(block)[0 .. @sizeOf(Checkpoint) - @sizeOf(u128)]))
        {
            return error.InvalidChecksum;
        }
        var state = State{ .count = 0 };
        for (block.entries[0..block.count]) |entry| {
            try state.apply(.add_one, entry.identity, entry.weight, entry.learner != 0);
        }
        return .{
            .state = state,
            .wal_last_checksum = block.wal_last_checksum,
            .wal_last_sequence = block.wal_last_sequence,
        };
    }
};

/// The replayed store: checkpoint state + WAL suffix.
pub const Replayed = struct {
    state: State,
    wal_last_checksum: u128,
    wal_last_sequence: u64,
    wal_count: u64,
};

/// Replay the WAL suffix onto a decoded checkpoint state.
/// `slots` must hold the WAL zone bytes in slot order.
pub fn replay(
    state_in: State,
    wal_last_checksum_in: u128,
    wal_last_sequence_in: u64,
    slots: []const WALSlot,
) !Replayed {
    var state = state_in;
    var last_checksum = wal_last_checksum_in;
    var last_sequence = wal_last_sequence_in;
    var count: u64 = 0;

    for (slots) |*slot| {
        if (slot.magic != magic_number) continue; // unused slot
        if (!slot.valid()) continue; // torn/corrupt entry: replay stops
        if (last_checksum != 0 and slot.parent != last_checksum) continue; // pre-checkpoint stale
        if (slot.sequence <= last_sequence) continue;

        try state.apply(slot.op, slot.member_identity, slot.weight, slot.learner != 0);
        last_checksum = slot.checksum;
        last_sequence = slot.sequence;
        count += 1;
    }

    return .{
        .state = state,
        .wal_last_checksum = last_checksum,
        .wal_last_sequence = last_sequence,
        .wal_count = count,
    };
}

comptime {
    // The checkpoint must hold dozens of learners + 7 active members.
    assert(members_capacity * @sizeOf(Checkpoint.CheckpointEntry) + 48 + @sizeOf(u128) <= checkpoint_block_size);
}
