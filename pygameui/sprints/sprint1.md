# Sprint 1 — Backend Skeleton + Storage

Goal: a running FastAPI app with `/health`, a DynamoDB-shaped SQLite single-table
repository, an in-memory repository, and a shared contract test suite that both pass.
This is spec Phase 1 (§24) minus tradition-pack loading (deferred to Sprint 2 with the
domain models, since packs are more naturally exercised there).

## Tasks

- [x] Add runtime deps: fastapi, uvicorn, pydantic, httpx, platformdirs. (pygame-ce deferred to Sprint 5.)
- [x] `merit_ledger/local/config.py` — app constants (backend host/port 127.0.0.1:8765, app name).
- [x] `merit_ledger/local/data_dir.py` — resolve local data dir via platformdirs; ensure exists; db path.
- [x] `merit_ledger/backend/repository/base.py` — `MeritItem` dataclass/pydantic + `MeritRepository` ABC
      (put_item, get_item, query_pk, query_gsi1, query_gsi2, delete_item, plus scan_all for export).
- [x] `merit_ledger/backend/repository/memory_repo.py` — `InMemoryMeritRepository`.
- [x] `merit_ledger/backend/repository/sqlite_repo.py` — `SqliteMeritRepository` on the §13.2 table.
- [x] `merit_ledger/backend/main.py` — FastAPI `create_app()`, health router, repo dependency.
- [x] `merit_ledger/backend/api/health.py` — `GET /health`.
- [x] Tests:
  - `tests/test_repo_contract.py` — parametrized over InMemory + Sqlite (tmp file).
  - `tests/test_api_health.py` — TestClient hits /health.
- [x] Run `uv run pytest` green; `uv run mypy` clean on new code.
- [x] Write `sprints/sprint2.md`.

## Design notes

- `MeritItem` fields mirror the SQLite columns exactly: pk, sk, entity_type,
  gsi1pk, gsi1sk, gsi2pk, gsi2sk, item (dict, serialized to item_json), created_at, updated_at.
- Repo stores the item dict as JSON. Query methods return items sorted by sk (Dynamo-like).
- `query_*` with `sk_begins_with=None` returns all under the partition.
- Keep repo interface sync (SQLite is sync; FastAPI can call sync fine for a local app).

## Definition of done

`uv run pytest` passes; `uvicorn merit_ledger.backend.main:app` (or create_app) serves
`GET /health` → `{"status":"ok"}`.
