//! PATCHED (item31): reduced constants for the tiny UVRR data file.
//! Derived from TB's src/constants.zig (0.17.9) — values not needed by the
//! vendored IO/superblock closure are removed; the removed zones (TB WAL ring
//! buffers, 512KiB LSM grid, client replies, message sizing) do not exist here.
//! Every retained value keeps TB's default. See zig/PATCH_MANIFEST.md.

const std = @import("std");
const assert = std.debug.assert;

/// TB: sector_size = 4096 (constants.zig:489). Unchanged.
pub const sector_size = 4096;

/// TB default block_size = 512 KiB (config.zig:161). Our checkpoint grid zone
/// uses a single 4 KiB block instead; block_size is patched accordingly.
pub const block_size = 4096;

/// TB: superblock_copies must be 4, 6, or 8 (superblock.zig compile switch).
pub const superblock_copies = 4;

/// PATCH: tiny view-header suffix (TB default is 7 via
/// view_change_headers_suffix_max=5 + 2). We carry no view-change traffic.
pub const view_headers_max = 2;

/// PATCH: membership size cap for the tiny data file (TB: replicas_max=6 +
/// standbys_max=6; same total, kept for parity with vsr.Members sizing).
pub const replicas_max = 6;
pub const standbys_max = 6;
pub const members_max = replicas_max + standbys_max;

/// PATCH: checkpoint-policy knob for the membership WAL (host concern; set
/// per-host before opening; 0 means "only checkpoint before WAL wrap").
pub const membership_checkpoint_interval_default = 4;

/// TB: config.process verify flag (queue integrity assertions). Default true.
pub const verify = true;

comptime {
    assert(superblock_copies == 4 or superblock_copies == 6 or superblock_copies == 8);
    assert(block_size % sector_size == 0);
}
