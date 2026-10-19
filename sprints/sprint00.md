# Sprint 0 — Restructure and skeleton
**State:** done 2026-10-18 (firmware compiles; no board touched)
**Depends on:** —   **Parallel with:** —

## Goal
Move v1 out of the way, and stand up an empty v2 that builds and runs on the desktop and compiles
for the ESP32-S3, following gamematcher's layout. Write the decisions, roadmap and sprints so the
rest can be handed off.

## Read first
- `../microcontroller/miniframework/README.md`, `docs/RECIPES.md` ("Start a new app", "Run on an S3")
- `../microcontroller/gamematcher/README.md`, `Makefile`, `api/` (the reference app)

## Deliverables
- [x] v1 moved to `pygameui/`, including its spec, sprints, tooling and changelog, with an archive banner in
  its README. `make legacy-test` (127 tests) passes from the new location.
- [x] Python CI (`build.yml`, `tox.yml`) runs in `pygameui/`. The PyPI release workflows moved to
  `pygameui/ci-archived/` (with a README) so they no longer fire.
- [x] `meritledger_rs/`: a crate from the gamematcher recipe.
  - Desktop bin `meritledger` (:8089) and board bin `meritledger-s3` (`BoardConfig::s3`).
  - `build.rs`, which reads Wi-Fi from `.env` and copies certificates and the web bundle.
  - S3 `sdkconfig.defaults` (octal PSRAM, 16 MiB) and a proposed `partitions.csv` (6 MiB app, 1 MiB `mlstate`, about 8 MiB kept for OTA).
  - `GET /api/v1/status` and 3 API tests.
- [x] `meritledger_ui/`: an Angular 22 shell using `miniframework-ng`, showing status and the footer
  line, with 2 vitest tests. Production build is 222 kB raw, 62 kB transferred.
- [x] Root `Makefile` (Git Bash): `test`, `check`, `run`, `web-dev`, `web`, `run-bundle`, `certs`,
  `firmware`, `legacy-test`.
- [x] `tools/build-s3.sh`: compiles `xtensa-esp32s3-espidf` and checks the image against the app partition.
- [x] `make certs`: the `meritledger.local` leaf from the household CA (key gitignored).
- [x] `spec/v2_decisions.md`, `spec/domain.md`, `spec/roadmap.md`, and sprint files 0–14.
- [x] New root `README.md`, `AGENTS.md` and `DEPLOY.md`.

## Exit criteria
- [x] `make check` green.
- [x] `make run-bundle` serves the UI and `/api/v1/status` on :8089.
- [x] `make firmware` produces `meritledger_rs/.embuild/meritledger-s3.bin` within the 6 MiB partition.

## Out of scope
Flashing the board (Sprint 1), storage, accounts and any product feature.

## Handoff
- **Framework dependency** is a path dependency on `../../microcontroller/miniframework`, and the UI's
  `tsconfig.json` maps `miniframework-ng` to that repo's source. Both repos must be checked out side
  by side. GitHub CI can't build v2 yet (see the roadmap's risks).
- **The desktop port is 8089**, so it doesn't collide with gamematcher's 8088.
- **The partition table is proposed, not measured.** Sprint 1 confirms it, along with the PSRAM and TLS settings,
  on real hardware.
- **Stale root `.venv`:** it belongs to v1. `pygameui/` now has its own `.venv`, and the old one can be deleted.
- **Read the Docs:** `.readthedocs.yaml` moved to `pygameui/`, so v1's docs build is effectively off.
