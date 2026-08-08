# vrr-core: the Rust suite, and the Maelstrom node the core is checked with.
#
# Prerequisites for the Maelstrom targets: `mise install` (JDK 21 + Leiningen
# 2.11.2) and `gnuplot` on PATH for Jepsen's latency/rate plots. Without gnuplot
# the checkers still run but the overall verdict degrades to `:valid? :unknown`.

BIN        := $(CURDIR)/target/release/maelstrom-lin-kv
STATE_DIR  ?= $(CURDIR)/.state
LEIN       := mise exec -- lein

# K=5 gives f=2, so a nemesis that kills two nodes stays inside the protocol's
# fault bound. At K=3 (f=1) Jepsen's default kill targets include `:majority`
# and `:all`, which puts the cluster over budget and makes liveness impossible
# by construction — safety still holds, but no operation can succeed.
NODES      ?= 5
TIME_LIMIT ?= 60
RATE       ?= 20
INTERVAL   ?= 20
WORKLOAD   ?= lin-kv

.PHONY: help build check test clean-state test-clean test-partition test-kill test-all serve

help:
	@echo "make test            - the Rust test suite"
	@echo "make check           - fmt, clippy and the Rust test suite"
	@echo "make build           - release-build the cdylib and the Maelstrom node"
	@echo "make test-clean      - lin-kv, no faults (plumbing + baseline linearizability)"
	@echo "make test-partition  - lin-kv under network partitions"
	@echo "make test-kill       - lin-kv under process kill/restart (exercises recovery)"
	@echo "make test-all        - lin-kv under partition + kill + pause"
	@echo "make serve           - browse past results at http://localhost:8080"
	@echo
	@echo "Vars: NODES=$(NODES) TIME_LIMIT=$(TIME_LIMIT) RATE=$(RATE) INTERVAL=$(INTERVAL)"

build:
	cargo build --release --all-targets

test:
	cargo test

check:
	cargo fmt -- --check
	cargo clippy --all-targets -- -D warnings
	cargo test

# Each run starts from no durable state, so every node's first boot is a first
# boot. A stale nonce marker would make a node believe it had restarted and
# enter recovery at cluster start, where nobody is Normal to answer it.
clean-state:
	rm -rf $(STATE_DIR)
	mkdir -p $(STATE_DIR)

define run_maelstrom
	cd maelstrom && MAELSTROM_VRR_STATE_DIR=$(STATE_DIR) $(LEIN) run test \
		-w $(WORKLOAD) --bin $(BIN) \
		--node-count $(NODES) --time-limit $(TIME_LIMIT) \
		--rate $(RATE) --concurrency 2n $(1)
endef

test-clean: build clean-state
	$(call run_maelstrom,)

test-partition: build clean-state
	$(call run_maelstrom,--nemesis partition --nemesis-interval $(INTERVAL))

test-kill: build clean-state
	$(call run_maelstrom,--nemesis kill --nemesis-interval $(INTERVAL))

test-all: build clean-state
	$(call run_maelstrom,--nemesis partition,kill,pause --nemesis-interval $(INTERVAL))

serve:
	cd maelstrom && $(LEIN) run serve
