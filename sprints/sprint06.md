# Sprint 6 — Meditation timer and bells
**State:** done 2026-10-19 (desktop and board), except the phone trials, which only the owner can run ([checklist](../spec/measurements/timer.md))
**Depends on:** 3   **Parallel with:** 4, 5, 7

## Goal
A meditation timer on the phone or laptop: an opening bell, interval bells, a closing bell with
three strikes, sitting and walking periods, and a counting (mala) mode. A finished session logs an
entry automatically.

## Already decided
- The timer runs in the browser; the board stores presets and receives the finished entries. Start
  and interrupt are noise and are never posted.
- HTTPS is required for Wake Lock. Over plain HTTP the page says once that it can't keep the
  screen on, and links to `/trust`.
- No metronome.

## Questions for the owner (proposals adopted 2026-10-19; say so to change them)
1. **Bells are synthesized** in Web Audio: no files and no licences. There are three timbres: singing bowl,
   small bell, wooden block, with a "Hear it" button.
2. **An interrupted sit logs the whole minutes actually sat,** if at least one.
3. **Counting mode counts each tap**, with a short buzz per bead and a longer one at each quarter of 108
   (where the phone supports vibration).

## What was built
- **Presets** (`timers.rs`, `catalog/timers.json`):
  - Built-ins:
    - 10, 20 and 30-minute sits (the 30 has a bell every 10 minutes)
    - sit/walk/sit 25 + 5 + 25
    - 108 daimoku (mala)
    - 108 prostrations
  - Custom presets are household or personal (guests: personal), with up to 12 sit or walk parts,
    interval bells, a sound, and the practice they log as. The kind's unit must match: minutes
    for timed, a count for counted.
  - Routes: `GET|POST /api/v1/timers`, `PUT /api/v1/timers/{key}` (replace, or archive).
    Built-ins can't be changed. Bound: 64.
- **Engine** (`timer.ts`, pure):
  - Plan: parts and bells. Opening strikes, one strike at each change of part, interval bells
    inside sitting parts (none in the last 30 s of a part), closing strikes.
  - Position: the current part and time left. Bells still to ring after a resume. Credit: whole minutes
    of sitting as the preset's kind, and of walking as `walking`.
  - A pausable `Clock`.
  - **Deviation from this file:** wall-clock deltas (`Date.now()`) rather than `performance.now()`,
    because some phones stop the monotonic clock while asleep and a locked phone must still end the
    sit on time. The clock is injected, so tests control it.
- **Bells** (`bell.ts`): additive sine partials at inharmonic ratios, each with its own exponential decay.
  They're scheduled ahead on the `AudioContext` clock, so a throttled tab still rings on time; pause
  and End cancel what's scheduled.
- **Screen:** Wake Lock while a session runs, re-acquired when the page becomes visible again. The
  run screen says "screen may sleep" when it couldn't get one.
- **Logging:**
  - On finish, each credit is saved in `localStorage` first (`session-queue.ts`), then sent with
    `source: timer`.
  - Each carries a `client_id`. The board stores it, and a retry with the same id records nothing new,
    so a lost reply never double-logs.
  - Network errors and 5xx keep entries for later; Today and the timer page send them on load.
  - A permanent 4xx drops the entry.
- **UI** (`/timer`, also "🔔 Timer" on Today):
  - Preset list with Start.
  - Run screen: part, big countdown, progress, total left, Pause/Resume, End.
  - Full-screen counting circle with −1 and Finish.
  - A summary of what was logged.
  - An editor for your own timers.

## Exit criteria
- [x] Vitest (17 tests for this sprint):
  - Plan and bell layout, interval margin, position and done.
  - Bells after a resume, credit for full and interrupted sessions and for counts.
  - Pause and resume, and a locked phone with no ticks at all.
  - The offline queue: reload, network failure, permanent refusal, 5xx, de-duplication.
  - Bell spacing.
- [x] Rust: preset validation and scoping (3 unit tests), plus an API test for presets and a retried
  session (one entry, not two).
- [ ] **A 20-minute sit on iOS Safari and Android Chrome over HTTPS:** owner, with
  [the checklist](../spec/measurements/timer.md).
- [ ] **Locked mid-sit, unlocked after the end:** owner, same checklist.

## Handoff
- **Found this sprint:** the Sprint 5 journal "peek" (which reads a record's id, person, date and
  deletion flag without a full decode) assumed the deletion flag was the record's last field.
  Adding `client_id` broke that, and the guard test caught it at once. `peek` now walks the fields
  generically, so new fields can't break it again.
- **Vitest** now maps `miniframework-ng` (`vitest.config.mts`), so pure modules may import its types
  and `ApiError`.
- A preset's `logs_as` must match the kind's unit. If someone later archives the kind a preset logs as,
  editing the preset asks for another, while starting it still works and logs to the archived kind.
- Sprint 8's practice documents can start a preset: `TimerPage.start(preset)` is the entry point.
