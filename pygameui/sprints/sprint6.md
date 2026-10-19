# Sprint 6 — Pygame Core Screens

Goal: make every main tab functional — Record, Vows, Vow Detail, Repentance, Dedication,
Stats, Settings, and the Mudita Garden. After this the app is a complete, usable local
tool (beauty polish is Sprint 7). Spec Phase 6 (§24), flows §5.2-§5.9, screens §15.

## Prereqs / context from Sprints 1-5 (DONE)

- Backend COMPLETE (all endpoints, 88 backend tests). Frontend SHELL done:
  - `frontend/pygame_app.py`: `Scene` base (on_enter/handle_event/update/draw) + `PygameApp`
    (owns `api`, `palette`, `switch_to`, main loop). `app.refresh_palette()` re-themes from settings.
  - `frontend/api_client.py`: `ApiClient` — thin httpx wrapper. Scenes use ONLY this for data.
    Currently has: profile, settings, traditions, set_tradition, templates, entries (list/create),
    vows (list), stats_today. **Add methods here as new scenes need them** (see below).
  - `frontend/theme.py`: `palette_for_theme(theme)` → Palette (bg_top/bottom, card, text,
    text_muted, accent). `high_contrast()` variant available.
  - `frontend/ui/`: `text.py` (draw_text/draw_text_center), `cards.py` (draw_vertical_gradient,
    draw_card), `buttons.py` (Button: label/rect/on_click, hover, hit-test).
  - Scenes: `splash.py` (routes via `next_scene_name(settings)`), `onboarding.py` (mode→points→
    name, persists + sets settings.onboarded), `dashboard.py` (today stats, active vows, recent
    entries, NAV button row that currently calls `_nav()` = NO-OP placeholder).
- Launch: `uv run merit_ledger` (or `--backend-only`). Tests headless with `SDL_VIDEODRIVER=dummy`.
- Rules: no branch/commit (user's job); `uv run`; keep logic server-side; strict mypy
  (frontend/api_client has a warn_return_any override — extend it only if truly needed).

## Tasks

### api_client additions (add methods for each endpoint the scenes need)
- [x] get_entry, update_entry, delete_entry; get_tradition already there (use for suggested
      practices/labels). create via existing create_entry.
- [x] vows: get_vow, create_vow, pause/resume/retire/complete/breach (POST bodies per API),
      list_vows(status) already exists.
- [x] repentance: get_categories, create_repentance.
- [x] dedications: list, presets, create.
- [x] mudita: demo_feed, rejoice.
- [x] stats: today (have), week, month, by_template, by_tradition, vows.
- [x] export: get export/json, export/markdown (text), import/json — for Settings > Data.
      (Keep the client returning raw dict/text; scenes format.)

### Navigation
- [x] A shared nav mechanism: extend DashboardScene `_nav(name)` to switch scenes, and give each
      main scene a "back to Today" affordance (button or Esc). Simplest: a `NavBar` helper in ui/
      drawn on each scene, or a `BaseScene` that renders the tab row + routes. Pick one and be
      consistent. Map: Record→RecordPracticeScene, Vows→VowsScene, Repent→RepentanceScene,
      Dedicate→DedicationScene, Mudita→MuditaGardenScene, Stats→StatsScene, Settings→SettingsScene.

### Scenes (spec §15.1)
- [x] `scenes/record_practice.py` (§5.2): pick a template (from list_templates; show name +
      default points), optional quantity, optional reflection, optional dedication toggle, Save →
      create_entry. Confirm feedback ("Recorded").
- [x] `scenes/vows.py`: list vows (active first; paused dimmed not red — spec §15.5), button to
      create a vow (name + type positive/negative), tap a vow → VowDetailScene.
- [x] `scenes/vow_detail.py` (§5.3-§5.6): show vow status + streak; actions by type/status:
      positive→Complete; negative→Record breach (→ RepentanceScene prefilled with linked_vow_id +
      category); Pause with care (reason), Resume, Retire. Show timeline via
      list_entries(vow_id=...). "repair available" not "failure" for repair_in_progress.
- [x] `scenes/repentance.py` (§5.7): choose category (get_categories), show privacy reminder
      prominently, optional non-secret reflection, repair intention, Save → create_repentance.
      Accept optional prefilled linked_vow_id/category when navigated from a breach.
- [x] `scenes/dedication.py` (§5.8): choose preset target (presets) or custom, dedication text
      (default from presets), points, Save → create dedication. Optionally link to a recent entry.
- [x] `scenes/stats.py` (§15): show today/week/month totals + by-template and vows summary.
      Simple bar rows are fine (no chart lib); if you draw charts, read the dataviz skill first.
- [x] `scenes/settings.py` (§17): tradition switch (set_tradition + app.refresh_palette),
      points mode, negative points toggle, reduced motion, high contrast, sound; Data section:
      export JSON/Markdown (write to data_dir, show path + privacy warning §19.3), import JSON
      (from a path), clear local data (confirm). Persist via put_settings.
      NOTE: backend + client for clear-data are DONE (added after Sprint 5): `POST /settings/clear`
      (requires `{"confirm": true}`, else 400; factory reset — settings revert, onboarding reruns)
      and `ApiClient.clear_data()`. The Settings scene just needs a confirm dialog that calls it,
      then route back to splash/onboarding. Repo gained `clear()` (contract-tested).
- [x] `scenes/mudita_garden.py` (§5.9, §16): show demo_feed entries as cards with a Rejoice
      button (label = feed verb); on rejoice → mudita rejoice + a small "flower" marker (a drawn
      circle is fine now; real animation Sprint 7).

### Wrap
- [x] Tests (spec §23.4 — no rendering assertions; test state/routing with a fake/TestClient api):
      record creates an entry; vow_detail chooses correct actions per status; breach routes to
      repentance with prefilled category; settings tradition switch calls set_tradition; mudita
      rejoice calls the endpoint. Construct scenes with an ApiClient over a TestClient app and call
      the handler methods directly (as done in the Sprint 5 smoke test).
- [x] Keep pygame imports safe headless; run suite with SDL_VIDEODRIVER=dummy.
- [x] `uv run pytest` + `uv run mypy merit_ledger` green. Launch `uv run merit_ledger` and click
      through every tab. Use /run or a screenshot.
- [x] Write `sprints/sprint7.md` (Beauty pass: tradition themes polish, card UI, animations —
      lotus appears on record, light flow on dedicate, flower in mudita — typography, reduced-motion).

## Design notes
- Reuse Button + card helpers; add a scrollable list helper if entry/vow lists overflow (simple
  offset + mouse wheel). Keep each scene's data load in on_enter/refresh via api_client.
- Route breach → repentance by constructing RepentanceScene with linked_vow_id + category set; on
  save, also nothing extra needed (backend breach already set repair_in_progress in Sprint 3; the
  repentance links to the vow). Decide if "Complete repair" should call complete_vow — offer it in
  vow_detail for repair_in_progress vows.
- Settings "clear local data": backend `POST /settings/clear` + `ApiClient.clear_data()` already
  exist (done post-Sprint 5). Just wire a confirm dialog in the Settings scene and route to splash.

## Definition of done
Every tab opens and works: record a practice, create+complete a positive vow, breach a negative vow
and complete its repair via repentance, dedicate merit, rejoice in the garden, view stats, and
change tradition in settings (theme updates live). Tests + mypy green; app clickable end-to-end.

## STATUS: done (2026-07-05) — with deliberate simplifications carried to Sprint 7

All 8 scenes exist, extend a shared `MainScene` (nav bar in `scenes/nav.py`), load via api_client,
and render headlessly without crashing. 115 tests + mypy + ruff all green (added flake8-bugbear
extend-immutable-calls for FastAPI Depends; fixed a few pre-existing backend lint nits).

Simplified vs. the task text (all non-blocking; fold into Sprint 7 polish):
- Record: one-tap record from template defaults. No quantity/reflection text entry or dedication
  toggle yet (needs a text-input widget — build one in Sprint 7 and reuse in repentance/dedication).
- Repentance/Dedication: category/target buttons save immediately with defaults; no free-text
  reflection / custom target / points entry yet (same text-input-widget dependency).
- Settings: tradition switch (live re-theme) + BOTH clear-data scopes (user_data / all) with a
  confirm step are done. NOT yet: points-mode/negative-points/reduced-motion/high-contrast/sound
  toggles, and export/import UI. api_client already has stats_*, export/import methods stubbed
  where needed; the toggles just need checkbox widgets + put_settings.
- Mudita "flower" is a drawn accent circle (real animation is Sprint 7).
- No scroll helper yet; lists are capped (entries[:6], vows[:7], templates[:8]).
