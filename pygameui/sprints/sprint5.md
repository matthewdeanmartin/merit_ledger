# Sprint 5 — Pygame Shell (frontend pivot)

Goal: a launchable desktop app. `merit_ledger` (or a new `merit-ledger` script) starts the
FastAPI backend on localhost, waits for /health, then opens a Pygame window with a splash,
onboarding flow, dashboard, and tab navigation. Business logic stays in the backend; the
frontend only renders + calls the HTTP API. Spec Phase 5 (§24), §5.1, §14.1, §15, §22.

## Prereqs / context from Sprints 1-4 (DONE — backend is COMPLETE)

- Backend fully built under `merit_ledger/backend/`, exposed via `create_app()` in
  `backend/main.py` (module-level `app = create_app()` uses the real SQLite db path).
  Endpoints implemented and tested: health, profile, settings, traditions (+ /settings/tradition),
  templates (CRUD), entries (CRUD + filters), vows (CRUD + pause/resume/retire/complete/breach),
  repentance (categories + create), dedications (list/presets/create), mudita (demo-feed/rejoice),
  stats (today/week/month/by-template/by-tradition/vows), export/json, export/markdown, import/json.
- Storage: SQLite single-table, thread-safe. Data dir via `merit_ledger/local/data_dir.py`
  (`data_dir()`, `db_path()`). Config in `local/config.py` (BACKEND_HOST/PORT/URL=127.0.0.1:8765,
  DEFAULT_USER_ID, APP_NAME="MeritLedger").
- Tradition packs (zen/chinese_mahayana/nichiren/pure_land/secular) carry labels, theme name,
  suggested practices, dedication/rejoicing language, point defaults. `GET /traditions/{id}` returns
  the full pack — the frontend uses `labels` + `theme` for tradition-aware UI.
- Tests: `uv run pytest` (88 green). Typecheck `uv run mypy merit_ledger` (strict, clean).
- Existing CLI is a stub: `merit_ledger/cli.py::main` (argparse, --version only). Entry point in
  pyproject `[project.scripts]` is `merit_ledger = "merit_ledger.cli:main"`.
- **User rules:** do NOT branch or commit (user's job). Prefer `uv run`. Turso migration is a
  future candidate (build on SQLite for now).

## Tasks

### Dependency + process management
- [x] `uv add pygame-ce` (spec §21). Confirm import name is `pygame`.
- [x] `merit_ledger/local/server_process.py` — start uvicorn in-process on a background thread
      (simplest + no child-process packaging pain): `uvicorn.Server` with a `Config` pointing at
      `merit_ledger.backend.main:app`, host/port from config. Provide `start()` returning a handle
      and `stop()` (server.should_exit = True). Also a `wait_for_health(url, timeout)` using httpx.
      NOTE: in-process thread means the SQLite repo is shared; that's fine (thread-safe). If you
      prefer a subprocess (`python -m uvicorn ...`), that's OK too but then health-poll over HTTP.
- [x] `merit_ledger/app.py` — orchestrator (spec §22): resolve data dir, ensure db, start backend,
      wait for /health, run the Pygame app, and on exit stop the backend. This is what the CLI calls.
- [x] Wire CLI: add a default action (no subcommand) to `cli.py::main` that calls `app.run()`.
      Keep `--version`. Optionally add `--no-ui`/`--backend-only` for debugging.

### Frontend API client + view models
- [x] `merit_ledger/frontend/api_client.py` — thin httpx client wrapping the endpoints the shell
      needs now: get/put profile, get/put settings, list/get traditions, put tradition, list
      templates, list entries, create entry, stats/today, list vows. (Add more as later scenes need
      them.) Base URL from config. Keep it dumb: methods return parsed JSON/dicts or small
      dataclasses. This is the ONLY thing scenes touch for data.
- [x] `merit_ledger/frontend/theme.py` — map tradition `theme` names (ink_wash, temple_gold,
      dawn_gradient, sunset_gold, calm_blue) → palettes (bg gradient, card, text, accent). Provide a
      neutral default + reduced-motion/high-contrast awareness (from settings). Consider loading the
      dataviz skill palette approach if you build charts later (Sprint 6/7).

### UI primitives (keep minimal this sprint)
- [x] `frontend/ui/` — `text.py` (font loading + cached render), `layout.py` (rects, spacing,
      simple vertical stack), `buttons.py` (Button with hover + click hit-test), `cards.py`
      (rounded card draw). No animations yet (Sprint 7). Bundle at least one open-licensed font in
      `frontend/assets/fonts/` (or fall back to pygame default font) and note the license.

### Scene framework + scenes
- [x] `frontend/pygame_app.py` — Scene base class (handle_event/update/draw), a SceneManager with a
      stack or current-scene + `switch_to`, the main loop (events → update → draw → flip, clock at
      ~60fps), and window setup (title "Merit Ledger", sensible size e.g. 960x640, resizable ok).
- [x] `frontend/scenes/splash.py` — SplashScene: brief calm title card, then auto-advance to
      onboarding (if no profile/tradition chosen) or dashboard.
- [x] `frontend/scenes/onboarding.py` — OnboardingScene (spec §5.1): welcome copy, choose mode
      (Zen/Chinese Mahayana/Nichiren/Pure Land/Secular/Custom), choose point style (points/count/
      reflection), privacy reminder (local-only, don't record secrets), enter profile name → PUT
      profile + settings + tradition → go to dashboard.
- [x] `frontend/scenes/dashboard.py` — DashboardScene (spec §15.4): today's total (stats/today),
      active vows count, a gentle tradition quote/label, quick buttons (Record / Vows / Repent /
      Dedicate / Mudita / Stats / Settings) that navigate. Recent entries list (list_entries).
- [x] Navigation: a simple top or side tab bar (spec §15.3) shared across main scenes, or just
      buttons on the dashboard for now + a "back" affordance. Keep it functional; beauty is Sprint 7.

### Wrap
- [x] Frontend tests (spec §23.4 — keep logic out of frontend): `tests/test_api_client.py`
      (against a TestClient-wrapped app or a live in-process server), `tests/test_theme.py`
      (theme resolves for each tradition), `tests/test_server_process.py` (start → /health → stop).
      Do NOT try to test pygame rendering; test scene *state transitions* where practical by
      constructing scenes with a fake api_client and asserting the next-scene decision.
- [x] Make pygame import lazy/guarded so the backend + tests still run in headless CI without a
      display. (Set `SDL_VIDEODRIVER=dummy` in tests if you must construct a display.)
- [x] `uv run pytest` + `uv run mypy merit_ledger` green. Manually launch (`uv run merit_ledger`)
      and confirm a window opens, onboarding completes, dashboard shows today's total. Use the
      `/run` skill or a screenshot if helpful.
- [x] Write `sprints/sprint6.md` (Pygame core screens: record/vows/vow_detail/repentance/
      dedication/stats/settings/mudita_garden).

## Design notes / decisions
- In-process uvicorn thread is the least painful for packaging (no child python resolution). The
  repo is thread-safe. Downside: a crash in one takes both down — acceptable for a local app.
- Keep ALL business logic server-side. Scenes call api_client only. This preserves the spec's
  "business logic independent of Pygame" acceptance criterion (§25.18).
- Onboarding must actually persist choices (PUT profile/settings/tradition) so relaunch skips it.
  Splash decides: if `GET /profile` returns a name that isn't the default AND a tradition is set,
  skip onboarding. (Or store an explicit `onboarded` flag in settings — cleaner; add it to Settings.)
- Fonts: ship an OFL/Apache-licensed font or use pygame's default; record the license in assets.

## Definition of done
`uv run merit_ledger` launches: backend comes up, /health passes, a Pygame window opens, first-run
onboarding lets you pick a tradition + point style + name, and the dashboard shows today's practice
total and quick-nav buttons. Backend shuts down cleanly on window close. Tests + mypy green.
