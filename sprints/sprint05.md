# Sprint 5 — History on SD, backups and export
**State:** done 2026-10-18 (desktop and board), except the long-history speed target (see the exit criteria and handoff)
**Depends on:** 1 (SD module), 3 (entries)   **Parallel with:** 6, 7

## Goal
Keep history indefinitely on a 32 GB SD card, back the whole ledger up automatically, and let
anyone export their data in standard formats. Without a card, keep working, with older detail
treated as ephemeral.

## Owner direction (2026-10-18)
- Big, chatty data (entries) moves to the SD card. A tap **appends** to a log on the card instead of
  rewriting the snapshot.
- This is the first app in the repo to use an SD card, so this sprint writes the guidance:
  [docs/SD_CARD.md](../docs/SD_CARD.md).

## Questions for the owner
1. **Backups kept:** I adopted the proposal: 14 daily, then the first of each month for 24 months.
   It's a constant in `transfer.rs` if the owner wants different numbers.
2. **Export:** both. Anyone can download their own entries as CSV (settings page); admins can download
   everything as JSON (Household page).
3. **Restore:** from the UI, as an **import** (admin): a merge with a dry run first, and safe to
   repeat. No USB job is needed.

## What was built
- **The journal** (`journal.rs`): `ml/journal/NNNNNN.log`, files closed at 1 MiB.
  - A record is `A5 4C | len | CRC-32 | CBOR(StoredEntry)`.
  - Edits append the new version, deletes append a tombstone, and the last record per id wins.
  - A torn tail in the newest file is cut off at open. Damage in an older file keeps everything before it.
- **Bounded memory:**
  - Files are read one at a time, and records decoded one at a time.
  - Finding each id's last record uses an index of 20 bytes per record.
  - A cheap head peek (id, person, date, tombstone) means a query only fully decodes what it keeps.
- **Saving** (`api.rs`): after each change, the RAM entries are compared with their state before.
  - With a card, changed entries are appended to the journal; the snapshot is rewritten only if
    something else changed. Tested: five taps, zero snapshot commits.
  - Without a card, the snapshot is committed as before.
  - Handlers didn't change.
- **The window:** the last 60 days live in RAM. Older days are kept as per-day totals
  (`stats::Tally`), split by what the person sees versus what others see, so stats stay
  privacy-correct.
  - With a card, the totals are rebuilt from the journal at boot.
  - Without one, they're persisted (`archived_days`) as entries age out.
- **Migration:** entries in the snapshot (recorded before Sprint 5, or without a card) move to the
  journal at the next boot with a card. Replaying after a crash is harmless.
- **Ids** (no snapshot write per tap):
  - With a card, the snapshot holds a lease renewed every 1,000 ids.
  - A boot with the card takes the journal's highest id ever written, deleted ones included.
  - A boot without the card starts after the lease.
- **Backups:** nightly after 03:00 household time to `ml/backup/YYYY-MM-DD.json`, written atomically
  (`.tmp` then rename) and rotated. "Back up now" and per-file download are on the Household page.
- **Export and import** (`transfer.rs`, [docs/EXPORT_FORMAT.md](../docs/EXPORT_FORMAT.md)):
  - JSON export of the whole household: no PINs, no email.
  - Per-person CSV (yourself, or an admin).
  - Import matches people by name and custom kinds by name and owner; an entry already on the board
    is skipped. `?dry_run=1` changes nothing. Uploads up to 16 MB, streamed in.
- **Storage report:** journal files, records and bytes, torn and damaged files, window size, days
  kept as totals, last backup, and "card missing".
- **UI:**
  - Household page: storage, backups, export and import.
  - Settings page: "Download my entries (CSV)".
- **Gate:** `POST /api/v1/gate/journal?entries=N` benchmarks a synthetic journal in a scratch
  folder, then deletes it.
- **Framework:** `Reply::owned_body()`, a read-only accessor with a test, added to
  `../microcontroller/miniframework/crates/miniframework/src/site.rs` so tests can read owned
  replies. **That's an uncommitted change in the other repo.**

## Exit criteria
- [x] Tests:
  - A truncated last record is cut off, and earlier records and later appends still read.
  - A crash between the journal write and the snapshot commit loses nothing and doubles nothing.
  - Export → import into an empty board gives the same streaks and minutes, including after a restart.
  - Importing twice adds nothing.
  - Rollups match a recomputation: a randomized test over 400 entries and 4 kinds of viewer, with
    and without a card, and across a snapshot reload.
  - Peek agrees with a full decode and takes the fast path.
  - Uppercase FAT names are found.
  - 62 unit tests, 19 + 10 + 10 API tests, and 2 gate tests.
- [ ] **Ten years of synthetic history reads one month in under 500 ms: not met.**
  - Measured with 4 people, not 16: 0.9 s at 1 year, 3.7 s at 5, 6.4 s at 10.
  - Boot takes about 2 s at 1 year and about 20 s at 10.
  - See [s3-journal.md](../spec/measurements/s3-journal.md). Fine for this household today (under a year
    of history), but it's the next storage task (handoff).
- [x] **Pull the card:** the board keeps serving, "card missing" shows, and stats from the totals
  show. Tested on desktop (`a_missing_card_is_reported`); on the board, the no-card path ran from
  Sprint 1 until the card went in.

## Handoff
- **Next storage task: make reads proportional to what they read.** Every query scans the whole journal
  (about 55 µs per record peeked, about 125 µs per record decoded; the card itself isn't the cost). A design that fits:
  - A **checkpoint file** (`ml/checkpoint.cbor`), written with the nightly backup. It holds the
    day totals, the id high-water mark, each person's newest window entries, and the journal
    position it covers. Boot reads the checkpoint, then only the records after that position.
  - A **month index** (`ml/index/YYYY-MM.idx`): for each month, the file and offset of every final
    record dated in it. It's rebuilt from the checkpoint pass and appended on writes. A month query
    reads one index and decodes only those records.
  - Compaction (rewriting old files without superseded records) is optional after that.
- **Found on the board, not on desktop:** `DirEntry::file_type()` fails on the FAT VFS (no `lstat`), so
  files vanished from listings. Fixed in `Sd::list` and written into `docs/SD_CARD.md`. Test listings on
  the board.
- **Boot with no clock:** the window's reference day is the newest date in the newest journal file.
  `tick` rolls the window once SNTP sets the clock.
- **The owner's board** ran the migration on 2026-10-18 with zero entries, so nothing moved. It is
  journaled now (`/api/v1/admin/storage`).
