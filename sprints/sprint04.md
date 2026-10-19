# Sprint 4 — Profiles, feed and stats
**State:** done 2026-10-18 (desktop and board), except the page-load target, which is blocked on Wi-Fi signal (see the exit criteria)
**Depends on:** 3   **Parallel with:** 6, 7

## Goal
Give everyone a public profile page on the board, plus a household feed and Strava-style stats:
streaks, weekly minutes and a heatmap.

## Read first
- `../spec/v2_decisions.md` § People and privacy (public by default; Mastodon mirrors the board)

## Questions for the owner (answered 2026-10-18)
1. **Signed-out visitors and guests** see everything that isn't private, but never demerit or repentance
   details. This is a server rule (`ledger::sensitive`): for anyone except the person and admins, a
   demerit or repentance entry is at most its generic line ("A moment of reflection", "A repentance
   practice"), whatever its visibility. Private entries stay hidden.
2. **A streak counts any merit,** day after day. A day with no merit ends it; a demerit neither helps nor breaks it.
   ("If someone can log on 7 days a week, wow.")
3. **Demerits in stats:** only the person and admins see their 30-day demerit count, on their own profile.
   Public stats are merit only.

## Design decisions made in this sprint
- **Stats are computed from the entries the board holds, as the viewer may see them.** Someone else's
  private entries don't count. A generic entry counts as a day of practice, but not its minutes (its
  category shows as "other").
- **Monthly rollups were moved to Sprint 5.** The rollup is a storage question: where history lives
  once it's on the SD card. Until then the stats cover the snapshot window, and the profile says how far back
  the board remembers (`history_from`); the heatmap outlines older days as unknown, not empty.
- **Generic entries no longer reveal their category** (a demerit's "speech" said too much). A leak
  fixed in passing.
- **Rejoicing (mudita):** 🙏 on someone else's public merit. It adds the rejoicer to the entry once, and
  records a "Rejoiced" merit of their own ("In Kid's meditation"). You can't rejoice in your own entries or in
  generic or demerit ones.

## Deliverables
- [x] `stats.rs`:
  - Current streak (today, or yesterday while today is still empty) and longest streak.
  - Merit minutes over 7 days, and totals by category over 7 and 30 days.
  - A 182-day heatmap at levels 0–4.
  - The 10 most recent entries, and demerits over 30 days (own view only).
- [x] `GET /api/v1/people/{id}/profile` and `GET /api/v1/feed?offset&limit`, both public.
  `POST /api/v1/entries/{id}/rejoice` needs sign-in, and returns the updated entry.
- [x] UI:
  - **The house** (`/house`): everyone's tiles, linking to profiles, and the household feed with paging and 🙏.
  - **Profiles** (`/p/:id`): streak, longest, this week; an inline-SVG 26-week heatmap; category bars; recent entries.
  - Both pages work signed out. Signed out, the nav shows "House · Sign in".
  - Signed-in nav: Today · House · Practices · *your name* (your profile) · ⚙︎ (settings) · Household (admins).

## Exit criteria
- [x] Visibility tests: a private entry never appears in someone else's profile, feed or stats. A
  generic entry, or a public demerit, shows only its label to others. The person sees their own details.
- [x] Stats tests: streaks (including "today isn't over"), levels, merit-only counting, generic counted as a day
  without minutes, feed order and paging, rejoice once and only in others' public merit. 7 unit tests and 2 API tests.
- [~] ~~The rollups equal a recomputation from raw entries~~ Moved to Sprint 5 with the rollups themselves.
- [ ] **Profile page under 1 s from the board over HTTP/2: not met.** Cold loads took a median of
  3.0 s; warm loads (cached assets) 1.2 s, about 1 s of which is a TLS handshake a real browser would resume.
  The cause is the radio link: the board reads -77 dBm, against -56 dBm for gamematcher on the same
  AP. A bigger TCP buffer didn't help. See [pageload.md](../spec/measurements/pageload.md).
  **Owner:** try moving the board, then re-run `uv run tools/pageload.py <ip> meritledger_rs/certs/household-ca.crt`.

## Out of scope
Mastodon mirroring (Sprint 11), vows on the profile (Sprint 9).

## Handoff
- **History for stats is the snapshot window** (60 days) until Sprint 5. Sprint 5 owns:
  - per-day rollups (person, day, merit entries, minutes) that survive without a card;
  - feeding `stats.rs` from them, so streaks and heatmaps reach beyond the window.
  The `Day` type and `current_streak`, `longest_streak` and `level` are the interface to keep.
- `entry_view` is the single place that decides what a viewer sees. Every new view (vows, the board's
  screen, Mastodon) should go through it.
- The profile's `heatmap` is 182 integers, about 400 bytes of JSON. Fine as it is; pack it only if a bigger view needs to.
- `tools/pageload.py` measures cold and warm loads by IP. Re-run it after moving the board.
