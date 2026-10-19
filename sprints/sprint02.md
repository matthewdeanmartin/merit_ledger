# Sprint 2 — Storage and accounts
**State:** done 2026-10-18 (desktop and board)
**Depends on:** 0   **Parallel with:** 1

## Goal
Add a persistent household state and the people in it. Use the snapshot store, accounts for the
owner, the kids and guests, PIN sign-in, and sessions that survive reboots.

## Read first
- `../spec/domain.md` § People, § Storage map
- `../microcontroller/gamematcher/spec/02-accounts-auth.md`, `05-storage-limits.md`
- `../microcontroller/miniframework/crates/miniframework/src/session.rs`

## Already decided
- The household trusts itself. Signing in is for "whose phone is this", not for security between
  family members.
- Roles are `admin`, `member` and `guest`. The first person on a fresh board becomes admin.
- Sessions are HttpOnly cookies (`ml_session`) that survive reboots.

## Questions for the owner (answered 2026-10-18)
1. **Kids get their own accounts.** It's the owner's house, so no GDPR or identity laws apply.
   - A parent adds each kid from the Household page, with a PIN.
   - A person without a PIN is still possible: they can't sign in, but anyone can record for them.
2. **Guests:**
   - Sign-up, while it's open, makes a **guest** with a PIN.
   - An admin can promote a guest to member.
   - An admin can switch sign-up off.
3. **What happens to a removed person's history** is decided in Sprint 3, which is when there is
   history. Today, removing someone deletes them and their sessions.

## Design choices made in this sprint
- **Sign-in is "tap your name, then enter your PIN"**, not gamematcher's email login, because kids
  shouldn't need an email address.
  - Names are therefore unique, ignoring case and extra spaces.
  - `GET /api/v1/people` is public. It lists people who aren't disabled, with `can_sign_in`.
  - Email is optional, kept for invitations later.
- **Picking a person id that doesn't exist costs the same as a wrong PIN.** The board verifies
  against a dummy hash and counts the attempt against the board-wide budget.

## Deliverables
- [x] `store.rs`: ported from gamematcher (magic `MLS1`).
  - Two slots, CRC-32, generations.
  - Refuses to start when both slots are corrupt, or when the schema is newer than the firmware.
- [x] `pin.rs`: ported (PBKDF2-SHA256, per-person lockout, board-wide failure budget).
- [x] `state.rs`: people, sessions, sign-up switch, commit count. The bounds are 16 people and 64
  sessions, 4 per person.
- [x] Routes (all in `api.rs`):
  - `GET /status` and `POST /setup` (optional `MERITLEDGER_SETUP_CODE`)
  - `POST /auth/signup` (guest), `POST /auth/pin`, `POST /auth/logout[?all=1]`
  - `GET /people`, `GET|PATCH /me`, `POST /me/pin` (the first PIN needs no current one)
  - `GET|POST /admin/people`, `PATCH|DELETE /admin/people/{id}`
  - `GET|PATCH /admin/settings`, `GET /admin/storage`
  - Every unsafe method needs the board's own origin.
- [x] Admin invariants:
  - At least one active admin stays.
  - You can't disable or remove yourself.
  - Disabling someone, or removing their PIN, ends their sessions.
  - An admin PIN reset clears that person's lockout.
- [x] Desktop settings: `MERITLEDGER_DATA` (default `.local/state`), `MERITLEDGER_SD`, `MERITLEDGER_SETUP_CODE`.
- [x] Board: state in the `mlstate` NVS partition. The PIN-hash timing is logged at every boot.
- [x] UI:
  - Setup, sign-in (name tiles, then PIN), guest sign-up.
  - Profile: name, email, PIN, sign out here or everywhere.
  - Household page: add people, role, PIN, disable, remove, sign-up switch, storage.
  - Route guards, plus the update-recovery overlay ported from gamematcher.

## Exit criteria
- [x] Tests: 21 unit tests and 19 API tests. They cover:
  - A torn newest slot loads the previous generation.
  - Both slots corrupt → refuses to start.
  - Eight concurrent setups → exactly one 201.
  - Lockout with `Retry-After`, and the admin reset clearing it.
  - Session eviction (4 per person), and sign-out here and everywhere.
  - The 17th person → 409 `full`.
  - A foreign `Origin` → 403.
- [x] State and sessions survive a restart: an API test, plus a desktop smoke run with snapshot files on disk.
- [x] **State survives a board reboot**, checked 2026-10-18: the owner set up the board, and the account
  was still there after a reset and three app-only updates.
- [x] PIN rounds measured on the S3: 1,000 rounds took 63 ms, so `Limits::board()` uses 1,600, which
  measured exactly 100 ms on the board.

## Out of scope
Mastodon sign-in (Sprint 11), email (Sprint 10), practice data (Sprint 3).

## Handoff
- The owner set the board up on 2026-10-18.
- Sprint 3 settled both items this sprint carried over: removing a person, and board-side recording.
  See its handoff.
- **Snapshot size today is 478 bytes** for 2 people and 2 sessions, so there is plenty of room for
  Sprint 3's 60-day entry window.
- The admin page uses `prompt()` and `confirm()` for a new PIN and for removal. That's plain but works on phones;
  replace it if it grates.
