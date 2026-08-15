# vrr-core: the Rust suite, and the Maelstrom node the core is checked with.
#
# Prerequisites for the Maelstrom targets: `mise install` (JDK 25 + Leiningen
# 2.11.2) and `gnuplot` on PATH for Jepsen's latency/rate plots. Without gnuplot
# the checkers still run but the overall verdict degrades to `:valid? :unknown`.
#
# For Docker-based end-to-end testing (no local JDK/Leiningen required):
#   make e2e           # build Docker image and run all Maelstrom tests

BIN        := $(CURDIR)/target/release/maelstrom-lin-kv
STATE_DIR  ?= $(CURDIR)/.state
LEIN       := mise exec -- lein

# K=5 gives f=2, so a nemesis that kills two nodes stays inside the protocol's
# fault bound. At K=3 (f=1) Jepsen's default kill targets include `:majority`
# and `:all`, which puts the cluster over budget and makes liveness impossible
# by construction — safety still holds, but no operation can succeed.
NODES      ?= 5
TIME_LIMIT ?= 180
RATE       ?= 20
INTERVAL   ?= 10
WORKLOAD   ?= lin-kv

.PHONY: help build check test clean-state test-clean test-partition test-kill test-all serve e2e docker-build docker-run tla tla-build tla-run tla-even tla-deep tla-mutations tla-local

help:
	@echo "make test            - the Rust test suite"
	@echo "make check           - fmt, clippy and the Rust test suite"
	@echo "make build           - release-build the cdylib and the Maelstrom node"
	@echo "make test-clean      - lin-kv, no faults (plumbing + baseline linearizability)"
	@echo "make test-partition  - lin-kv under network partitions"
	@echo "make test-kill       - lin-kv under process kill/restart (exercises recovery)"
	@echo "make test-all        - lin-kv under partition + kill + pause"
	@echo "make serve           - browse past results at http://localhost:8080"
	@echo "make e2e             - Docker: build image and run all Maelstrom tests"
	@echo "make tla             - Docker: model-check the TLA+ safety models"
	@echo "make tla-even        - Docker: reproduce the refused even-split transition"
	@echo "make tla-deep        - Docker: fixed-seed crash/overlap simulations"
	@echo "make tla-mutations   - Docker: require every defect regression to fail"
	@echo "make tla-local       - model-check with TLA2TOOLS_JAR and local Java"
	@echo "make docker-build    - build the Docker image"
	@echo "make docker-run      - run tests in the built Docker image"
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

# Docker targets - no local JDK/Leiningen required
# Works with Colima (no BuildKit, no volume mounts).
# The image architecture matches the Docker daemon's native architecture.
DOCKER_IMAGE ?= vrr-core-maelstrom
DOCKER_FILE   ?= Dockerfile.maelstrom

docker-build:
	docker build -f $(DOCKER_FILE) -t $(DOCKER_IMAGE) .

docker-run:
	docker run --rm \
		-e NODES=$(NODES) \
		-e TIME_LIMIT=$(TIME_LIMIT) \
		-e RATE=$(RATE) \
		-e INTERVAL=$(INTERVAL) \
		$(DOCKER_IMAGE) test-all

# End-to-end: build Docker image and run all Maelstrom tests
e2e: docker-build docker-run

# TLA+ command-line model checking. The image contains the models, so the run
# path works with Colima/classic Docker and deliberately uses no volume mount.
TLA_IMAGE     ?= vrr-core-tla
TLA_FILE      ?= Dockerfile.tla
TLA_PLATFORM  ?= linux/arm64
TLA_ARCH      ?= arm64
TLA_WORKERS   ?= 2
TLA2TOOLS_JAR ?=

tla-build:
	docker build --platform $(TLA_PLATFORM) --build-arg TLA_DEB_ARCH=$(TLA_ARCH) -f $(TLA_FILE) -t $(TLA_IMAGE) .
	test "$$(docker image inspect $(TLA_IMAGE) --format '{{.Architecture}}')" = "$(TLA_ARCH)"

tla-run:
	test "$$(docker image inspect $(TLA_IMAGE) --format '{{.Architecture}}')" = "$(TLA_ARCH)"
	docker run --rm --platform $(TLA_PLATFORM) $(TLA_IMAGE) -workers $(TLA_WORKERS) -config VrrCore.cfg VrrCore.tla
	docker run --rm --platform $(TLA_PLATFORM) $(TLA_IMAGE) -workers $(TLA_WORKERS) -config VrrCoreRecovery.cfg VrrCore.tla
	docker run --rm --platform $(TLA_PLATFORM) $(TLA_IMAGE) -workers $(TLA_WORKERS) -config VrrCoreEras.cfg VrrCoreEras.tla
	@expect_failure() { \
		cfg="$$1"; needle="$$2"; output="$$(mktemp -t vrr-tla.XXXXXX)"; \
		if docker run --rm --platform $(TLA_PLATFORM) $(TLA_IMAGE) -workers $(TLA_WORKERS) -config "$$cfg" VrrCoreEras.tla >"$$output" 2>&1; then \
			cat "$$output"; rm -f "$$output"; echo "expected $$cfg to fail"; exit 1; \
		fi; \
		if ! grep -F "$$needle" "$$output"; then cat "$$output"; rm -f "$$output"; exit 1; fi; \
		rm -f "$$output"; \
	}; \
	expect_failure VrrCoreErasWitnessNonStop.cfg "Invariant WNonStop is violated"; \
	expect_failure VrrCoreErasWitnessOverlap.cfg "Invariant WOverlapStreams is violated"

tla: tla-build tla-run

# The currently proposed even-split increment is not normative: the closed
# reverse cross-era gate rejects it before Init. Keep its exact witness runnable
# while the quorum-family ruling is resolved.
tla-even: tla-build
	@output="$$(mktemp -t vrr-tla-even.XXXXXX)"; \
	if docker run --rm --platform $(TLA_PLATFORM) $(TLA_IMAGE) -workers 1 -config VrrCoreErasEven.cfg VrrCoreEras.tla >"$$output" 2>&1; then \
		cat "$$output"; rm -f "$$output"; echo "expected even-split gate refusal"; exit 1; \
	fi; \
	if ! grep -F "Error: Assumption" "$$output"; then cat "$$output"; rm -f "$$output"; exit 1; fi; \
	rm -f "$$output"

tla-deep: tla-build
	docker run --rm --platform $(TLA_PLATFORM) $(TLA_IMAGE) -workers $(TLA_WORKERS) -simulate num=10000 -depth 40 -seed 1 -config VrrCoreErasCrash.cfg VrrCoreEras.tla
	docker run --rm --platform $(TLA_PLATFORM) $(TLA_IMAGE) -workers $(TLA_WORKERS) -simulate num=100000 -depth 60 -seed 1 -config VrrCoreErasDeep.cfg VrrCoreEras.tla

tla-mutations: tla-build
	@expect_failure() { \
		cfg="$$1"; needle="$$2"; shift 2; output="$$(mktemp -t vrr-tla-mutation.XXXXXX)"; \
		if docker run --rm --platform $(TLA_PLATFORM) $(TLA_IMAGE) -workers $(TLA_WORKERS) "$$@" -config "$$cfg" VrrCoreEras.tla >"$$output" 2>&1; then \
			cat "$$output"; rm -f "$$output"; echo "expected $$cfg to fail"; exit 1; \
		fi; \
		if ! grep -F "$$needle" "$$output"; then cat "$$output"; rm -f "$$output"; exit 1; fi; \
		rm -f "$$output"; \
	}; \
	expect_failure VrrCoreErasM1.cfg "Invariant CommittedEntrySurvives is violated"; \
	expect_failure VrrCoreErasM2.cfg "Invariant CommittedLogsAgree is violated"; \
	expect_failure VrrCoreErasM3.cfg "Invariant CommittedLogsAgree is violated" -simulate num=10000 -depth 60 -seed 1; \
	expect_failure VrrCoreErasM4.cfg "Invariant CommittedEntrySurvives is violated"; \
	expect_failure VrrCoreErasM5.cfg "Invariant CommittedLogsAgree is violated"; \
	expect_failure VrrCoreErasM6.cfg "Invariant CommittedLogsAgree is violated"; \
	expect_failure VrrCoreErasM7.cfg "Error: Assumption"

tla-local:
	test -n "$(TLA2TOOLS_JAR)"
	cd formal && java -XX:+UseParallelGC -jar "$(TLA2TOOLS_JAR)" -workers $(TLA_WORKERS) -config VrrCore.cfg VrrCore.tla
	cd formal && java -XX:+UseParallelGC -jar "$(TLA2TOOLS_JAR)" -workers $(TLA_WORKERS) -config VrrCoreRecovery.cfg VrrCore.tla
	cd formal && java -XX:+UseParallelGC -jar "$(TLA2TOOLS_JAR)" -workers $(TLA_WORKERS) -config VrrCoreEras.cfg VrrCoreEras.tla
	@cd formal && \
	expect_failure() { \
		cfg="$$1"; needle="$$2"; output="$$(mktemp -t vrr-tla-local.XXXXXX)"; \
		if java -XX:+UseParallelGC -jar "$(TLA2TOOLS_JAR)" -workers $(TLA_WORKERS) -config "$$cfg" VrrCoreEras.tla >"$$output" 2>&1; then \
			cat "$$output"; rm -f "$$output"; echo "expected $$cfg to fail"; exit 1; \
		fi; \
		if ! grep -F "$$needle" "$$output"; then cat "$$output"; rm -f "$$output"; exit 1; fi; \
		rm -f "$$output"; \
	}; \
	expect_failure VrrCoreErasWitnessNonStop.cfg "Invariant WNonStop is violated"; \
	expect_failure VrrCoreErasWitnessOverlap.cfg "Invariant WOverlapStreams is violated"
