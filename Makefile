SHELL := /bin/bash

SOCKET ?= /tmp/chatminald.sock
DAEMON_ENDPOINT := CHATMINAL_DAEMON_ENDPOINT=$(SOCKET)

APP_MANIFEST := apps/chatminal-app/Cargo.toml
DAEMON_MANIFEST := apps/chatminald/Cargo.toml

WIDTH ?= 120
HEIGHT ?= 32
SIDEBAR_WIDTH ?= 20
PREVIEW_LINES ?= 120
PREVIEW_CHARS ?= 200

.PHONY: help clean-socket daemon daemon-reset dashboard window attach workspace sessions create activate bootstrap-terminal-deps verify-third-party-reference-only check check-desktop test smoke-window bench-rtt bench-phase02 fidelity-smoke fidelity-matrix-smoke fidelity-matrix-smoke-relaxed fidelity-input-ime-smoke phase06-killswitch-verify phase08-killswitch-verify soak-smoke release-dry-run

help:
	@echo "Chatminal shortcuts:"
	@echo "  make daemon                                 # Run daemon"
	@echo "  make daemon-reset                           # Kill old daemon, clean socket, run daemon"
	@echo "  make dashboard                              # Run TUI dashboard"
	@echo "  make window                                 # Run Chatminal Desktop window with session sidebar (default runtime)"
	@echo "  make attach [SESSION_ID=<id>]               # Attach interactive terminal (F10 to quit)"
	@echo "  make workspace                              # Print workspace snapshot"
	@echo "  make sessions                               # Print sessions list"
	@echo "  make create NAME='Dev'                      # Create a session"
	@echo "  make activate SESSION_ID='<id>'             # Activate desktop session"
	@echo "  make bootstrap-terminal-deps                # Hydrate vendored C deps for desktop runtime"
	@echo "  make verify-third-party-reference-only      # Assert active build/runtime no longer depends on third_party/terminal-engine-reference"
	@echo "  make check                                  # cargo check --workspace"
	@echo "  make check-desktop                      # Check first-party Chatminal Desktop package (requires native GUI dev deps)"
	@echo "  make test                                   # Run core test suites"
	@echo "  make smoke-window                           # Run desktop launcher smoke"
	@echo "  make bench-rtt                              # Run quick RTT benchmark command"
	@echo "  make bench-phase02                          # Run phase-02 RTT+RSS hard gate script"
	@echo "  make fidelity-smoke                         # Run phase-05 fidelity smoke (JSON report)"
	@echo "  make fidelity-matrix-smoke                  # Run phase-03 fidelity matrix smoke strict mode (JSON report)"
	@echo "  make fidelity-matrix-smoke-relaxed          # Run phase-03 fidelity matrix smoke non-strict"
	@echo "  make fidelity-input-ime-smoke               # Run phase-06 modifier/input smoke + IME manual gate report"
	@echo "  make phase06-killswitch-verify              # Verify runtime input pipeline rollback path (desktop/legacy)"
	@echo "  make phase08-killswitch-verify              # Verify desktop launcher/runtime gate"
	@echo "  make soak-smoke                             # Run phase-05 soak smoke (JSON report)"
	@echo "  make release-dry-run                        # Build release artifacts + checksum + smoke"
	@echo ""
	@echo "Optional vars:"
	@echo "  SOCKET=$(SOCKET)"
	@echo "  WIDTH=$(WIDTH) HEIGHT=$(HEIGHT) SIDEBAR_WIDTH=$(SIDEBAR_WIDTH)"
	@echo "  PREVIEW_LINES=$(PREVIEW_LINES) PREVIEW_CHARS=$(PREVIEW_CHARS)"

clean-socket:
	rm -f $(SOCKET)

daemon:
	$(DAEMON_ENDPOINT) cargo run --manifest-path $(DAEMON_MANIFEST)

daemon-reset:
	-pkill -f 'target/debug/chatminald' || true
	$(MAKE) clean-socket
	$(DAEMON_ENDPOINT) cargo run --manifest-path $(DAEMON_MANIFEST)

dashboard:
	@if [ ! -S "$(SOCKET)" ]; then echo "Daemon chưa sẵn sàng tại $(SOCKET). Hãy chạy: make daemon"; exit 1; fi
	$(DAEMON_ENDPOINT) cargo run --manifest-path $(APP_MANIFEST) -- dashboard-tui $(PREVIEW_LINES) $(PREVIEW_CHARS) $(WIDTH) $(HEIGHT) $(SIDEBAR_WIDTH)

window:
	cargo run --manifest-path $(APP_MANIFEST) -- window

attach:
	@if [ ! -S "$(SOCKET)" ]; then echo "Daemon chưa sẵn sàng tại $(SOCKET). Hãy chạy: make daemon"; exit 1; fi
	@if [ -n "$(SESSION_ID)" ]; then \
		$(DAEMON_ENDPOINT) cargo run --manifest-path $(APP_MANIFEST) -- attach "$(SESSION_ID)" $(WIDTH) $(HEIGHT) $(PREVIEW_LINES); \
	else \
		$(DAEMON_ENDPOINT) cargo run --manifest-path $(APP_MANIFEST) -- attach $(WIDTH) $(HEIGHT) $(PREVIEW_LINES); \
	fi

workspace:
	$(DAEMON_ENDPOINT) cargo run --manifest-path $(APP_MANIFEST) -- workspace

sessions:
	$(DAEMON_ENDPOINT) cargo run --manifest-path $(APP_MANIFEST) -- sessions

create:
	@if [ -z "$(NAME)" ]; then echo "Missing NAME. Example: make create NAME='Dev'"; exit 1; fi
	$(DAEMON_ENDPOINT) cargo run --manifest-path $(APP_MANIFEST) -- create "$(NAME)"

activate:
	@if [ -z "$(SESSION_ID)" ]; then echo "Missing SESSION_ID. Example: make activate SESSION_ID='<id>'"; exit 1; fi
	$(DAEMON_ENDPOINT) cargo run --manifest-path $(APP_MANIFEST) -- activate "$(SESSION_ID)" $(WIDTH) $(HEIGHT) $(PREVIEW_CHARS)

bootstrap-terminal-deps:
	bash scripts/bootstrap-terminal-vendor-deps.sh

verify-third-party-reference-only:
	bash scripts/verify-third-party-terminal-reference-only.sh

check:
	bash scripts/verify-third-party-terminal-reference-only.sh
	cargo check --workspace

check-desktop:
	bash scripts/verify-third-party-terminal-reference-only.sh
	cargo check -p chatminal-desktop

test:
	cargo test --manifest-path crates/chatminal-protocol/Cargo.toml
	cargo test --manifest-path crates/chatminal-store/Cargo.toml
	cargo test --manifest-path apps/chatminald/Cargo.toml
	cargo test --manifest-path apps/chatminal-app/Cargo.toml

smoke-window:
	bash scripts/smoke/window-desktop-smoke.sh

bench-rtt:
	@if [ ! -S "$(SOCKET)" ]; then echo "Daemon chưa sẵn sàng tại $(SOCKET). Hãy chạy: make daemon"; exit 1; fi
	$(DAEMON_ENDPOINT) cargo run --manifest-path $(APP_MANIFEST) -- bench-rtt 80 15 2000 $(WIDTH) $(HEIGHT)

bench-phase02:
	bash scripts/bench/phase02-rtt-memory-gate.sh

fidelity-smoke:
	bash scripts/fidelity/phase05-fidelity-smoke.sh

fidelity-matrix-smoke:
	CHATMINAL_FIDELITY_STRICT=1 bash scripts/fidelity/phase03-fidelity-matrix-smoke.sh

fidelity-matrix-smoke-relaxed:
	CHATMINAL_FIDELITY_STRICT=0 bash scripts/fidelity/phase03-fidelity-matrix-smoke.sh

fidelity-input-ime-smoke:
	bash scripts/fidelity/phase06-input-modifier-ime-smoke.sh

phase06-killswitch-verify:
	bash scripts/migration/phase06-killswitch-verify.sh

phase08-killswitch-verify:
	bash scripts/migration/phase08-desktop-killswitch-verify.sh

soak-smoke:
	bash scripts/soak/phase05-soak-smoke.sh

release-dry-run:
	bash scripts/release/phase05-release-dry-run.sh
