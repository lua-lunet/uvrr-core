//! NEW : minimal `vsr` shim for the vendored superblock/quorums closure.
//! This is NOT TB's src/vsr.zig (1786 lines, consensus engine). It carries only
//! the symbols superblock.zig / superblock_quorums.zig / checksum.zig need:
//! checksum (vendored verbatim), Header (TB's extern layout, no body/release
//! plumbing), Release, Members, root_members, member_index, fatal, and a
//! Zone enum sized for our three-zone data file. See PATCH_MANIFEST.md.

const std = @import("std");
const assert = std.debug.assert;
const constants = @import("constants.zig");
const stdx = @import("stdx");

pub const checksum = @import("vsr/checksum.zig").checksum;
pub const ChecksumStream = @import("vsr/checksum.zig").ChecksumStream;

/// TB multiversion.zig Release, reduced (value: u32 extern).
pub const Release = extern struct {
    value: u32,

    comptime {
        assert(@sizeOf(Release) == 4);
        assert(stdx.no_padding(Release));
    }

    pub const zero = Release.from(.{ .major = 0, .minor = 0, .patch = 0 });
    pub const minimum = Release.from(.{ .major = 0, .minor = 0, .patch = 1 });

    pub fn from(triple: ReleaseTriple) Release {
        return .{ .value = encode(triple.major, triple.minor, triple.patch) };
    }

    pub fn encode(major: u16, minor: u16, patch: u16) u32 {
        return (@as(u32, major) << 20) | (@as(u32, minor & 0xFF) << 12) | (patch & 0xFFF);
    }
};

pub const ReleaseTriple = struct {
    major: u16,
    minor: u16,
    patch: u16,
};

/// TB vsr.zig Members: replica_ids in ring order (members_max slots).
pub const Members = [constants.members_max]u128;

/// TB vsr.zig root_members(): deterministic replica_id derivation from the
/// cluster id (fnv1a-style fold, same shape; the PRNG-dependent full version
/// is not needed for the membership demo).
pub fn root_members(cluster: u128) Members {
    var members = std.mem.zeroes(Members);
    for (&members, 0..) |*member, i| {
        var hash = cluster ^ @as(u128, i);
        hash ^= hash >> 33;
        hash *%= 0xff51afd7ed558ccd;
        hash ^= hash >> 33;
        hash *%= 0xc4ceb9fe1a85ec53;
        hash ^= hash >> 33;
        member.* = hash | 1;
    }
    return members;
}

pub fn member_index(members: *const Members, replica_id: u128) ?u8 {
    for (members, 0..) |member, i| {
        if (member == replica_id and member != 0) {
            return @intCast(i);
        }
    }
    return null;
}

pub const FatalReason = enum { unsupported, corrupted_superblock, io };
pub fn fatal(reason: FatalReason, comptime fmt: []const u8, args: anytype) noreturn {
    std.debug.panic("vsr fatal ({s}): " ++ fmt, .{ @tagName(reason) } ++ args);
}

/// TB message_header.zig Header extern layout — frame only, verbatim field
/// order; the body/message-type plumbing (Command switch, HeaderFunctionsType)
/// is out of scope for the vendored closure.
pub const Command = enum(u8) { reserved, prepare, prepare_ok, reply, commit };
pub const Operation = enum(u8) { reserved, root, uvrr_membership };

pub const Header = extern struct {
    checksum: u128,
    checksum_padding: u128,
    checksum_body: u128,
    checksum_body_padding: u128,
    nonce_reserved: u128,
    cluster: u128,
    size: u32,
    epoch: u32,
    view: u32,
    release: Release,
    protocol: u16,
    command: Command,
    replica: u8,
    reserved_frame: [12]u8,
    reserved_command: [128]u8,

    comptime {
        assert(@sizeOf(Header) == 256);
        assert(@alignOf(Header) == 16);
        assert(stdx.no_padding(Header));
    }

    pub const Prepare = extern struct {
        checksum: u128 = 0,
        checksum_padding: u128 = 0,
        checksum_body: u128 = 0,
        checksum_body_padding: u128 = 0,
        nonce_reserved: u128 = 0,
        cluster: u128,
        size: u32 = @sizeOf(Header),
        epoch: u32 = 0,
        view: u32,
        release: Release,
        protocol: u16 = 1,
        command: Command,
        replica: u8 = 0,
        reserved_frame: [12]u8 = @splat(0),
        parent: u128,
        parent_padding: u128 = 0,
        request_checksum: u128,
        request_checksum_padding: u128 = 0,
        checkpoint_id: u128,
        client: u128,
        op: u64,
        commit: u64,
        timestamp: u64,
        request: u32,
        operation: Operation,
        reserved: [3]u8 = @splat(0),

        comptime {
            assert(@sizeOf(Prepare) == 256);
            assert(@alignOf(Prepare) == 16);
            assert(stdx.no_padding(Prepare));
        }

        pub fn calculate_checksum(self: *const Prepare) u128 {
            return checksum(std.mem.asBytes(self)[@sizeOf(u128)..]);
        }

        pub fn set_checksum(self: *Prepare) void {
            self.checksum = self.calculate_checksum();
        }

        pub fn valid_checksum(self: *const Prepare) bool {
            return self.checksum == self.calculate_checksum();
        }
    };
};

/// PATCH: Zone for our three-zone data file (TB's Zone has six zones sized for
/// the LSM grid + WAL ring buffers + client replies; those zones do not exist
/// in the tiny data file).
pub const Zone = enum {
    superblock,
    membership_wal,
    checkpoint,

    pub const superblock_zone_size = @import("vsr/superblock.zig").superblock_zone_size;
    pub const membership_wal_size = 8 * constants.sector_size;
    pub const checkpoint_size = constants.block_size;

    pub fn start(zone: Zone) u64 {
        return switch (zone) {
            .superblock => 0,
            .membership_wal => superblock_zone_size,
            .checkpoint => superblock_zone_size + membership_wal_size,
        };
    }

    pub fn size(zone: Zone) u64 {
        return switch (zone) {
            .superblock => superblock_zone_size,
            .membership_wal => membership_wal_size,
            .checkpoint => checkpoint_size,
        };
    }
};
