# Using the SD card

Merit Ledger is the first app in this household's ESP32 repos to keep data on an SD card. This is
what it does and why, written so the next app can copy the parts that fit. The code is
`meritledger_rs/src/sd.rs` (plain file operations), `journal.rs` (the entry log), the
`attach_journal`, `save` and `backup_now` functions in `api.rs`, and the `card` module in `src/bin/s3.rs`.

## The short version

- **Flash (NVS) is for small state that changes rarely:** people, settings, custom practices. It is
  one snapshot in two CRC-checked slots (gamematcher's scheme), rewritten whole on each change.
- **The SD card is for data that grows:** every practice entry, as an append-only log, plus nightly
  backups.
- **The board works without a card.** Recent entries then live in the snapshot, and older ones are
  kept only as daily totals. Nothing nags about it; the Household page says so once.

## Why not just the snapshot?

A snapshot is the simplest crash-safe design: write the whole state to the slot that isn't
current, and a power cut costs at most the last change. But it rewrites everything on every change.
With practice entries that meant **258 KB of flash written per tap** once the window was full
(Sprint 3). A log turns a tap into about 100 bytes appended.

## The card

| | |
|---|---|
| Interface | 4-bit SDMMC on the S3 Touch 1.47: CLK 16, CMD 15, D0–D3 17/18/13/14 |
| Driver | `esp-idf-hal` `SdMmcHostDriver::new_4bits` → `SdCardDriver::new_mmc` → `esp-idf-svc` `Fatfs` + `MountedFatfs` at `/sd` |
| Filesystem | FAT32. Cards over 32 GB (SDXC) usually ship as exFAT: reformat them as FAT32, or enable exFAT in FATFS |
| Long names | `CONFIG_FATFS_LFN_HEAP=y`; otherwise names are 8.3 only |
| Afterwards | Plain `std::fs` works (the VFS), on the board and on desktop alike, **except** `DirEntry::file_type()`: it needs `lstat`, which the FAT VFS doesn't provide. List folders with `stat` by path (`Sd::list` does), and compare names case-insensitively |
| No card | `send_op_cond` times out (`ESP_ERR_TIMEOUT`) in about 10 ms; the app runs without one |

The card mounts once at boot (`SIGNALS.step(8, "sd card")`). A card inserted later is used after
the next reset. On desktop, a directory stands in for it (`MERITLEDGER_SD`, default `.local/sd`).

## Measured on the board (2026-10-18)

| Operation | Time |
|---|---|
| Append 4 KB + fsync | 66 ms typical, 92 ms worst (under full network load) |
| Write 1 MiB + fsync | 1.5–2.0 s (0.5–0.7 MB/s) |
| Read 1 MiB | 210 ms (about 5 MB/s) |

Journal throughput is in `spec/measurements/s3-journal.md`. In short:
- A tap is 15 ms.
- Boot reads a year of history in about 2 s.
- Per-record CPU work (CRC, CBOR), not the card, is what grows with history.

**Consequences:**
- fsync is the cost, not bytes: batch what you can into one append.
- Writing is slow enough to notice: never write megabytes on the serving path.
- Reading is fast: scanning a year of entries at boot is cheap.

## Layout

```
/sd/ml/
  journal/000001.log, 000002.log …   practice entries, append-only, a new file past 4 MiB
  backup/2026-10-18.json …           nightly exports
  posts/12.md …                      vow post bodies (markdown), one file per post id
  texts/3.md …                       household text bodies (markdown), one file per text id
  gate/                              scratch space for the gate's benchmarks (gate builds only)
```

Everything an app writes lives under its own top folder (`ml/`), so cards can be shared or inspected
on a PC.

## The journal

**Record:** `A5 4C | length u16 LE | CRC-32 u32 LE | payload`, where the payload is one
`StoredEntry` as CBOR with integer keys.
- An edit appends the whole new version.
- A delete appends a tombstone (`deleted: true`).
- Reading keeps the **last** record for each id.

**Writes:**
- One mutation becomes one batch: one `write` and one `fsync` per file, from the request's handler.
- About 15 ms per tap measured (one record), on the serving task, which runs on its own core on the S3.
- Files are closed at 1 MiB; a big batch (an import) spills across files.
- If the append fails, the change is rolled back in RAM and the client gets a 500.

**Crash safety:**
- **Power cut mid-append:** the newest file ends in a torn record. At open, the reader stops at
  the first record whose magic, length or CRC is wrong, and **truncates the newest file there**.
  Otherwise later appends would land after garbage and be unreadable.
- **A bad record in an older file** (a card fault): reading stops there for that file. Everything
  before it, and every other file, is kept, and the Household page reports it.
- **Migration from the snapshot** (entries recorded without a card, or before Sprint 5): the
  entries are appended to the journal, then the snapshot is rewritten without them. A crash
  between the two leaves the entries in both places. They migrate again at the next boot, and since
  the last record per id wins, nothing doubles.
- **Ids:** with a card, the snapshot doesn't record each new id (that would mean a snapshot write per
  tap). Instead it holds a *lease*, renewed every 1,000 ids. A boot with the card takes the
  journal's highest id ever written, tombstones included, so deleted ids are never reused. A boot
  without the card starts after the lease.

**Memory is bounded:**
- Files are read one at a time, and records are decoded one at a time.
- Finding each id's last record needs an index of 20 bytes per record.
- A cheap "head peek" reads id, person, date and the tombstone flag without decoding the whole record.
  It relies on the wire format writing fields in order; a test checks it never silently falls back.

**At boot** the journal is read twice (check, then index and read):
- Entries from the last 60 days go into RAM.
- Older days become per-day totals (`stats::Tally`), so streaks and heatmaps reach back as far as
  the card does.

Old entries are read back from the card on demand: a profile's history beyond the window, exports,
CSV.

## What the snapshot still does

People, sessions, settings, custom practices, the id lease, the "card used before" flag, and
the daily totals of entries that aged out while there was no card. With a card, a tap doesn't touch
it. The tests check this (`taps_append_to_the_journal_not_the_snapshot`).

## Backups

Every night after 03:00 household time the board writes the whole household to
`ml/backup/YYYY-MM-DD.json`: the export format in [EXPORT_FORMAT.md](EXPORT_FORMAT.md), written to
a `.tmp` file, synced, then renamed.
- **Rotation:** the newest 14 daily files, plus the first file of each month for 24 months.
- **On demand:** "Back up now" on the Household page.
- **Downloads:** any backup can be downloaded from that page (streamed from the card, 4 KiB of RAM).
- **Restoring:** import the file on the Household page. It's a merge; a dry run shows what would change.

## Card missing, card replaced

| Situation | What happens |
|---|---|
| Never had a card | Snapshot mode, as before Sprint 5 |
| Card removed | Snapshot mode for new entries; "card missing" on the Household page; history beyond the window is unavailable until the card is back |
| Card back | New entries move to the card at the next boot; nothing is lost |
| New, empty card | Treated as a card returning empty: new entries go to it, old history is not on it. Restore a backup with Import to fill it |

## Rules for the next app

1. Decide what grows and what doesn't: what grows goes on the card, in an append-only form.
2. Put a CRC on every record, and truncate the torn tail before appending again.
3. Batch the fsyncs. A small append costs 15–70 ms, mostly the sync.
4. Keep the board useful without a card, and say plainly what is lost without one.
5. Never write large files from the serving task. Build in RAM under the lock, and write outside it
   (backups from `tick`).
6. Make re-applying a write harmless (last-wins by id), so crash recovery can just replay.
7. Measure on the real card. Cards differ a lot, and the FAT VFS differs from a desktop
   filesystem in ways tests won't show (file listing, name case).
8. Keep the cost of a query proportional to what it reads. A scan of everything grows with years;
   index by month before history gets long (open item in Sprint 5).
