# Sprint 3 — Practice catalog and one-tap logging
**State:** done 2026-10-18 (desktop and board). The starter catalog is shipped for the owner to review in place.
**Depends on:** 2   **Parallel with:** 7

## Goal
Build the core loop, like Strava for practice. Open the site, tap a practice, and it's recorded as
done today. It works with canned merit and demerit kinds plus your own, and you can record for
someone else.

## Read first
- `../spec/domain.md` § Practice
- `../spec/v2_decisions.md` § Effortless merit tracking, § People and privacy

## Already decided
- One tap = one entry, with a `once` unit or a default amount. It can be edited afterwards (minutes,
  count, note, visibility, day).
- Demerit and repentance kinds default to `private`. `generic` shows only the category publicly.
- Start, interrupted and ended events are noise and are never entries.
- Practices from anywhere count: sutra copying, thangka painting, and reading fiction and illustrating it.
- Nichiren chanting (daimoku) is in. Pure Land kinds aren't shipped.

## Questions for the owner (answered or decided 2026-10-18)
1. **Starter catalog:** the owner reviews it **in place**. It shipped as below, in
   `meritledger_rs/catalog/practices.json`.
2. **Scoring:** plain counts and minutes, Strava-like. There are no point weights; Sprint 4's stats can add
   any if the owner wants them.
3. **Twice a day:** yes. Each tap is a new entry; a tile shows ✓ and today's total after the first.

The shipped catalog (21 kinds):
- **Merit:**
  - Meditation (20 min), walking meditation (10), chanting (15)
  - Daimoku (108), prostrations (108), recited a text
  - Reading Dharma (30 min), reading fiction mindfully (30), sutra copying (20), Dharma art (30)
  - Kept the precepts, vegetarian today, act of service, dedicated merit, rejoiced
- **Repentance:** repentance practice. Defaults to `generic`.
- **Demerit:** broke a precept, harsh speech, lost my temper, intoxicants, careless harm. All `private`.
- **Default tiles:** meditation, chanting, prostrations, reading, precepts, vegetarian.

## Carried over from Sprint 2 (settled)
- **Removing a person:** the admin must say `?entries=keep` or `?entries=delete`.
  - Kept entries show their person as "a former member".
  - Deleting also removes that person's personal kinds.
- **Recording from the board itself:** the touch menu runs inside the firmware, so Sprint 13 calls
  `App::record_from_board(person, kind)` directly. There is no unauthenticated HTTP route. Such entries
  have `recorded_by: 0` and `source: "board"`, and show "recorded by the board".

## Deliverables
- [x] Built-in catalog as JSON compiled into the firmware (`catalog.rs`), decoded once with the
  framework's JSON decoder. Keys are stable, and a test checks the catalog's shape and defaults.
- [x] Custom kinds (`c<n>` keys):
  - Household-wide or personal (guests can only add personal ones). Bound: 256.
  - Rename, change the default amount or visibility, archive and restore.
  - Built-ins can't be changed, only unpinned.
- [x] Entries (`ledger.rs`). Routes:
  - `POST /api/v1/entries` (person 0 = me, defaults from the kind, today).
  - `PATCH|DELETE /api/v1/entries/{id}`, with a `rev` check (409 `stale`). Each returns the
    refreshed Today view.
  - `GET /api/v1/today?person=`, which returns tiles with today's counts and totals, plus
    today's entries.
  - `GET /api/v1/people/{id}/entries?from&to&limit`. This is public: private entries are hidden,
    and generic ones are reduced.
  - `GET|POST /api/v1/kinds`, `PATCH /api/v1/kinds/{key}`, `PUT /api/v1/me/pins`.
- [x] Who may do what:
  - Anyone in the household may record for anyone. Guests record only for themselves.
  - Editing an entry is for its person, whoever recorded it, or an admin.
- [x] Checks:
  - Minutes 1–1440, counts 1–1,000,000; `once` ignores an amount.
  - Notes up to 500 characters.
  - Dates no later than today and no more than 59 days back (the snapshot window).
  - No clock and no date → 503 `clock_not_set`.
- [x] **Household time zone** (`tz.rs`): a pure-Rust POSIX TZ evaluator, the same on desktop, board and
  tests. It's set on the Household page (admin settings) and defaults to `EST5EDT,M3.2.0,M11.1.0`.
  Sprint 7's calendars should use `Tz::local_date` and `tz::Date`.
- [x] Write budget: every change is one commit, as in Sprint 2. Session touches are still batched each minute.
- [x] UI:
  - **Today** (the home page): big tiles that turn green when done, with a "For: me / Kid" picker.
  - A 6-second "Logged … Undo · Edit" bar after each tap.
  - Today's list, editing inline (amount, day, note, who sees it, delete).
  - **Practices**: choose tiles and their order, browse merit, demerit and archived kinds, add your own.
  - **Household**: the time zone, and the keep-or-delete choice on removal.

## Exit criteria
- [x] Tests: 43 unit and 8 API tests for this sprint, alongside Sprint 2's. They cover:
  - 23:59 vs 00:01 in New York.
  - Sydney's daylight time spanning New Year.
  - Visibility defaults.
  - Recording for someone else, and from the board.
  - A stale `rev` → 409.
  - Bounds.
  - Custom and personal kind scoping.
  - Pins.
  - Keep vs delete on removal.
- [x] **Two taps or fewer** from a cold phone: open the site (signed in) and tap the tile. That's one tap.
- [x] **Snapshot size with a full window:** 4,096 entries across 16 people is **258,645 bytes**, about 63 bytes
  each. It fits the 1 MiB `mlstate` partition with both slots, but see the handoff.

## Out of scope
Timers (Sprint 6), history beyond the snapshot window (Sprint 5), stats and feeds (Sprint 4), publishing.

## Handoff
- **Commit cost grows with the window.** Every tap rewrites the whole snapshot: about 1 KB today,
  and up to 258 KB with a full window. Sprint 5 should make the snapshot window short and put the
  rest on the SD card. Two options: entries from the last 14 days in NVS and older ones in month
  logs, or every entry appended to the SD log with only a recent index in NVS. Measure the
  commit time on the board at about 50 KB and 250 KB before choosing.
- **`created` is 0** for entries recorded before the clock was set (with an explicit date).
- `GET /api/v1/people/{id}/entries` is public by design ("data is public by default"). Sprint 4's
  profile question 1 (signed-out visitors) applies to it too.
- The UI is Today-centred. Sprint 4 adds the profile, feed and heatmap on top of the same
  `entries` route and the monthly rollups.
- The board runs this build (2026-10-18). The owner's account carried over.
