#!/bin/bash
# Docker entrypoint for vrr-core Maelstrom tests
# Supports: test-all, test-clean, test-partition, test-kill, serve, build, check

set -euo pipefail

BIN="/usr/local/bin/maelstrom-lin-kv"
STATE_DIR="/tmp/maelstrom-state"
WORKLOAD="lin-kv"

# Default values (can be overridden by env vars)
NODES=${NODES:-5}
TIME_LIMIT=${TIME_LIMIT:-180}
RATE=${RATE:-20}
INTERVAL=${INTERVAL:-10}

# Clean state
clean_state() {
    rm -rf "$STATE_DIR"
    mkdir -p "$STATE_DIR"
}

# Build the binary (if not already built)
build_binary() {
    if [ ! -f "$BIN" ]; then
        echo "Building maelstrom-lin-kv..."
        cd /usr/src/vrr-core
        cargo build --release --bin maelstrom-lin-kv
    fi
}

# Run maelstrom test
run_maelstrom() {
    local nemesis_args="$1"
    cd /usr/src/vrr-core/maelstrom
    MAELSTROM_VRR_STATE_DIR="$STATE_DIR" lein run test \
        -w "$WORKLOAD" --bin "$BIN" \
        --node-count "$NODES" --time-limit "$TIME_LIMIT" \
        --rate "$RATE" --concurrency 2n \
        $nemesis_args
}

case "$1" in
    test-clean)
        clean_state
        build_binary
        run_maelstrom ""
        ;;
    test-partition)
        clean_state
        build_binary
        run_maelstrom "--nemesis partition --nemesis-interval $INTERVAL"
        ;;
    test-kill)
        clean_state
        build_binary
        run_maelstrom "--nemesis kill --nemesis-interval $INTERVAL"
        ;;
    test-all)
        clean_state
        build_binary
        run_maelstrom "--nemesis partition,kill,pause --nemesis-interval $INTERVAL"
        ;;
    serve)
        cd /usr/src/vrr-core/maelstrom
        lein run serve
        ;;
    build)
        build_binary
        ;;
    check)
        cd /usr/src/vrr-core
        cargo fmt -- --check
        cargo clippy --all-targets -- -D warnings
        cargo test
        ;;
    test)
        cd /usr/src/vrr-core
        cargo test
        ;;
    help|--help|-h)
        echo "Usage: docker-entrypoint.sh [command]"
        echo ""
        echo "Commands:"
        echo "  test-all       - lin-kv under partition + kill + pause (default)"
        echo "  test-clean     - lin-kv, no faults (baseline linearizability)"
        echo "  test-partition - lin-kv under network partitions"
        echo "  test-kill      - lin-kv under process kill/restart"
        echo "  serve          - browse past results at http://localhost:8080"
        echo "  build          - release-build the cdylib and Maelstrom node"
        echo "  check          - fmt, clippy and the Rust test suite"
        echo "  test           - the Rust test suite"
        echo ""
        echo "Environment variables:"
        echo "  NODES          - number of nodes (default: 5)"
        echo "  TIME_LIMIT     - time limit in seconds (default: 180)"
        echo "  RATE           - request rate (default: 20)"
        echo "  INTERVAL       - nemesis interval (default: 10)"
        ;;
    *)
        # Default: run test-all
        exec "$0" test-all
        ;;
esac
