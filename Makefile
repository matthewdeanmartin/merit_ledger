SHELL := bash
.DEFAULT_GOAL := help

API := meritledger_rs
UI := meritledger_ui
MF := ../microcontroller/miniframework
CARGO ?= cargo

.PHONY: help test test-rust test-web check run web web-dev run-bundle certs firmware board-check flash update gate legacy-test

help:
	@printf '%s\n' \
	  'Merit Ledger v2: Rust API ($(API)/) + Angular UI ($(UI)/) on an ESP32-S3.' \
	  'Needs the sibling repo C:\github\microcontroller (miniframework).' \
	  '' \
	  'make test        Rust tests and a UI type check + unit tests' \
	  'make check       test + fmt + clippy + production UI build' \
	  'make run         API on http://127.0.0.1:8089' \
	  'make web-dev     Angular dev server on http://localhost:4200 (proxies /api to :8089)' \
	  'make run-bundle  build the UI and serve UI + API from :8089' \
	  '' \
	  'Board (Waveshare ESP32-S3-Touch-LCD-1.47; see DEPLOY.md):' \
	  'make certs       issue meritledger.local from the household CA' \
	  'make firmware    build only, never touches a board; TRANSPORT=http|https|http2 (default http2), GATE=1 for the gate' \
	  'make board-check PORT=COMn   read-only: identity, and would `make update` be allowed?' \
	  'make flash PORT=COMn         FIRST INSTALL / REPAVE: erases the whole board' \
	  'make update PORT=COMn        app only; refuses unless the board has our partition table' \
	  'make gate URL=http://<board> Sprint 1 load, outbound HTTPS and SD measurement' \
	  '' \
	  'v1 (archived Python app): cd pygameui && uv run make help' \
	  'make legacy-test run the v1 tests'

test: test-rust test-web

test-rust:
	cd $(API) && $(CARGO) test
	cd $(API) && $(CARGO) test --features gate gate

test-web:
	cd $(UI) && npx tsc -p tsconfig.app.json --noEmit && npx vitest run

check: test
	cd $(API) && $(CARGO) fmt --check && $(CARGO) clippy --all-targets -- -D warnings
	cd $(API) && $(CARGO) clippy --all-targets --features gate -- -D warnings
	cd $(UI) && npx ng build

run:
	cd $(API) && $(CARGO) run

web-dev:
	cd $(UI) && npx ng serve

web:
	cd $(UI) && npx ng build
	node $(MF)/tools/bundle-web.mjs $(UI)/dist/meritledger/browser $(API)/.embuild/web

run-bundle: web
	cd $(API) && $(CARGO) run --features bundled-web

certs:
	bash $(MF)/tools/certs.sh $(API) meritledger

TRANSPORT ?= http2
firmware: web
	@case "$(TRANSPORT)" in http|https|http2) ;; *) echo 'TRANSPORT must be http, https or http2' >&2; exit 2;; esac
	features="$(filter-out http,$(TRANSPORT))"; \
	if [ -n "$(GATE)" ]; then features="$${features:+$$features,}gate"; fi; \
	bash tools/build-s3.sh "$$features"

ESP_PYTHON ?= C:/Espressif/python_env/idf5.5_py3.11_env/Scripts/python.exe
NEED_PORT = @[ -n "$(PORT)" ] || { echo 'Set PORT=COMn (the S3 enumerates as 303A:1001).' >&2; exit 2; }

board-check:
	$(NEED_PORT)
	"$(ESP_PYTHON)" tools/deploy-s3.py --port "$(PORT)" --check

flash:
	$(NEED_PORT)
	"$(ESP_PYTHON)" tools/deploy-s3.py --port "$(PORT)" --replace

update:
	$(NEED_PORT)
	"$(ESP_PYTHON)" tools/deploy-s3.py --port "$(PORT)" --update

gate:
	@[ -n "$(URL)" ] || { echo 'Set URL=http://<board ip>' >&2; exit 2; }
	uv run tools/gate.py --url "$(URL)" --ca $(API)/certs/household-ca.crt $(ARGS)

legacy-test:
	cd pygameui && uv run pytest -q
