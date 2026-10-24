# Sprint 14 — Hardening and release
**State:** done 2026-10-19 except the board runs and the week of use (owner)
**Depends on:** all   **Parallel with:** —

## Goal
Make it something the household relies on, and that someone else could run on their own board.

## Read first
- Every sprint's Handoff section, for the deferred items
- `../microcontroller/miniframework/spec/OTA.md`
- `../microcontroller/miniframework/docs/RESOURCE_PROTECTION.md`

## Questions for the owner
Not asked yet (the owner was away). The defaults this sprint assumes, for the owner to confirm:
- **No OTA:** USB `make update` stays the way to update.
- **No browser push:** the bell polls, and calls go out over Mastodon DMs and email.
- **Household-only:** no `wifi-setup` build yet. The docs are written so another household could
  follow them.

The original questions:

1. Is OTA from the web UI wanted, or are USB `make update` runs fine?
2. Browser push: is it worth doing now ("1x in a kalpa")? Or should the paid notification service be evaluated instead?
3. Should there be a public release for other households (docs, a `wifi-setup` provisioning build), or should it stay household-only?

## Deliverables
- [ ] A load walk on the board: 4 phones doing a morning sit, follow-along and logging at once, with the
  screen on. Record the heap and latency.
- [x] Flash wear: commit count per day in normal use, projected lifetime, and more slots if needed.
- [x] Rate limits (framework `rate-limit`) on sign-in, the touch menu proxy routes and search.
- [x] A clock-not-synced audit: every date-dependent feature behaves sensibly before SNTP.
- [x] An accessibility pass on the UI (keyboard, screen reader labels, contrast in both themes).
- [ ] Optional, per the answers above: OTA, browser push, a `wifi-setup` build for other households.
- [x] A CI story for v2: vendor or publish miniframework, or a self-hosted runner, so GitHub can run
  `make check`.
- [x] README, DEPLOY and a user guide for the household (one page, printable via the Sprint 8 print CSS).

## What was built
- **Property tests** (the owner asked for "the ones that work like python hypothesis"):
  `tests/props.rs` (proptest) and `src/app/props.test.ts` (fast-check), run by `make test`, with
  `make fuzz` for 20,000 cases per rule. They found five real bugs on their first runs:
  - Time zones up to 27 h were accepted (`"AAA27"`). Offsets are now limited to ±24 h.
  - `Face::wrap` panicked when asked for 0 lines.
  - An event ending at `u64::MAX` overflowed (debug builds panicked; the board's release build
    would have wrapped). Times past year 9999 are now refused, and the hour padding saturates.
  - `parseSearch` threw on a malformed Open Library reply (`{"docs":0}`). It now trusts nothing
    about the shape.
  - A desktop-only test found the board's undo refused by `can_edit`, which correctly bars viewer
    0. The undo now checks its own rule.
- **Rate limits:** miniframework's `rate-limit` `Limiter`, one bucket per route class (`METER`,
  `metered` in `api.rs`): sign-up and setup, OAuth start, guest RSVP, `frame.bmp` and export.
  The answer is 429 or 503 "busy" with `Retry-After`. PIN sign-in keeps its own lockout.
- **Before the clock is set:** calls, events, RSVPs and reminders now refuse with 503
  `clock_not_set`, like entries and backups already did, instead of stamping 1970. So does the
  board's Call to practice. Reads, housekeeping (skipped until synced) and the screen (it shows
  "setting the clock") were already fine.
- **Accessibility:** `tools/a11y_walk.py` runs axe-core (WCAG 2.1 A and AA) on 19 pages in light
  and dark, in Edge. It found and fixed:
  - The light accent was 4.2:1. It's now `#935814`, at 5.0:1 on the page and 5.8:1 under white
    text.
  - `--ok` green is now `#466d33`.
  - The danger button was red on the accent background (1.4:1 and 1.04:1). It now has a
    transparent background.
  - The practice tiles had an `aria-label` that didn't contain their visible text. They're now
    named by that text.
  Both themes are now clean. Keyboard focus rings were already there (`:focus-visible`).
- **Load walk:** `tools/load_walk.py`, four phones over HTTP/2. On the desktop for 45 s: no
  errors, and p95 latency under 50 ms for everything. The board run needs the owner (see Handoff).
- **CI:** `.github/workflows/v2.yml` checks out `microcontroller` (public) next to this repo, as on
  the owner's PC, and runs `make check`. No vendoring or publishing is needed. It goes green once
  the local miniframework patches are committed and pushed.
- **Docs:** README (testing, tools, the guide), DEPLOY (after-an-update checks, rate limits, the
  clock), and `/guide`, a one-page household guide in the app that prints on one sheet.
- **`unsafe` audit (owner request):** the library and the desktop binary are now
  `#![forbid(unsafe_code)]`. The firmware's `unsafe` is only FFI: the screen driver, wrapped in
  `Panel`, and `esp_vfs_fat_info`, which has no safe wrapper in esp-idf-svc. In the uncommitted
  miniframework diff (not this sprint's work), `unsafe { sys::esp_restart() }` in the ops restart
  path could be esp-idf-hal's safe `reset::restart()`. That's for the owner to decide.

## Flash wear
Snapshots go to the 1 MiB `mlstate` NVS partition (256 sectors of 4 KiB). NVS writes round-robin
across its pages, so wear spreads over the whole partition. At about 100,000 erase cycles per
sector, that is roughly 100 GB of writes in its life.
- **A typical household:** 4 people, about 60 commits a day (every change commits once; the
  minute housekeeping only flushes when something changed). The snapshot is about 50 KB with 60
  days of entries, notifications and the rest, so about 3 MB a day. That's about **90 years**.
- **The worst case:** a full snapshot (4,096 entries, about 250 KB) committed 1,000 times a day is
  250 MB a day, about **14 months**. That isn't plausible for one household, but it's the bound.
- **To measure:** `/api/v1/admin/storage` shows `commits` and `snapshot_bytes`. Read them a day
  apart. The desktop load walk (4 phones logging every 20 s) made 36 commits in 45 s, which is
  heavier than a real morning.

No more slots are needed. If the measured rate is ever more than 10x the estimate, debounce the
non-urgent commits (notifications read, sessions), as gamematcher does.

## Exit criteria
- [ ] A week of real household use with no data loss and no unexplained reboots. (Owner.)
- [ ] Restore from an SD backup onto a freshly repaved board. This needs the owner's explicit
  go-ahead to erase.

## Handoff
- **The owner, with the board plugged in:**
  1. `make update PORT=COM5` (the MAC is checked first).
  2. The Sprint 13 touch checks.
  3. `tools/load_walk.py` against the board, for 5 minutes.
  4. Note the heap low point and the redraw ms in DEPLOY.md.
- **Commit the regressions file** (`tests/props.proptest-regressions`): its seeds are the bugs
  found, retried first on every run.
- **Answer the three questions above.** OTA, push and a `wifi-setup` build are deferred until
  then.

### Household feedback, 2026-10-21

Local changes: role/person selects bind each rendered option; Today adopts the returned person
and prevents overlapping person changes/claims. New events exclude guest accounts from default
invitees. Practice documents embed the existing timer/counter; custom practice tiles can be edited
and have a searchable native emoji palette. Appearance follows Nanacoin's browser-stored palettes.
Book covers distinguish loading, missing metadata and load failure. Calendar has a download/import
option: the live Chinese feed contains Guanyin's 2026-10-28 observance, so no lunar calculation change
was needed for the Chromebook report.

Calls to practice now prepare email as well as DMs, respecting channel opt-outs. Their response
reports missing channels/recipient addresses and points to delivery status. Real delivery remains
an owner check; no external messages were sent. The caller remains excluded. Arbitrary public
Mastodon hostnames can be entered for sign-in/linking; miniframework's bounded OAuth flights now
accept an app-validated provider without altering concurrent flights or household CA trust.

Validation: `make check`; browser regression against isolated localhost data
(`tools/household_regression.cjs`, uses the workspace's Mawkingbird Playwright installation).
No firmware flash performed. The broader miniframework gate is blocked by the wheels-only
requirement: esptool 4.12.0 has no binary distribution; focused OAuth tests and clippy pass.
