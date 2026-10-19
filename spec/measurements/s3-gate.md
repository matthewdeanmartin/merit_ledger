# Sprint 1 gate: the Waveshare S3 Touch 1.47, measured

**Date and setup:** 2026-10-18.
- Board `28:84:85:b1:70:e4`: ESP32-S3 rev 0.2, 8 MiB octal PSRAM, 16 MiB flash.
- Firmware `9ee1a2e+` with `http2,gate`.
- 192.168.1.151 on the household Wi-Fi: WPA2, channel 11, **RSSI -84 dBm**.
- Load from `tools/gate.py`; raw samples in [s3-gate.json](s3-gate.json).

## Verdict

**Pass.** The S3 serves HTTP, HTTPS and HTTP/2 under load, makes outbound HTTPS with both CA
kinds, and writes to the SD card at the same time.

- **First run (no card):** network side clean. The SD target failed for lack of a card.
  Raw samples: [s3-gate.json](s3-gate.json).
- **Second run, 2026-10-18 with a 32 GB FAT32 card:** everything passed.
  Raw samples: [s3-gate-sd.json](s3-gate-sd.json).

## Load

5 minutes, all at once:
- 3 keep-alive HTTP/1.1 clients polling status and the UI every 0.5 s.
- 1 HTTPS/1.1 client (household CA, SNI `meritledger.local`) reconnecting every 5 requests.
- 2 HTTP/2 "browsers" (HTTPX2): a fresh connection every 3 s, with 4 multiplexed requests each.
- The board's gate worker, fetching in turn:
  - gamematcher.local (household CA)
  - mastodon.social discovery (public CA)
  - an SD append
- `/api/v1/sys` sampled every 10 s.

## Results

| | Result |
|---|---|
| Reboots / hangs | none |
| HTTP requests | 1,667 ok, 0 failed (max 2.5 s) |
| HTTPS requests | 203 ok, 0 failed (max 4.2 s, full handshakes) |
| HTTP/2 page loads | 126 ok (504 requests), 0 failed (max 3.9 s) |
| Server TLS handshake failures | 0 |
| Outbound, household CA | 85 ok, 0 failed; 1.8 s typical, 3.3 s max |
| Outbound, public CA | 85 ok, 0 failed; 0.8 s typical, 2.3 s max |
| SD append (4 KB + fsync, run 2) | 85 ok, 0 failed; 66 ms typical, 92 ms max |
| SD 1 MiB write + fsync (run 2) | 1.5 s before load, 2.0 s after (0.5–0.7 MB/s) |
| SD 1 MiB read (run 2) | 210 ms (about 5 MB/s) |
| Run 2, same load with SD writes | HTTP 1,676 / HTTPS 186 / HTTP/2 107 page loads, all 0 failed; internal heap min 29,435; largest block min 31,744 |
| Internal heap, minimum ever | 29,271 bytes |
| Smallest "largest free internal block" | 30,720 bytes (bar: 24 KiB) |
| PSRAM free, minimum | 7.95 MB of 8 MB |
| TLS handshake (one, idle) | about 1.0 s, TLS 1.2, ALPN h2 |

## Notes for later sprints

- **Internal RAM is the tight resource, not PSRAM.** At 29 KB minimum free internal heap there's
  room, but the display driver (Sprint 13) needs DMA-capable internal memory for its strip buffer.
  Measure again then. If it's tight, lower `CONFIG_SPIRAM_MALLOC_ALWAYSINTERNAL` (16 KiB today) so
  more allocations go to PSRAM.
- **Wi-Fi signal is weak (-84 dBm) where the board sat.** Its first boot after flashing failed auth
  for minutes (reason 2, `AUTH_EXPIRE`), and joined in 4 s after a press of the RESET button.
  `DEPLOY.md` now says to press RESET after flashing.
- **Image size:** 1.6 MB with `http2,gate`, which is 25% of the 6 MiB app partition. The public CA
  bundle and HTTP/2 are most of the growth from the 0.5 MB plain-HTTP build. The partition
  table stands as proposed.
- **PSRAM:** the octal PSRAM tested OK at boot (8 MiB at 80 MHz, about 0.4 s of self-test).
- **SD:** with an empty slot the card driver reports `ESP_ERR_TIMEOUT` and the board runs on
  without a card, as designed. With a card it mounts in about 10 ms at boot.
  - Writes with fsync are slow-ish (0.5–0.7 MB/s), and small appends cost about 70 ms each.
  - Sprint 5's archiver should batch records into one append per commit, and stay off the serving task.
