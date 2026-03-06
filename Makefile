SHELL := /bin/bash
UI_BIN_TSC := ui/node_modules/.bin/tsc

.PHONY: help setup ui-install ui-build ui-test rust-test test run release-bin fmt clean

help:
	@echo "Available targets:"
	@echo "  make setup      - install UI dependencies"
	@echo "  make ui-build   - build the UI bundle"
	@echo "  make run        - build UI, then run the Tauri app"
	@echo "  make release-bin - build UI and produce a release binary"
	@echo "  make rust-test  - run Rust tests"
	@echo "  make ui-test    - run UI tests"
	@echo "  make test       - run Rust and UI tests"
	@echo "  make fmt        - run Rust formatter"
	@echo "  make clean      - remove build/install artifacts"

setup: ui-install

ui-install:
	cd ui && npm ci

$(UI_BIN_TSC): ui/package-lock.json
	cd ui && npm ci

ui-build: $(UI_BIN_TSC)
	cd ui && npm run build

run: ui-build
	cargo run

release-bin: ui-build
	cargo build --release -p rtsp_viewer_tauri

rust-test:
	cargo test

ui-test: $(UI_BIN_TSC)
	cd ui && npm test

test: rust-test ui-test

fmt:
	cargo fmt

clean:
	cargo clean
	rm -rf ui/dist ui/node_modules
