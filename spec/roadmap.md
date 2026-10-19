# Roadmap

Merit Ledger v2 is a household Buddhist practice ledger served from an ESP32-S3 touch board.
It covers Strava-style practice logging, phone meditation timers with bells, computed lunar
calendars, printable and follow-along practices, vows as a blog, Open Library reading lists,
Mastodon and email notifications, and a touch screen in the house.

Decisions: [v2_decisions.md](v2_decisions.md). Data model: [domain.md](domain.md). Sprint
files: [../sprints/](../sprints/README.md).

## Milestones

| Milestone | Sprints | What the household can do |
|---|---|---|
| **M0 Skeleton** | 0 | Build and run the empty app on desktop; build firmware |
| **M1 On the board** | 1 | The board serves the app over HTTPS with an SD card, and its limits are measured |
| **M2 Daily use** | 2–5 | Sign in, tap to log practice, see profiles and streaks, never lose history |
| **M3 Practice tools** | 6–8 | Sit with phone bells, follow the lunar calendar, print or follow along with practices |
| **M4 Sangha** | 9–12 | Vows blog, reminders and invitations, Mastodon publishing, reading lists |
| **M5 The house** | 13–14 | The touch screen in the hallway, hardening, a release |

## Sprints

| # | Sprint | Depends on | Parallel with | State |
|---|---|---|---|---|
| 0 | [Restructure and skeleton](../sprints/sprint00.md) | — | — | **done** 2026-10-18 |
| 1 | [Board bring-up and gate](../sprints/sprint01.md) | 0 | 2 | **done** 2026-10-18 |
| 2 | [Storage and accounts](../sprints/sprint02.md) | 0 | 1 | **done** 2026-10-18 |
| 3 | [Practice catalog and one-tap logging](../sprints/sprint03.md) | 2 | 7 | **done** 2026-10-18 |
| 4 | [Profiles, feed and stats](../sprints/sprint04.md) | 3 | 6, 7 | **done** 2026-10-18 (page-load target blocked on Wi-Fi signal) |
| 5 | [History on SD, backups and export](../sprints/sprint05.md) | 1, 3 | 6, 7 | **done** 2026-10-18 (long-history speed: follow-up in its handoff) |
| 6 | [Meditation timer and bells](../sprints/sprint06.md) | 3 | 4, 5, 7 | **done** 2026-10-19 (phone trials: owner checklist) |
| 7 | [Meritledger lunar calendars](../sprints/sprint07.md) | 0 (domain), 2 (settings) | 3–6 | **done** 2026-10-19 |
| 8 | [Texts and build-a-practice](../sprints/sprint08.md) | 3, 6 | 9 | **done** 2026-10-19 (owner reviews in place) |
| R1 | [Detour: precept texts, research and sources](../sprints/detour-precepts.md) | — | 8 | research done 2026-10-19; owner review of renderings pending |
| 9 | [Vows, the vow blog and precepts](../sprints/sprint09.md) | 4, R1 | 8, 12 | **done** 2026-10-19 (owner reviews in place) |
| 10 | [Notifications, reminders and invitations](../sprints/sprint10.md) | 7 | 12 | |
| 11 | [Mastodon sign-in and publishing](../sprints/sprint11.md) | 4, 10 | 12 | |
| 12 | [Books via Open Library](../sprints/sprint12.md) | 3 | 9–11 | |
| 13 | [The board's screen and touch menu](../sprints/sprint13.md) | 1, 3, 7, 10 | — | |
| 14 | [Hardening and release](../sprints/sprint14.md) | all | — | |

### Parallel tracks

Three people can work at once without stepping on each other:

- **Board track:** 1 → 13 (display and touch drivers can start right after 1, against a stub deck).
- **Domain track:** 2 → 3 → 4 → 9 → 11.
- **Pure-logic track:** 7 (astronomy and calendars are plain Rust with no I/O) → 10, plus 6 (UI-only timer).

Merge points are the `App` route table (`meritledger_rs/src/api.rs`) and `messages.rs`. Keep route
blocks grouped by sprint, and append message fields with new numbers only.

## Risks to watch

| Risk | Where it bites | Mitigation |
|---|---|---|
| JD9853 panel and AXS5106L touch have no Rust driver | 13 | Port the init sequence from Waveshare's demo or the CircuitPython port; draw in strips like gamematcher |
| Locked phone stops the timer | 6 | Wake Lock over HTTPS. Without it, say "keep this screen on". Test iOS Safari first |
| SD card corruption or a missing card | 5 | Append-only logs with CRC per record; snapshot rollups; the board runs fine without a card |
| Lunar maths disagreeing with published calendars | 7 | Intentional: they're "meritledger lunar calendars". Golden-file tests pin our own output |
| Copyright on liturgy | 8 | LLM-drafted originals, reviewed, with recorded provenance |
| Cross-repo path dependency on `../microcontroller` | CI | GitHub CI can't see it; v2 checks run locally (`make check`) until the framework is published or vendored |
| Flash wear from frequent commits | 3, 14 | gamematcher's write budget: commit people's changes at once, debounce the rest, report the commit count |
