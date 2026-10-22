# /// script
# requires-python = ">=3.12"
# dependencies = ["httpx2[http2]>=2.14"]
# ///
"""Load walk (Sprint 14): four phones at a morning sit, at once, with the
screen preview open. Each phone signs in, then for the whole run:

- polls today and the bell every 5 s (what open pages do);
- reloads the app shell (index.html and its main bundle) every 60 s;
- opens a practice to follow along (practice list, one practice, a text)
  every 30 s;
- logs a one-minute sit every 20 s (deleted again at the end: on the
  board this is the household's real ledger);
- fetches the screen's frame.bmp every 30 s.

/api/v1/sys (the board's heap and uptime) is sampled every 10 s when
present. Prints latency per kind of request (p50, p95, max), errors, and
the heap's low points.

    uv run tools/load_walk.py --url http://127.0.0.1:8089 --person 1 --pin 1234
    uv run tools/load_walk.py --url https://192.168.1.151 --ca meritledger_rs/certs/household-ca.crt \\
        --person 1 --pin <PIN> --seconds 300
"""

import argparse
import asyncio
import re
import ssl
import statistics
import time
from collections import defaultdict

import httpx2

SNI = "meritledger.local"


class Stats:
    def __init__(self) -> None:
        self.times: dict[str, list[float]] = defaultdict(list)
        self.errors: dict[str, int] = defaultdict(int)

    def report(self) -> None:
        print(f"{'request':<14}{'n':>6}{'p50 ms':>9}{'p95 ms':>9}{'max ms':>9}")
        for kind, ts in sorted(self.times.items()):
            ts = sorted(ts)
            p95 = ts[min(len(ts) - 1, int(len(ts) * 0.95))]
            print(f"{kind:<14}{len(ts):>6}{statistics.median(ts):>9.0f}{p95:>9.0f}{ts[-1]:>9.0f}")
        for e, n in sorted(self.errors.items()):
            print(f"  error x{n}: {e}")


async def timed(stats: Stats, kind: str, coro):
    start = time.perf_counter()
    try:
        r = await coro
    except Exception as e:  # noqa: BLE001 - a load walk counts every failure
        stats.errors[f"{kind}: {type(e).__name__}: {e}"] += 1
        return None
    stats.times[kind].append((time.perf_counter() - start) * 1000)
    if r.status_code >= 400:
        stats.errors[f"{kind}: HTTP {r.status_code}"] += 1
    return r


async def every(seconds: float, until: float, step) -> None:
    while time.monotonic() < until:
        started = time.monotonic()
        await step()
        await asyncio.sleep(max(0.0, seconds - (time.monotonic() - started)))


async def phone(n: int, args, stats: Stats, created: list[int], until: float) -> None:
    async with client(args) as c:
        r = await c.post("/api/v1/auth/pin?fmt=json", json={"person": args.person, "pin": args.pin}, extensions=ext(args))
        r.raise_for_status()
        get = lambda path: c.get(path, extensions=ext(args))

        async def poll():
            await timed(stats, "today", get("/api/v1/today?fmt=json"))
            await timed(stats, "bell", get("/api/v1/me/summary?fmt=json"))

        async def shell():
            page = await timed(stats, "index.html", get("/"))
            if page is not None:
                for js in re.findall(r'src="([^"]*main[^"]*\.js)"', page.text)[:1]:
                    await timed(stats, "main.js", get("/" + js.lstrip("/")))

        async def follow():
            listing = await timed(stats, "practices", get("/api/v1/practices?fmt=json"))
            ids = re.findall(r'"id":(\d+)', listing.text) if listing is not None else []
            if ids:
                await timed(stats, "practice", get(f"/api/v1/practices/{ids[0]}?fmt=json"))
            await timed(stats, "text", get("/texts/heart-sutra.txt"))

        async def log():
            r = await timed(stats, "log entry", c.post(
                "/api/v1/entries?fmt=json",
                json={"person": args.person, "kind": "meditation", "amount": 1, "note": f"load walk {n}", "client_id": ""},
                extensions=ext(args),
            ))
            if r is not None and r.status_code == 201:
                m = re.search(r'"id":(\d+)', r.text)
                if m:
                    created.append(int(m.group(1)))

        async def frame():
            await timed(stats, "frame.bmp", get("/api/v1/display/frame.bmp"))

        await asyncio.sleep(n * 0.7)  # phones don't arrive in the same millisecond
        await asyncio.gather(
            every(5, until, poll), every(60, until, shell), every(30, until, follow),
            every(20, until, log), every(30, until, frame),
        )
        # Tidy up the sits this phone logged.
        for _ in range(len(created)):
            if not created:
                break
            entry = created.pop()
            await c.delete(f"/api/v1/entries/{entry}?fmt=json", extensions=ext(args))


def client(args) -> httpx2.AsyncClient:
    if args.url.startswith("https"):
        ctx = ssl.create_default_context(cafile=args.ca) if args.ca else ssl.create_default_context()
        transport = httpx2.AsyncHTTPTransport(http2=True, verify=ctx)
        return httpx2.AsyncClient(transport=transport, base_url=args.url, timeout=30, headers={"Host": SNI, "Origin": f"https://{SNI}"})
    return httpx2.AsyncClient(base_url=args.url, timeout=30, headers={"Origin": args.url})


def ext(args) -> dict:
    return {"sni_hostname": SNI} if args.url.startswith("https") else {}


async def sample_sys(args, until: float, samples: list[dict]) -> None:
    async with client(args) as c:
        while time.monotonic() < until:
            try:
                r = await c.get("/api/v1/sys?fmt=json", extensions=ext(args))
                if r.status_code == 200:
                    samples.append(r.json())
                elif r.status_code == 404:
                    return  # the desktop has no /sys
            except Exception:  # noqa: BLE001
                pass
            await asyncio.sleep(10)


def heap_report(samples: list[dict]) -> None:
    if not samples:
        print("heap: no /api/v1/sys (desktop)")
        return
    numeric = defaultdict(list)
    for s in samples:
        for k, v in s.items():
            if isinstance(v, (int, float)) and ("free" in k or "largest" in k or "uptime" in k):
                numeric[k].append(v)
    for k, vs in sorted(numeric.items()):
        print(f"heap {k}: first {vs[0]}, min {min(vs)}, last {vs[-1]}")
    up = numeric.get("uptime_ms") or numeric.get("uptime_s") or []
    if any(b < a for a, b in zip(up, up[1:])):
        print("REBOOTED during the walk (uptime went backwards)")


async def main() -> None:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--url", default="http://127.0.0.1:8089")
    p.add_argument("--ca")
    p.add_argument("--person", type=int, default=1)
    p.add_argument("--pin", required=True)
    p.add_argument("--phones", type=int, default=4)
    p.add_argument("--seconds", type=int, default=120)
    args = p.parse_args()
    stats, created, samples = Stats(), [], []
    until = time.monotonic() + args.seconds
    print(f"{args.phones} phones for {args.seconds} s against {args.url}")
    await asyncio.gather(
        sample_sys(args, until, samples),
        *(phone(n, args, stats, created, until) for n in range(args.phones)),
    )
    stats.report()
    heap_report(samples)


if __name__ == "__main__":
    asyncio.run(main())
