# /// script
# requires-python = ">=3.12"
# dependencies = ["httpx2[http2]>=2.14"]
# ///
"""Page-load timing for the board (Sprint 4). Cold load of /p/1 by IP (no mDNS), HTTPS + HTTP/2,
SNI meritledger.local.

    uv run tools/pageload.py <board ip> meritledger_rs/certs/household-ca.crt [runs]

Prints each run's total and the main bundle's time; then a warm load (assets cached)."""
import asyncio
import re
import ssl
import sys
import time

import httpx2

IP, CA, RUNS = sys.argv[1], sys.argv[2], int(sys.argv[3]) if len(sys.argv) > 3 else 5
SNI = "meritledger.local"
EXT = {"sni_hostname": SNI}


async def load(cached: bool) -> tuple[float, float]:
    ctx = ssl.create_default_context(cafile=CA)
    t = httpx2.AsyncHTTPTransport(http2=True, verify=ctx)
    async with httpx2.AsyncClient(transport=t, base_url=f"https://{IP}", headers={"Host": SNI}, timeout=30) as c:
        start = time.perf_counter()
        page = await c.get("/p/1", extensions=EXT)
        assets = [] if cached else re.findall(r'(?:src|href)="([^"]+\.(?:js|css))"', page.text)
        api = ["/api/v1/status?fmt=json", "/api/v1/me?fmt=json", "/api/v1/people/1/profile?fmt=json"]
        main_t = 0.0

        async def get(path: str) -> None:
            nonlocal main_t
            s = time.perf_counter()
            await c.get(path, extensions=EXT)
            if "main-" in path:
                main_t = time.perf_counter() - s

        await asyncio.gather(*(get("/" + a) for a in assets), *(get(p) for p in api))
        return time.perf_counter() - start, main_t


async def main() -> None:
    cold = [await load(False) for _ in range(RUNS)]
    warm = [await load(True) for _ in range(RUNS)]
    fmt = lambda xs: ", ".join(f"{x * 1000:.0f}" for x in xs)
    print(f"cold totals ms: {fmt([c[0] for c in cold])}")
    print(f"  main.js ms:   {fmt([c[1] for c in cold])}")
    print(f"warm totals ms: {fmt([w[0] for w in warm])}  (assets cached; new TLS connection each)")
    s = sorted(c[0] for c in cold)
    print(f"cold median {s[len(s) // 2] * 1000:.0f} ms; warm median {sorted(w[0] for w in warm)[len(warm) // 2] * 1000:.0f} ms")


asyncio.run(main())
