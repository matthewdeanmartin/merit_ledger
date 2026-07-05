# Sprint 7 — Beauty Pass + Deferred Sprint 6 Polish

Goal: make the app genuinely pleasant to look at and complete the interactions Sprint 6
simplified. Spec Phase 7 (§24), pretty details §15.5, secular/appearance settings §17.4.
"Once that works, make it beautiful." — spec §32.

## Prereqs / context from Sprints 1-6 (DONE)

- Full stack works. Backend complete. Frontend: 8 scenes + splash/onboarding on a shared
  `MainScene` (nav bar in `frontend/scenes/nav.py`), all driven by `frontend/api_client.py`.
  UI primitives in `frontend/ui/` (text, cards[gradient+rounded card], buttons). Palettes in
  `frontend/theme.py` (5 traditions + high_contrast()). Launch `uv run merit_ledger`.
- Test headless: `SDL_VIDEODRIVER=dummy uv run pytest`. 115 tests, mypy + ruff green.
- Rules: no branch/commit (user's job); `uv run`; logic stays server-side.
- **Read the `dataviz` skill BEFORE building the Stats charts.**

## Tasks

### Deferred from Sprint 6 (do these first — they unblock real usage)
- [ ] `frontend/ui/text_input.py` — a simple focusable text field widget (caret, backspace,
      max length, click-to-focus). Reuse the KEYDOWN handling pattern from onboarding.py.
- [ ] Record scene: add optional quantity field + reflection field (text_input) + a "dedicate
      after saving" toggle that, on save, routes to DedicationScene with the new entry_id.
- [ ] Repentance scene: add optional non-secret reflection + repair-intention text fields
      (keep the privacy reminder prominent; still no who/what/where fields).
- [ ] Dedication scene: custom target field + editable dedication text + points field; allow
      linking to the most recent entry (source_entry_id).
- [ ] Settings scene: checkbox/toggle widget for points_mode (cycle), negative_points_enabled,
      reduced_motion, high_contrast, sound_enabled → put_settings. Data section: export JSON +
      Markdown to the data dir (show the path + the §19.3 privacy warning), and import from a
      chosen JSON file. (api_client has the export/import methods.)
- [ ] `frontend/ui/scroll.py` or a list helper: mouse-wheel scroll for entries/vows/templates
      so nothing is capped at [:6]/[:7]/[:8]. Wire into dashboard, vows, record, mudita.

### Beauty pass (spec §15.2, §15.5)
- [ ] `frontend/ui/animation.py` — a tiny tween/easing helper (time-based, respects
      settings.reduced_motion → instant). Keep it dependency-free.
- [ ] Pretty details (spec §15.5), all gated on reduced_motion:
      - Record: a lotus/card gently fades+rises into the ledger list on save.
      - Dedicate: points "flow" into a soft light (a few eased particles toward a target).
      - Mudita: a real flower bloom (grow/fade) instead of the current static circle.
      - Repentance complete: a calm "Returned to practice." confirmation transition.
      - Paused vows dimmed (already), repair shows "repair available" (already) — refine styling.
- [ ] Typography: bundle an OFL font (e.g. an open sans/serif) in `frontend/assets/fonts/` and
      load it in ui/text.py (fallback to default). Record the license. Set up a small type scale
      (title/heading/body/caption sizes) instead of ad-hoc sizes.
- [ ] Theme polish: refine the 5 palettes for real contrast + calm; add soft card shadows,
      consistent spacing/margins, and a subtle header per scene. Apply high_contrast() when
      settings.high_contrast is on; scale fonts by settings.font_scale.
- [ ] Onboarding + splash: gentle fade transitions; make the splash art tasteful (enso-like ring
      for Zen etc. — draw it, avoid copyrighted assets).
- [ ] Stats: turn the bar rows into a proper small chart (READ dataviz skill first; keep it
      accessible in light + dark palettes).

### Wrap
- [ ] Keep everything testable headless; animations must no-op cleanly when reduced_motion or when
      there's no display. Add tests for text_input (key handling), the toggle widget, and that
      reduced_motion short-circuits animation.
- [ ] `uv run pytest` + `uv run mypy merit_ledger` + `uv run ruff check merit_ledger` green.
- [ ] Launch and screenshot each tradition (spec acceptance §25.20 "visually pleasant enough to
      screenshot proudly"). Use the /run skill.
- [ ] Write `sprints/sprint8.md` (Packaging: README with screenshots, CLI polish, data-dir notes,
      installer/build notes, GitHub release artifact — spec Phase 8).

## Design notes
- Animations are the ONLY place motion lives; everything must degrade to instant. Never block the
  main loop — tween via dt in update().
- Don't regress the "no casino gamification" principle (spec §2.5, §28): soft, slow, optional.
- If bundling a font balloons the wheel, it's fine to keep the default font and just fix the type
  scale — legibility over novelty.

## Definition of done
The app looks like "a small digital shrine, not an admin dashboard" (spec §2.5): calm palettes per
tradition, readable type, gentle motion (honoring reduced_motion), and the Sprint 6 interactions are
now complete (text entry, settings toggles, export/import, scrolling). Screenshots are share-worthy.
Tests + mypy + ruff green.
