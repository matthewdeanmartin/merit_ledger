# Sprint 1 — Board bring-up and gate
**State:** done 2026-10-18
**Depends on:** 0   **Parallel with:** 2

## Goal
Get Merit Ledger onto the Waveshare ESP32-S3-Touch-LCD-1.47 with a safe deploy path. Measure
HTTPS, HTTP/2 and outbound HTTPS under load, and prove the SD card works, so later sprints can
rely on the board.

## Read first
- `../microcontroller/BOARD_SKILL_ESP32_S3_TOUCH_LCD_1_47.md`: identity, pins, the **factory backup**
  and recovery.
- `../microcontroller/gamematcher/DEPLOY.md`, `tools/deploy-c6.py`, `tools/gate.py` and
  `spec/measurements/c6-gate.md`. These were copied for this sprint.
- `../DEPLOY.md` (this repo)

## Already decided
- The board is the S3 Touch 1.47 (MAC `28:84:85:b1:70:e4`, 16 MiB flash, 8 MiB octal PSRAM), on COM5.
- HTTPS is needed for the timer, so the release transport is `http2` (which includes HTTPS).

## Questions for the owner (answered)
1. **Repave:** yes, 2026-10-18 ("board attached, backup already").
2. **SD card:** a 32 GB FAT32 card has been in the slot since 2026-10-18.
3. **Household-CA gate target:** `https://gamematcher.local/api/v1/status`, which is up and has a
   household-CA certificate. Override it with `MERITLEDGER_GATE_HOUSEHOLD_URL`.

## Deliverables
- [x] `tools/deploy-s3.py` (run with the ESP-IDF Python), ported from `deploy-c6.py`:
  - `--replace` checks the chip, the 16 MB flash and the MAC, then erases and writes everything.
  - `--update` writes the app only, and refuses unless the board's partition table is ours.
  - `--check` is read-only.
  - Make targets: `make board-check|flash|update PORT=COMn`. `tools/manifest.py` writes the SHA-256
    manifest that deploy verifies.
- [ ] ~~Board marker compiled into the image~~ **Deferred.** With one board, the MAC check does the same
  job. Add a marker if a second Merit Ledger board appears.
- [x] First flash on 2026-10-18. The board serves `http://` and `https://meritledger.local/`, and HTTP/2
  is negotiated by ALPN.
- [x] `sdkconfig.defaults` confirmed:
  - The octal PSRAM tests OK (8 MiB at 80 MHz).
  - The USB-Serial/JTAG console works.
  - TLS buffers go in PSRAM.
  - FATFS long names are on.
  - Heap numbers are in [s3-gate.md](../spec/measurements/s3-gate.md).
- [x] `partitions.csv` confirmed. The image is 1.67 MB with `http2`, which is 26.5% of the 6 MiB app partition.
- [x] SD code, which is in the firmware but not yet tested with a card:
  - `sd.rs` uses plain `std::fs`, with a directory standing in on desktop.
  - The board mounts the card in 4-bit SDMMC through esp-idf-hal and esp-idf-svc `Fatfs`, so no
    framework change was needed.
  - `GET /api/v1/admin/storage` (admin since Sprint 2).
  - `POST /api/v1/gate/sd` does a write-sync-read self-test, in gate builds only.
- [x] **SD with a card inserted:** mounts at boot (31.25 GB). The 1 MiB self-test writes in 1.5–2.0 s and reads in 210 ms. The second gate run passed with SD appends under load.
- [x] Gate (`tools/gate.py`, with the HTTP/2 clients on HTTPX2): network side passes. See
  [s3-gate.md](../spec/measurements/s3-gate.md).
- [x] `DEPLOY.md` up to date, including the "press RESET after `make flash`" finding.

## Exit criteria
- [x] The board serves the UI over HTTPS and HTTP/2, and survived several resets and two app-only updates.
- [x] Gate, network side:
  - no reboots
  - 0 server TLS handshake failures
  - outbound HTTPS 85/85 to both CA kinds
  - internal heap minimum 29 KB, largest free block at least 30 KB
- [x] SD data written before a reset is still there after it: free space was identical to the byte across the reboot. A true unplugged power cycle with a read-back of the contents is covered by Sprint 5, whose archive reads its logs back.
- [x] `make update` refusing a foreign partition table was proven live: `make board-check` on the factory
  image said "update would refuse", before the first flash. A different MAC is refused by the same
  code path (`identify()`).

## Out of scope
The display and touch screen (Sprint 13), OTA.

## Handoff
- **To re-run the gate:**
  ```sh
  make firmware GATE=1 && make update PORT=COM5
  make gate URL=http://meritledger.local
  make firmware && make update PORT=COM5   # back to the release build
  ```
  SDXC cards (over 32 GB) usually come formatted as exFAT. ESP-IDF's FATFS needs exFAT enabled,
  or the card reformatted as FAT32. The household's 32 GB card is FAT32 and mounts.
- **SD appends with fsync cost about 70 ms,** and 1 MiB writes run at 0.5–0.7 MB/s. Sprint 5's
  archiver should batch, and stay off the serving task.
- **Internal RAM is the tight resource, not PSRAM.** The minimum was 29 KB free internal heap under load.
  The display's DMA buffers (Sprint 13) will need it. See the notes in `s3-gate.md`.
- **Wi-Fi at -84 dBm** where the board sat. The first boot after a repave failed auth until a manual RESET.
- **Git Bash's curl has no HTTP/2,** and with the household CA it fails verification. Use Python (`gate.py`)
  or a browser to test the board's TLS.
- `tools/gate.py` reads storage from `/api/v1/gate/storage` (gate builds), because `/admin/storage` now
  needs an admin.
