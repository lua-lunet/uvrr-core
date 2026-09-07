//! PATCHED  from TB 0.17.9 src/vsr/superblock.zig — see zig/PATCH_MANIFEST.md.
//!
//! Retained verbatim (TB line refs in comments):
//!   * the SuperBlockHeader checksum scheme (calculate/set/valid_checksum)
//!   * the copy layout (`superblock_copy_size * copy_index`)
//!   * the 3-of-4 write / 2-of-4 read quorums (via vendored superblock_quorums.zig)
//!   * the hash-chained `parent` across sequence numbers
//!
//! Patches (all listed in PATCH_MANIFEST.md):
//!   * sizes/shapes for the tiny UVRR data file: no TB WAL ring buffers, no
//!     512KiB LSM grid, no client-replies zone; VSRState slimmed to the
//!     membership fields the reincarnation protocol needs.
//!   * `uvrr_incarnation` (u64) and `uvrr_flushed` (u8): the reincarnation
//!     `flushed`/`unflushed` state as declared fields, covered by the checksum
//!     (checksum discipline holds; TB's reserved `flags` stays 0 and is still
//!     asserted zero in set_checksum).
//!   * identity/sequence bumps per the reincarnation protocol
//!     (`open_highest_identity`: higher identity wins among equal sequences).
//!   * the async SuperBlockType(Storage) state machine is replaced by the
//!     synchronous store in zig/uvrr/store.zig (callbacks remain in quorums).

const std = @import("std");
const assert = std.debug.assert;
const mem = std.mem;
const meta = std.meta;

const constants = @import("../constants.zig");
const stdx = @import("stdx");
const vsr = @import("../vsr.zig");
const log = std.log.scoped(.superblock);

pub const Quorums = @import("superblock_quorums.zig").QuorumsType(.{
    .superblock_copies = constants.superblock_copies,
});

/// PATCH: TB derives this from constants.config.process.release; our data file
/// format is fixed at 2 (TB's current SuperBlockVersion) for release builds.
pub const SuperBlockVersion: u16 = 2;

/// PATCH: TB computes the reserved suffix so that the view headers end on a
/// sector boundary. Kept verbatim in form, with our tiny view_headers_max.
const view_headers_reserved_size = constants.sector_size -
    ((constants.view_headers_max * @sizeOf(vsr.Header.Prepare)) % constants.sector_size);

const reserved_size = constants.sector_size -
    (@offsetOf(SuperBlockHeaderFixedPrefix, "view_headers_count") + @sizeOf(u32));

/// The bytes of SuperBlockHeader that precede `view_headers_count`. Declared as
/// a struct only so `reserved_size` can be computed at comptime; the actual
/// layout lives in SuperBlockHeader (the field order is identical).
const SuperBlockHeaderFixedPrefix = extern struct {
    checksum: u128 = undefined,
    checksum_padding: u128 = 0,
    copy: u16 = 0,
    version: u16,
    release_format: vsr.Release,
    sequence: u64,
    cluster: u128,
    parent: u128,
    parent_padding: u128 = 0,
    vsr_state: VSRState,
    flags: u64 = 0,
    uvrr_incarnation: u64 = 0,
    uvrr_flushed: u8 = 0,
    uvrr_flushed_padding: [3]u8 = @splat(0),
    view_headers_count: u32,
};

pub const VSRState = extern struct {
    /// Globally unique identifier of the replica, must be non-zero.
    replica_id: u128,

    /// Set of replica_ids of cluster members, where order of ids determines
    /// replica indexes (TB vsr.Members).
    members: vsr.Members,

    /// The highest operation up to which we may commit.
    commit_max: u64,

    /// The last view in which the replica's status was normal.
    log_view: u32,

    /// The current view number of the replica.
    view: u32,

    /// Number of replicas (determines sizes of the quorums).
    replica_count: u8,

    reserved: [15]u8 = @splat(0),

    comptime {
        assert(@sizeOf(VSRState) == 240);
        assert(stdx.no_padding(VSRState));
    }

    /// PATCH: monotonicity over the slimmed VSRState (TB's version checks the
    /// full CheckpointState; ours keeps the same view/commit_max/log_view rules).
    pub fn monotonic(old: VSRState, new: VSRState) bool {
        assert(old.replica_id == new.replica_id);
        assert(old.replica_count == new.replica_count);
        assert(std.mem.eql(u8, std.mem.asBytes(&old.members), std.mem.asBytes(&new.members)));

        if (old.view > new.view) return false;
        if (old.log_view > new.log_view) return false;
        if (old.commit_max > new.commit_max) return false;
        return true;
    }

    pub fn root(options: struct {
        cluster: u128,
        replica_id: u128,
        members: vsr.Members,
        replica_count: u8,
        view: u32,
    }) VSRState {
        _ = options.cluster;
        return .{
            .replica_id = options.replica_id,
            .members = options.members,
            .commit_max = 0,
            .log_view = options.view,
            .view = options.view,
            .replica_count = options.replica_count,
        };
    }
};

/// Fields are aligned to work as an extern or packed struct.
pub const SuperBlockHeader = extern struct {
    checksum: u128 = undefined,
    checksum_padding: u128 = 0,

    /// Protects against misdirected reads at startup.
    /// Excluded from the checksum calculation to ensure that all copies have
    /// the same checksum. This simplifies writing and comparing multiple copies.
    copy: u16 = 0,

    /// The version of the superblock format in use.
    version: u16,

    /// The release that the data file was originally formatted by.
    release_format: vsr.Release,

    /// A monotonically increasing counter to locate the latest superblock.
    sequence: u64,

    /// Protects against writing to or reading from the wrong data file.
    cluster: u128,

    /// The checksum of the previous superblock to hash chain across sequence
    /// numbers.
    parent: u128,
    parent_padding: u128 = 0,

    vsr_state: VSRState,

    /// Reserved for future minor features (kept zero; see set_checksum).
    flags: u64 = 0,

    /// PATCH: the reincarnation identity. Bumped on a dirty restart; among
    /// copies with the same sequence, the highest identity wins on read.
    uvrr_incarnation: u64 = 0,

    /// PATCH: the reincarnation `flushed` mark. Set (1) only by the clean
    /// shutdown path, after the flush has reached every copy.
    uvrr_flushed: u8 = 0,
    /// Explicit alignment padding (declared, to keep no_padding honest).
    uvrr_flushed_padding: [3]u8 = @splat(0),

    /// The number of headers in view_headers_all.
    view_headers_count: u32 = 0,

    reserved: [reserved_size]u8 = @splat(0),

    /// View/JV header suffix (zeroed; we carry no view-change traffic).
    view_headers_all: [constants.view_headers_max]vsr.Header.Prepare,
    view_headers_reserved: [view_headers_reserved_size]u8 = @splat(0),

    comptime {
        assert(@sizeOf(SuperBlockHeader) % constants.sector_size == 0);
        assert(@divExact(@sizeOf(SuperBlockHeader), constants.sector_size) >= 2);
        assert(@offsetOf(SuperBlockHeader, "parent") % @sizeOf(u256) == 0);
        assert(@offsetOf(SuperBlockHeader, "vsr_state") % @sizeOf(u256) == 0);
        assert(@offsetOf(SuperBlockHeader, "view_headers_all") == constants.sector_size);
        assert(stdx.no_padding(SuperBlockHeader));
    }

    pub fn calculate_checksum(superblock: *const SuperBlockHeader) u128 {
        comptime assert(meta.fieldIndex(SuperBlockHeader, "checksum") == 0);
        comptime assert(meta.fieldIndex(SuperBlockHeader, "checksum_padding") == 1);
        comptime assert(meta.fieldIndex(SuperBlockHeader, "copy") == 2);

        const checksum_size = @sizeOf(@TypeOf(superblock.checksum));
        comptime assert(checksum_size == @sizeOf(u128));

        const checksum_padding_size = @sizeOf(@TypeOf(superblock.checksum_padding));
        comptime assert(checksum_padding_size == @sizeOf(u128));

        const copy_size = @sizeOf(@TypeOf(superblock.copy));
        comptime assert(copy_size == 2);

        const ignore_size = checksum_size + checksum_padding_size + copy_size;

        // PATCH: `uvrr_flushed` is excluded from the checksum by masking (all
        // copies of one sequence share one checksum; the flushed mark is a
        // per-copy durability bit, like `copy`). Without the mask, a copy whose
        // flushed write was lost would form a second checksum at the same
        // sequence, and TB's quorum logic reports that as a fork.
        var masked: SuperBlockHeader = superblock.*;
        masked.uvrr_flushed = 0;
        return vsr.checksum(std.mem.asBytes(&masked)[ignore_size..]);
    }

    pub fn set_checksum(superblock: *SuperBlockHeader) void {
        // `copy` is not covered by the checksum.
        // PATCH: TB asserts copy == 0 here because its staging header is
        // always zero and copies are stamped when written; our sync store
        // computes the checksum once per copyset with a per-copy `copy` value
        // set afterwards, so only the range is asserted.
        assert(superblock.copy < constants.superblock_copies);

        assert(superblock.version == SuperBlockVersion);
        assert(superblock.release_format.value > 0);
        assert(superblock.flags == 0);

        assert(stdx.zeroed(&superblock.reserved));
        assert(stdx.zeroed(&superblock.vsr_state.reserved));
        assert(stdx.zeroed(&superblock.view_headers_reserved));
        assert(superblock.view_headers_count == 0);

        assert(superblock.checksum_padding == 0);
        assert(superblock.parent_padding == 0);

        superblock.checksum = superblock.calculate_checksum();
    }

    pub fn valid_checksum(superblock: *const SuperBlockHeader) bool {
        return superblock.checksum == superblock.calculate_checksum() and
            superblock.checksum_padding == 0;
    }

    /// PATCH: byte-equality modulo the checksum-protected `copy` field.
    pub fn equal(a: *const SuperBlockHeader, b: *const SuperBlockHeader) bool {
        var x = a.*;
        var y = b.*;
        x.copy = 0;
        y.copy = 0;
        // PATCH: uvrr_flushed is per-copy durability state (not covered by the
        // checksum), so copies of one sequence may differ in it.
        x.uvrr_flushed = 0;
        y.uvrr_flushed = 0;
        return std.mem.eql(u8, std.mem.asBytes(&x), std.mem.asBytes(&y));
    }
};

comptime {
    switch (constants.superblock_copies) {
        4, 6, 8 => {},
        else => @compileError("superblock_copies must be either { 4, 6, 8 } for flexible quorums."),
    }
}

/// The size of the entire superblock storage zone.
/// PATCH: no copy padding (TB pads for potential extra view headers; we carry
/// no view-change traffic).
pub const superblock_zone_size = superblock_copy_size * constants.superblock_copies;

pub const superblock_copy_size = @sizeOf(SuperBlockHeader);
comptime {
    assert(superblock_copy_size % constants.sector_size == 0);
}

/// PATCH: the tiny UVRR data file: superblock zone + membership WAL zone +
/// one 4KiB checkpoint block (see vsr.Zone for offsets).
pub const data_file_size_min =
    vsr.Zone.superblock_zone_size +
    vsr.Zone.membership_wal_size +
    vsr.Zone.checkpoint_size;

comptime {
    assert(superblock_zone_size % constants.sector_size == 0);
    assert(data_file_size_min % constants.sector_size == 0);
}

/// The sequence number progression of the SuperBlock's headers (TB table,
/// unchanged): format writes a copyset for the first sequence, open verifies
/// the read quorum (2/4), writes repair to 3/4 (verify quorum).
///
/// PATCH (reincarnation protocol):
///   * clean shutdown: bump nothing; set uvrr_flushed=1 on all four copies.
///   * dirty restart: read the working quorum; bump sequence by 1 and
///     uvrr_incarnation by 1; clear uvrr_flushed; write all four copies.
///   * open: among copies with the highest valid sequence, the highest
///     uvrr_incarnation wins (higher-identity-wins read rule).
pub fn copy_offset(copy_index: u32) u64 {
    assert(copy_index < constants.superblock_copies);
    return superblock_copy_size * copy_index;
}
