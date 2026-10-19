# Page loads from the board (Sprint 4)

**Setup:** 2026-10-18. The board was at 192.168.1.151 on channel 11, reading **RSSI -77 dBm**.
Gamematcher, on the same access point, reads -56 dBm. The client was a wired desktop running
`tools/pageload.py`: HTTPS + HTTP/2, a new TLS connection per load, and the board reached by IP so that
Windows' slow `.local` lookups (about 2 s on a first request) aren't counted.

## Results

A **cold** load fetches the page, every asset and the profile page's three API calls, with an empty cache.
A **warm** load skips the hashed assets, which browsers cache as `immutable`.

| | Cold (5 runs) | Warm (5 runs) |
|---|---|---|
| Release build, TCP send buffer 5,760 | 2.7–4.9 s, median 3.0 s | 1.1–1.6 s, median 1.2 s |
| Same, send buffer 23,040 (experiment, reverted) | 2.5–4.1 s, median 4.0 s | 1.1–1.6 s, median 1.1 s |

What the time goes on:
- **`main.js`** is 273 KB, 83 KB gzipped, and took 1.4–3.6 s of the cold load. That's 25–60 KB/s.
  A warm keep-alive `curl` of the same file by IP measured 66–171 KB/s.
- **The TLS handshake is about 1.0 s** (`/api/v1/sys`: `handshake_ms_avg` 998). It is most of a warm
  load in this test, because each run is a new client. Browsers resume sessions with tickets, so repeat visits
  in a real browser skip it.
- **API handlers take 1–2 ms** (`Server-Timing`). The board's code is not the bottleneck.

## Verdict

**Sprint 4's "under 1 s" target is not met for a cold load, and the radio link is the cause.**
- A 4× larger TCP send buffer made no measurable difference, so it was reverted.
- The board's signal is 21 dB weaker than gamematcher's on the same AP, and throughput per download
  varies 3× from run to run.

What would help, in order:
1. **Move the board** somewhere with a better signal (aim for -65 dBm or stronger), then re-run
   `uv run tools/pageload.py <ip> meritledger_rs/certs/household-ca.crt`.
2. **A smaller first bundle.** Today is eager; every other page is lazy. Angular's runtime is most of
   the 83 KB, so expect a small gain at best.
3. Nothing is needed for repeat visits. After the first visit a phone fetches only `index.html` and
   API JSON (well under 2 KB).
