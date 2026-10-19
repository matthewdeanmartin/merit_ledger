# Sprint 4 — Repentance, Dedication, Mudita, Stats, Markdown Export, JSON Import

Goal: finish the backend's domain surface so the whole API the Pygame frontend will need
(Sprints 5-6) exists and is tested. Spec Phase 4 (§24) + stats (§14.3) + export/import (§19).

## Prereqs / context from Sprints 1-3 (DONE)

- Package `merit_ledger/` at root. Backend under `merit_ledger/backend/`.
- Storage abstracted by `MeritRepository` (base.py); InMemory + thread-safe Sqlite. `scan_all()`.
- Keys ONLY in `backend/repository/item_keys.py` (profile/settings/template/entry/vow helpers).
- Domain models in `backend/domain/models.py`: LedgerEntry already has category, repentance_style,
  repair_intention, linked_vow_id, dedication_id fields. Enums exist: RepentanceCategory,
  DedicationTarget. ids.py: `new_id(prefix)`, `now_iso()`.
- Services take `repo` first, `user_id=DEFAULT_USER_ID` last. entry_service.create_entry scores;
  list_entries supports date_prefix / entry_type / vow_id filters.
- vow_service.breach_vow currently records a `vow_breached` entry with repair_intention + category
  and sets repair_in_progress. Sprint 4 links repentance to that (see §5.4 "repentance created or
  scheduled", §10 linked_vow_id).
- API: routers wired in backend/main.py via `create_app`; `client` fixture (InMemory) in conftest.
- Tests: `uv run pytest`; typecheck `uv run mypy merit_ledger`. Both green (68 tests).

## Tasks

### Repentance (spec §5.7, §10)
- [x] `services/repentance_service.py`:
      - `categories()` → the 9 RepentanceCategory values (+ maybe display labels from tradition pack).
      - `create_repentance(repo, *, category, title?, reflection?, repair_intention?,
        linked_vow_id?, dedicate=None)` → creates a `repentance_completed` LedgerEntry (points from
        tradition pack point_defaults["repentance"], scored via entry_service), returns it.
        If linked_vow_id and that vow is in repair_in_progress, optionally complete the repair
        (call vow_service.complete_vow) — decide + document; simplest: leave vow status alone and
        just link. Keep the "no secrets" contract: no free-form who/what fields beyond reflection.
- [x] `api/repentance.py`: `GET /repentance/categories`, `POST /repentance`. Wire in.
      Include the privacy reminder string in the categories response (spec §5.7 default UX copy).

### Dedication (spec §5.8, §11)
- [x] `models.py`: add `Dedication` model (spec §11): dedication_id, user_id, source_entry_id?,
      target_type (DedicationTarget), target_name, dedication_text, points_dedicated, created_at.
- [x] `item_keys.py`: `to_dedication_item`/`dedication_from_item`; SK=`DEDICATION#<created_at>#<id>`
      (spec §13.3), entity_type="DEDICATION". Add DEDICATION_SK_PREFIX.
- [x] `services/dedication_service.py`: create_dedication (default does NOT reduce balance —
      spec §5.8; respect settings.dedication_reduces_balance flag but the balance math can be a
      no-op stub with a TODO since there's no running "balance" entity yet), list_dedications,
      preset targets from tradition pack dedication_language (+ secular replacements §18).
      When a dedication is created from an entry, set entry.dedication_id (update the entry).
- [x] `api/dedication.py`: `GET /dedications`, `POST /dedications` (+ maybe GET presets). Wire in.

### Mudita demo (spec §5.9, §16)
- [x] Sample feed data: a small module or JSON of anonymous wholesome actions (spec §5.9 examples),
      optionally tradition-flavored. `services/mudita_service.py`: `demo_feed(tradition)` returns
      sample entries with rejoice-verb from the pack (Rejoice/Sadhu/Namu/Anumodana/Appreciate);
      `rejoice(repo, sample_id/text)` → creates a `mudita_rejoiced` LedgerEntry (points from pack
      rejoicing default). No network.
- [x] `api/mudita.py`: `GET /mudita/demo-feed`, `POST /mudita/rejoice`. Wire in.

### Stats (spec §14.3)
- [x] `services/stats_service.py` + `domain/stats.py` (pure aggregation): today/week/month point
      totals + counts, by-template, by-tradition, vows summary (counts by status, active streaks).
      Pull entries via entry_service.list_entries; keep aggregation pure/testable.
- [x] `api/stats.py`: `GET /stats/today|week|month|by-template|by-tradition|vows`. Wire in.

### Export/import (spec §19)
- [x] `export_service.export_markdown(repo)` (spec §19.2 format) — human-readable; include the
      privacy warning text somewhere the API/frontend can surface (spec §19.3).
- [x] `import_service` (or in export_service): `import_json(repo, snapshot)` — validate version,
      upsert profile/settings/templates/vows/entries/dedications via item_keys. Idempotent-ish.
- [x] `api/export.py`: add `GET /export/markdown` (text/markdown) and `POST /import/json`. Wire in.

### Wrap
- [x] Tests: test_repentance.py, test_dedication.py, test_api_repentance.py, test_api_dedication.py,
      test_mudita.py, test_stats.py, test_import.py, test_export_markdown.py.
- [x] `uv run pytest` + `uv run mypy merit_ledger` green.
- [x] Write `sprints/sprint5.md` (Pygame shell — this is the big frontend pivot; note pygame-ce
      must be added via `uv add pygame-ce`, and the server_process manager + app.py launcher land here).

## Design notes / decisions to make
- Dedication "reduces balance": there is no persistent balance entity in the MVP. Default is
  "does not reduce" (spec §5.8) so implement create as record-only; honor the flag by storing it
  but do not compute a balance. Document the stub.
- Repentance linked to a broken vow: keep it simple — link via linked_vow_id and leave the vow in
  repair_in_progress unless the user explicitly completes the repair. Note this in code + sprint5 handoff.
- Keep all reflection fields optional and never prompt for secrets (spec §2.4, §5.7). Backend must
  not add any "who/what/where" fields.

## Definition of done
Create repentance in a category (points from pack); create a dedication to "All beings" that links
to an entry; rejoice at a mudita sample and get a mudita_rejoiced entry; GET stats/today reflects
totals; export markdown renders; import_json round-trips a prior export/json snapshot.
