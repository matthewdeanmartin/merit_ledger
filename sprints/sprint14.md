# Sprint 14 — Hardening and release
**State:** not started
**Depends on:** all   **Parallel with:** —

## Goal
Make it something the household relies on, and that someone else could run on their own board.

## Read first
- Every sprint's Handoff section, for the deferred items
- `../microcontroller/miniframework/spec/OTA.md`
- `../microcontroller/miniframework/docs/RESOURCE_PROTECTION.md`

## Questions for the owner
1. Is OTA from the web UI wanted, or are USB `make update` runs fine?
2. Browser push: is it worth doing now ("1x in a kalpa")? Or should the paid notification service be evaluated instead?
3. Should there be a public release for other households (docs, a `wifi-setup` provisioning build), or should it stay household-only?

## Deliverables
- [ ] A load walk on the board: 4 phones doing a morning sit, follow-along and logging at once, with the
  screen on. Record the heap and latency.
- [ ] Flash wear: commit count per day in normal use, projected lifetime, and more slots if needed.
- [ ] Rate limits (framework `rate-limit`) on sign-in, the touch menu proxy routes and search.
- [ ] A clock-not-synced audit: every date-dependent feature behaves sensibly before SNTP.
- [ ] An accessibility pass on the UI (keyboard, screen reader labels, contrast in both themes).
- [ ] Optional, per the answers above: OTA, browser push, a `wifi-setup` build for other households.
- [ ] A CI story for v2: vendor or publish miniframework, or a self-hosted runner, so GitHub can run
  `make check`.
- [ ] README, DEPLOY and a user guide for the household (one page, printable via the Sprint 8 print CSS).

## Exit criteria
- [ ] One week of real household use with no data loss and no unexplained reboots (`/api/v1/log`).
- [ ] Restore from an SD backup onto a freshly repaved board, end to end.

## Handoff
_(fill in)_
