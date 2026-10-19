# Deploying to the board

**Board:** Waveshare ESP32-S3-Touch-LCD-1.47.
- MAC `28:84:85:b1:70:e4`
- 16 MiB flash, 8 MiB octal PSRAM
- One USB-C port, which enumerates as `303A:1001` (COM5)

Board notes, pins and the factory backup are in
`../microcontroller/BOARD_SKILL_ESP32_S3_TOUCH_LCD_1_47.md`. Measured limits are in
[spec/measurements/s3-gate.md](spec/measurements/s3-gate.md).

**Status:** the board was repaved on 2026-10-18 (the Waveshare factory demo is gone; a backup exists).
It runs Merit Ledger at `http://meritledger.local/` and `https://meritledger.local/`.

## Settings

Wi-Fi is compiled into the firmware from the environment or a gitignored `.env`. `build.rs` looks
in this order:

1. `meritledger_rs/.env`
2. `./.env` (the repo root)
3. `$MINIFRAMEWORK_ENV`
4. `C:\github\.env`

```sh
MERITLEDGER_WIFI_SSID=...
MERITLEDGER_WIFI_PASSWORD=...
# optional
MERITLEDGER_HOSTNAME=meritledger
MERITLEDGER_SETUP_CODE=...      # first-run setup must quote it
```

The generic `MINIFRAMEWORK_WIFI_SSID`, `MINIFRAMEWORK_WIFI_PASSWORD`, `WIFI_SSID` and
`WIFI_PASSWORD` also work. The S3 radio is **2.4 GHz only**.

## Build

```sh
make certs                  # once: certs/meritledger.crt + .key from the household CA
make firmware               # TRANSPORT=http2 (default; includes HTTPS) | https | http
make firmware GATE=1        # adds the Sprint 1 load generators; never for daily use
```

The build writes the following to `meritledger_rs/.embuild/`:
- `meritledger-s3.bin`
- `bootloader.bin`
- `partition-table.bin`
- `deployment-images.json`, a manifest of sizes and SHA-256 that the deploy script checks

Compiling never opens a serial port.

## Flash

```sh
make board-check PORT=COM5  # read-only: identity, and would `update` be allowed?
make update PORT=COM5       # the normal case: app only, keeps everyone's data
make flash PORT=COM5        # FIRST INSTALL / REPAVE ONLY: erases the whole board
```

Every mode first checks the chip, the 16 MB flash and the MAC (`tools/deploy-s3.py`).
`update` refuses unless the board already has Merit Ledger's partition table, so the `mlstate`
partition (people, sessions, and later the ledger) survives.

**After a `make flash`, press the board's RESET button.** After the first repave, the board
failed Wi-Fi authentication for minutes until it was reset by hand. App-only updates came back on
their own. The Wi-Fi signal where the board sat was weak (-84 dBm); if it drops off the network,
look at placement first.

## Rules

- The owner says yes before any flash, every session.
- `make flash` is for a first install or a deliberate repave only. Everything else is `make update`.
- Read board state over HTTP, not long serial captures:
  - `/api/v1/status`
  - `/api/v1/sys` (heap, Wi-Fi, TLS)
  - `/api/v1/log.txt` (recent log, including the PIN-hash timing at boot)
  - `/api/v1/admin/storage` (admin)
  - `/.well-known/ops/health` (public checks for mastomini-bots' `sre`: heap, Wi-Fi, clock,
    `store.people`, `store.slot`, and `sd.space` / `sd.journal` when a card is present)
- Windows curl needs `--ssl-no-revoke` for HTTPS checks against the household CA (it publishes no
  revocation list). It still verifies the chain and hostname; never use `-k`.

## Deployments

### 2026-10-19: ops health endpoint (SRE sprint 0)

- Owner attached the board for this flash. COM5, MAC `28:84:85:b1:70:e4`, 16 MB. `make check`
  passed; `make board-check` confirmed the Merit Ledger partition table (its RTS reset restarted
  the board, which came back by itself).
- `make update`: app 2,381,152 / 6,291,456 bytes (37.8%), written at `0x10000` only, hash
  verified. No backup (owner's choice; slow USB).
- Before: build `9ee1a2e+`. After: build `970ff1d+`, setup done, clock synced; HTTP and strict
  HTTPS status 200 at `192.168.1.151`.
- Health all `ok`: 68 KiB internal free (lowest 65 KiB), largest block 31 KiB, −68 dBm, 1/16
  people, newest snapshot slot, SD card present (about 31 GB, almost empty), journal intact.
- Logs: `.local/ops-deploy-*.log`.
