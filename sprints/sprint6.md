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
- [ ] get_entry, update_entry, delete_entry; get_tradition already there (use for suggested
      practices/labels). create via existing create_entry.
- [ ] vows: get_vow, create_vow, pause/resume/retire/complete/breach (POST bodies per API),
      list_vows(status) already exists.
- [ ] repentance: get_categories, create_repentance.
- [ ] dedications: list, presets, create.
- [ ] mudita: demo_feed, rejoice.
- [ ] stats: today (have), week, month, by_template, by_tradition, vows.
- [ ] export: get export/json, export/markdown (text), import/json — for Settings > Data.
      (Keep the client returning raw dict/text; scenes format.)

### Navigation
- [ ] A shared nav mechanism: extend DashboardScene `_nav(name)` to switch scenes, and give each
      main scene a "back to Today" affordance (button or Esc). Simplest: a `NavBar` helper in ui/
      drawn on each scene, or a `BaseScene` that renders the tab row + routes. Pick one and be
      consistent. Map: Record→RecordPracticeScene, Vows→VowsScene, Repent→RepentanceScene,
      Dedicate→DedicationScene, Mudita→MuditaGardenScene, Stats→StatsScene, Settings→SettingsScene.

### Scenes (spec §15.1)
- [ ] `scenes/record_practice.py` (§5.2): pick a template (from list_templates; show name +
      default points), optional quantity, optional reflection, optional dedication toggle, Save →
      create_entry. Confirm feedback ("Recorded").
- [ ] `scenes/vows.py`: list vows (active first; paused dimmed not red — spec §15.5), button to
      create a vow (name + type positive/negative), tap a vow → VowDetailScene.
- [ ] `scenes/vow_detail.py` (§5.3-§5.6): show vow status + streak; actions by type/status:
      positive→Complete; negative→Record breach (→ RepentanceScene prefilled with linked_vow_id +
      category); Pause with care (reason), Resume, Retire. Show timeline via
      list_entries(vow_id=...). "repair available" not "failure" for repair_in_progress.
- [ ] `scenes/repentance.py` (§5.7): choose category (get_categories), show privacy reminder
      prominently, optional non-secret reflection, repair intention, Save → create_repentance.
      Accept optional prefilled linked_vow_id/category when navigated from a breach.
- [ ] `scenes/dedication.py` (§5.8): choose preset target (presets) or custom, dedication text
      (default from presets), points, Save → create dedication. Optionally link to a recent entry.
- [ ] `scenes/stats.py` (§15): show today/week/month totals + by-template and vows summary.
      Simple bar rows are fine (no chart lib); if you draw charts, read the dataviz skill first.
- [ ] `scenes/settings.py` (§17): tradition switch (set_tradition + app.refresh_palette),
      points mode, negative points toggle, reduced motion, high contrast, sound; Data section:
      export JSON/Markdown (write to data_dir, show path + privacy warning §19.3), import JSON
      (from a path), clear local data (confirm). Persist via put_settings.
- [ ] `scenes/mudita_garden.py` (§5.9, §16): show demo_feed entries as cards with a Rejoice
      button (label = feed verb); on rejoice → mudita rejoice + a small "flower" marker (a drawn
      circle is fine now; real animation Sprint 7).

### Wrap
- [ ] Tests (spec §23.4 — no rendering assertions; test state/routing with a fake/TestClient api):
      record creates an entry; vow_detail chooses correct actions per status; breach routes to
      repentance with prefilled category; settings tradition switch calls set_tradition; mudita
      rejoice calls the endpoint. Construct scenes with an ApiClient over a TestClient app and call
      the handler methods directly (as done in the Sprint 5 smoke test).
- [ ] Keep pygame imports safe headless; run suite with SDL_VIDEODRIVER=dummy.
- [ ] `uv run pytest` + `uv run mypy merit_ledger` green. Launch `uv run merit_ledger` and click
      through every tab. Use /run or a screenshot.
- [ ] Write `sprints/sprint7.md` (Beauty pass: tradition themes polish, card UI, animations —
      lotus appears on record, light flow on dedicate, flower in mudita — typography, reduced-motion).

## Design notes
- Reuse Button + card helpers; add a scrollable list helper if entry/vow lists overflow (simple
  offset + mouse wheel). Keep each scene's data load in on_enter/refresh via api_client.
- Route breach → repentance by constructing RepentanceScene with linked_vow_id + category set; on
  save, also nothing extra needed (backend breach already set repair_in_progress in Sprint 3; the
  repentance links to the vow). Decide if "Complete repair" should call complete_vow — offer it in
  vow_detail for repair_in_progress vows.
- Settings "clear local data": there's no endpoint yet. Either add a backend `POST /settings/clear`
  (scan_all + delete) in this sprint, or defer with a disabled button + TODO. Prefer adding it.

## Definition of done
Every tab opens and works: record a practice, create+complete a positive vow, breach a negative vow
and complete its repair via repentance, dedicate merit, rejoice in the garden, view stats, and
change tradition in settings (theme updates live). Tests + mypy green; app clickable end-to-end.
