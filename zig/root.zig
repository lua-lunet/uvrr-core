//! NEW (item31): compilation root for the UVRR C-ABI static library. Lives at
//! zig/ so every `../` relative import in the vendored closure stays inside
//! the module path.
comptime {
    _ = @import("uvrr/capi.zig");
}
