# Merit Ledger — Sprint Plan (master roadmap)

This is the top-level roadmap. Each sprint has its own `sprintN.md` with a checklist.
After finishing a sprint, the *next* sprint's file is always written out so any LLM
(or human) can pick up cold. See `spec/spec.md` for the full product spec.

## Ground rules / decisions

- **Package layout:** existing scaffold uses `merit_ledger/` at repo root (NOT `src/`).
  Respect it. Entry point is `merit_ledger.cli:main` per `pyproject.toml`.
- **Tooling:** `uv` for everything. Run tests with `uv run pytest`. mypy is `strict`.
- **Deps needed** (add via `uv add`): fastapi, uvicorn, pydantic, httpx, platformdirs, pygame-ce.
- **Architecture (per spec §13, §14):** domain logic must not know about SQLite or Pygame.
  - `merit_ledger/backend/domain/` — pydantic models, scoring, pure logic
  - `merit_ledger/backend/repository/` — base interface + sqlite/memory/dynamodb impls, item_keys
  - `merit_ledger/backend/services/` — orchestration over repo + domain
  - `merit_ledger/backend/api/` — FastAPI routers
  - `merit_ledger/backend/tradition_packs/` — JSON packs
  - `merit_ledger/frontend/` — pygame app + scenes + ui
  - `merit_ledger/local/` — data_dir, config, server_process
- **Storage:** SQLite single-table (pk, sk, entity_type, gsi1pk/sk, gsi2pk/sk, item_json, created_at, updated_at).
- **Repos share one contract test** run against InMemory + Sqlite.

## Sprint breakdown (maps to spec §24 build phases)

- **Sprint 1 — Backend skeleton + storage.** deps, data_dir/config, MeritItem, item_keys,
  repository base + InMemory + Sqlite, shared contract tests, FastAPI app w/ /health, run it. ✅ target
- **Sprint 2 — Domain models + Ledger + Templates + Scoring + JSON export.**
  Pydantic entity models, tradition pack loader + 5 packs, entry_service, scoring engine,
  templates + entries API, export/json. Tests.
- **Sprint 3 — Vows.** vow model + state machine, vow_service, vow API
  (create/pause/resume/retire/complete/breach), repair status. Tests.
- **Sprint 4 — Repentance + Dedication.** repentance categories + service + API,
  dedication targets/presets + service + API, linked repentance for breaches,
  mudita demo feed + rejoice, stats service + API, markdown export, import/json. Tests.
- **Sprint 5 — Pygame shell.** server_process manager, app.py launcher, theme/ui primitives,
  splash, onboarding, dashboard, navigation, API client.
- **Sprint 6 — Pygame core screens.** record, vows, vow detail, repentance, dedication,
  stats, settings, mudita garden scenes.
- **Sprint 7 — Beauty pass.** tradition themes, cards, animations, typography, reduced motion.
- **Sprint 8 — Packaging.** CLI wiring (`merit_ledger` launches app), README, screenshots, release notes.

## Status log

- Sprint 1: ✅ done (2026-07-05) — deps, config/data_dir, MeritItem + 2 repos + contract tests,
  FastAPI create_app + /health. `uv run pytest` (21) + `uv run mypy` green.
- Sprint 2: ✅ done (2026-07-05) — domain models + ids, item_keys, scoring engine,
  5 tradition packs + loader, profile/template/entry/export services, API routers
  (profile, settings, traditions, templates, entries, export/json) wired into create_app.
  Fixed SqliteMeritRepository for FastAPI threadpool (check_same_thread=False + lock).
  50 tests + hypothesis props green; mypy clean (36 files).
- Sprint 3: not started — see sprints/sprint3.md. (Vows.)
