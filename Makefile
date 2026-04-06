SHELL := /bin/bash

DESKTOP_MANIFEST := apps/desktop/Cargo.toml
DEFAULT_DATA_DIR := $(HOME)/Library/Application Support/chatminal
DEFAULT_CACHE_DIR := $(HOME)/Library/Caches/chatminal
DEFAULT_LOG_DIR := $(HOME)/Library/Logs/chatminal
FALLBACK_LOCAL_DATA_DIR := $(HOME)/.local/share/chatminal

.PHONY: help clean clean-data window bootstrap-terminal-deps verify-third-party-reference-only check check-desktop test

help:
	@echo "Chatminal desktop shortcuts:"
	@echo "  make clean-data                             # Kill Chatminal processes and remove local app data/cache/logs"
	@echo "  make clean                                  # Alias of clean-data"
	@echo "  make window                                 # Run Chatminal Desktop unified shell"
	@echo "  make bootstrap-terminal-deps                # Hydrate vendored C deps for desktop runtime"
	@echo "  make verify-third-party-reference-only      # Assert active build/runtime no longer depends on third_party/terminal-engine-reference"
	@echo "  make check                                  # cargo check --workspace"
	@echo "  make check-desktop                          # cargo check -p chatminal-desktop"
	@echo "  make test                                   # Run core desktop/runtime test suites"

clean: clean-data

clean-data:
	-pkill -f 'chatminal-desktop|chatminal-mux' || true
	@if [ -n "$$CHATMINAL_DATA_DIR" ]; then \
		echo "Removing CHATMINAL_DATA_DIR=$$CHATMINAL_DATA_DIR"; \
		rm -rf "$$CHATMINAL_DATA_DIR"; \
	fi
	-rm -rf "$(DEFAULT_DATA_DIR)" "$(DEFAULT_CACHE_DIR)" "$(DEFAULT_LOG_DIR)" "$(FALLBACK_LOCAL_DATA_DIR)"

window:
	cargo run --manifest-path $(DESKTOP_MANIFEST)

bootstrap-terminal-deps:
	bash scripts/bootstrap-terminal-vendor-deps.sh

verify-third-party-reference-only:
	bash scripts/verify-third-party-terminal-reference-only.sh

check:
	bash scripts/verify-third-party-terminal-reference-only.sh
	cargo check --workspace

check-desktop:
	bash scripts/verify-third-party-terminal-reference-only.sh
	cargo check -p desktop

test:
	cargo test -p runtime
	cargo test --manifest-path crates/store/Cargo.toml
