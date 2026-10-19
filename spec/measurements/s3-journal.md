# The SD journal on the board (Sprint 5)

**Setup:** 2026-10-18, on the Waveshare S3 Touch 1.47 with the household's 32 GB FAT32 card. The
data came from `POST /api/v1/gate/journal?entries=N` (gate builds only). It writes synthetic entries
(4 people, 5 a day, going back in time) to a scratch journal under `ml/gate/bench`, times what a real
board does with them, then deletes them. The household's journal is never touched.

## Results (final firmware: streaming reads, head peeks)

| Records | Size | Write in batches of 500 | One tap (1 record + fsync) | `open` (boot, 1st read) | `for_each` (boot, 2nd read; export) | One person, one month |
|---|---|---|---|---|---|---|
| 7,320 (1 household-year) | 0.5 MB | 2.5 s | 15 ms | 0.48 s | 1.4 s | 0.9 s |
| 36,520 (5 years) | 2.6 MB | 21 s | 17 ms | 2.6 s | 7.4 s | 3.7 s |
| 73,020 (10 years) | 5.2 MB | 34 s | 15 ms | 5.2 s | 14.9 s | 6.4 s |

- **Memory:** PSRAM free never dropped below 4.6 MB of 8 MB, at 10 years during `for_each`, from
  the 20-byte-per-record index. Internal heap minimum 34 KB.
- **Boot** is `open` plus `for_each`: about 1.9 s for a year of history, about 20 s for ten.

## Earlier runs (for comparison)

| Firmware | 73,020 records: `open` / `for_each` |
|---|---|
| Full decode of every record in both passes | 9.3 s / 18.8 s |
| Head peeks in pass 1 (final) | 5.2 s / 14.9 s |

**The very first run found a real bug:** the journal's files were invisible after a reboot.
- **Cause:** `DirEntry::file_type()` on ESP-IDF's FAT VFS needs `lstat`, which FAT doesn't provide. My
  listing filter turned that error into "not a file".
- **Fix:** `Sd::list` uses `stat` by path, and lower-cases names (FAT may list 8.3 names in upper case).
- **Lesson:** a desktop test would never have caught it. Test file listing on the board.

## What costs the time

Not the card: a 1 MiB read takes 210 ms (Sprint 1). What remains is per record:
- Checking the CRC of each payload, peeking at its head, and parsing its date: about 55 µs.
- Decoding a full `StoredEntry` from CBOR: about 125 µs.

Every query today reads the whole journal, so cost grows with history.

## Verdict against Sprint 5's exit criterion

"Ten years of synthetic history reads one month in under 500 ms": **not met.** It's 6.4 s at
ten years (4 people), and 0.9 s at one year. The household has under a year of history, so daily
use is fine. The fix is structural, not a tuning knob: see the handoff in `sprints/sprint05.md`
(a month index or checkpoint, so a query reads only its months and a boot only new records).
