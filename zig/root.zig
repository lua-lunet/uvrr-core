//! NEW : compilation root for the UVRR C-ABI static library. Lives at
//! zig/ so every `../` relative import in the vendored closure stays inside
//! the module path. `zig test` over this root collects the module tests
//! (the Zig convention: with the module); `zig build-lib` ignores them.
comptime {
    _ = @import("uvrr/capi.zig");
}

test {
    _ = @import("uvrr/store.zig");
}
