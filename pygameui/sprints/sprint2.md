# Sprint 2 — Domain Models + Templates + Ledger + Scoring + JSON Export

Goal: real entities flow through the stack. A user can create practice templates and
ledger entries via the API, points are computed by a scoring engine, tradition packs
load, and the whole ledger can be exported to JSON. Spec Phase 2 (§24) + pack loading
from Phase 1.

## Prereqs / context from Sprint 1 (DONE)

- Repo layout: package at `merit_ledger/` (root, not src). Backend under `merit_ledger/backend/`.
- `MeritItem` + `MeritRepository` in `backend/repository/base.py`; InMemory + Sqlite impls pass
  shared `tests/test_repo_contract.py`. `scan_all()` exists for export.
- FastAPI app: `backend/main.py::create_app(repo=None)`, sets `app.state.repo`. `/health` works.
- `local/config.py` (DEFAULT_USER_ID="local_user", BACKEND_URL, DB_FILENAME) and
  `local/data_dir.py` (`data_dir()`, `db_path()`).
- Run tests: `uv run pytest`. Typecheck: `uv run mypy merit_ledger`. Both green.

## Tasks

- [x] `backend/domain/models.py` — pydantic models matching spec:
      `PracticeTemplate` (§7), `LedgerEntry` (§9, all entry_type literals), `Profile`,
      `Settings` (points mode: points/count_only/reflection_only; negative_points flag).
      Use `Literal[...]` for enums. Add id + timestamp helpers (ULID-ish: use `ulid`? no new
      dep — use `uuid4().hex` with a sortable time prefix, or `entry_{iso}_{uuid8}`).
      Provide a small `ids.py` for `new_id(prefix)` and `now_iso()`.
- [x] `backend/repository/item_keys.py` — build pk/sk/gsi keys per spec §13.3 for each entity
      (profile, template, entry, vow, dedication). Include entry→MeritItem and MeritItem→entry
      (de)serialization helpers so services stay clean. Unit-test key formats.
- [x] `backend/domain/scoring.py` — pure scoring per §12: points mode vs count-only vs
      reflection-only; base_points * quantity_multiplier; daily_cap; manual override.
      No I/O. Property-test with hypothesis (cap never exceeded, override respected).
- [x] Tradition packs: write the 5 JSON packs in `backend/tradition_packs/`
      (zen, chinese_mahayana, nichiren, pure_land, secular) with the §6 fields
      (display_name, labels, suggested_practices as template dicts, repentance_categories,
      dedication_language, rejoicing_language, point_defaults, glossary). Keep them modest but real.
- [x] `backend/domain/traditions.py` (or `services/tradition_service.py`) — loader that reads packs
      from the package dir (use `importlib.resources`), validates, lists, gets by id.
- [x] `services/entry_service.py` — create/list/get/update/delete entries; on create, run scoring;
      persist via repo + item_keys. list supports filter by date-prefix and by entry_type (GSI1).
- [x] `services/template_service.py` — CRUD custom templates + list (merge pack-suggested + custom).
- [x] `services/export_service.py` — `export_json()` returns full snapshot (profile, settings,
      tradition, templates, vows[], entries, dedications) by scanning repo.
- [x] API routers, wire into `main.py`:
      `api/templates.py` (§14.3 templates), `api/entries.py` (entries CRUD),
      `api/traditions.py` (list/get, PUT /settings/tradition), `api/export.py` (GET /export/json).
      Use a `deps.py` providing `get_repo(request)` from `app.state.repo`.
- [x] `api/profile.py` + `api/settings.py` — GET/PUT profile & settings (needed by onboarding later).
- [x] Tests: `test_scoring.py`, `test_item_keys.py`, `test_traditions.py`,
      `test_api_entries.py`, `test_api_templates.py`, `test_export.py`. Keep API tests on InMemory repo.
- [x] `uv run pytest` + `uv run mypy merit_ledger` green.
- [x] Write `sprints/sprint3.md`.

## Design notes

- Keep the single-table discipline: services build keys ONLY via `item_keys.py`.
- Entry create flow: validate template exists (pack or custom) → compute points via scoring
  (respecting settings mode) → assign id/timestamps → write MeritItem. Return the LedgerEntry.
- For "entries by date": `query_pk(pk, "ENTRY#2026-07")`. For "by type": GSI1
  `USER#local_user#ENTRY_TYPE#practice_completed`. For "by tradition": GSI2.
- Settings/profile are single items at SK=SETTINGS / SK=PROFILE under PK=USER#local_user.
- Don't implement vows/repentance/dedication endpoints yet beyond what export needs (export can
  scan-all and bucket by entity_type, so it won't break as later sprints add entities).

## Definition of done

Create a template (or use a pack one), POST an entry, GET it back with computed points,
GET /export/json returns a coherent snapshot. Secular vs Buddhist labels resolvable from packs.
