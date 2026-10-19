# Sprint 3 — Vows

Goal: users can create positive/negative vows, complete positive vows, breach negative
vows, and pause/resume/retire them — with a clean state machine and a "repair in
progress" status after a breach. Spec Phase 3 (§24), flows §5.3–§5.6, model §8, API §14.3.

## Prereqs / context from Sprints 1–2 (DONE)

- Package at `merit_ledger/` (root). Backend under `merit_ledger/backend/`.
- Storage: `MeritItem` + `MeritRepository` (base.py); InMemory + Sqlite (thread-safe:
  `check_same_thread=False` + lock). `scan_all()` for export.
- Keys built ONLY in `backend/repository/item_keys.py` (has profile/settings/template/entry
  helpers + `user_pk`). Add vow + dedication helpers there.
- Domain models in `backend/domain/models.py` (LedgerEntry has vow_id, linked_vow_id,
  category, repair_intention fields already). ids.py: `new_id(prefix)`, `now_iso()`.
- Services pattern: pure functions taking `repo` first arg, `user_id=DEFAULT_USER_ID` last.
  See entry_service / template_service / profile_service.
- API pattern: router + `Depends(get_repo)` from `api/deps.py`; wire into `backend/main.py`.
- Ledger entry_types already include vow_completed, vow_breached, vow_created, vow_paused,
  vow_resumed, vow_retired — creating vow lifecycle entries is just entry_service.create_entry
  with the right entry_type (points 0 unless spec says otherwise; completions get points).
- Tests: `uv run pytest`; typecheck `uv run mypy merit_ledger`. Keep API tests on InMemory repo
  via the `client` fixture in tests/conftest.py.

## Tasks

- [x] `backend/domain/models.py` — add `Vow` model (spec §8): vow_id, user_id, name, description,
      vow_type ("positive"|"negative"), strength ("aspiration"|"training_commitment"|
      "formal_vow"|"experiment"), status ("draft"|"active"|"paused"|"repair_in_progress"|
      "completed"|"retired"), frequency (free string: "continuous"|"daily"|...),
      start_date, end_date, default_points, repentance_category (optional), pause_reason (opt),
      resume_date (opt), created_at, updated_at. Use `new_id("vow")` default.
      Also a small `VowStreak`/`streak` int on the vow (spec §5.3 "Streak updated") — keep simple:
      store `streak` and `last_completed_date` on the Vow.
- [x] `backend/domain/vow_state.py` — pure state-transition helpers + allowed-transition table.
      Functions like `can_pause(status)`, `apply_complete`, `apply_breach`, `apply_pause`,
      `apply_resume`, `apply_retire` returning the new status (raise/return error on illegal).
      Unit-test the table (e.g. can't pause a retired vow; breach → repair_in_progress).
- [x] `item_keys.py` — `to_vow_item`/`vow_from_item`; SK=`VOW#<vow_id>`; entity_type="VOW";
      GSI1 by status: gsi1pk=`USER#<u>#VOW_STATUS#<status>`, gsi1sk=`VOW#<vow_id>`.
      Add `VOW_SK_PREFIX`.
- [x] `services/vow_service.py`:
      - create_vow (status defaults active unless draft), list_vows (optionally by status via GSI1),
        get_vow, update_vow.
      - complete_vow(positive): create `vow_completed` ledger entry with points (vow.default_points,
        scored via entry_service so points-mode is honored), bump streak/last_completed_date.
      - breach_vow(negative): create `vow_breached` entry (0 points; negative points only if
        settings.negative_points_enabled), set status → repair_in_progress, accept optional
        category + note + repair intention. (The actual repentance entry creation is Sprint 4;
        here just record the breach + optionally stash repair_intention on the entry.)
      - pause_vow(reason, resume_date?) → status paused + `vow_paused` entry.
      - resume_vow(new_frequency?, new_points?, renewed_intention?) → status active + `vow_resumed`.
      - retire_vow → status retired + `vow_retired` entry.
      When status changes, re-put the vow so its GSI1 status key updates.
- [x] `api/vows.py` (spec §14.3): GET /vows (query ?status=), GET /vows/{id}, POST /vows,
      PUT /vows/{id}, POST /vows/{id}/pause, /resume, /retire, /complete, /breach.
      Wire into main.py. Use small pydantic bodies for pause/resume/breach inputs.
- [x] Export: vows already bucket via scan_all (entity_type VOW) — verify export includes them.
- [x] Tests: `test_vows.py` (state machine + service), `test_api_vows.py`
      (create/pause/resume/complete/breach/retire, list by status). Cover:
      breach sets repair_in_progress; paused vow is not "failed"; complete bumps streak.
- [x] `uv run pytest` + `uv run mypy merit_ledger` green.
- [x] Write `sprints/sprint4.md`.

## Design notes

- Keep vow lifecycle events as ledger entries so the timeline (Sprint 6 vow detail) is just
  `entry_service.list_entries` filtered to a vow_id. Consider adding an optional
  `vow_id` filter to entry_service.list_entries (scan + filter is fine locally).
- Don't implement negative points math beyond respecting the settings flag; default off (spec §12.3).
- "Repair available, not failure" is UI copy (Sprint 6/7) — backend just uses status
  `repair_in_progress`; never a shame score.

## Definition of done

Create a negative vow → breach it → status is repair_in_progress and a vow_breached entry
exists. Create a positive vow → complete it → vow_completed entry with points + streak=1.
Pause then resume a vow → status transitions and timeline entries recorded. List vows by status works.
