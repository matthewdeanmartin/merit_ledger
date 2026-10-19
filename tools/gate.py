# /// script
# requires-python = ">=3.12"
# dependencies = ["httpx2[http2]>=2.14"]
# ///
"""Sprint 1 hardware gate (sprints/sprint01.md), ported from gamematcher.

Loads the board the way a household morning would, while the board's own
gate worker makes outbound HTTPS requests and appends to the SD card:

- HTTP_CLIENTS keep-alive HTTP/1.1 clients polling /api/v1/status and the UI;
- one HTTPS/1.1 client (household CA, SNI meritledger.local) reconnecting
  every few requests, so full TLS handshakes keep happening;
- H2_CLIENTS HTTP/2 "browsers" (HTTPX2, a fresh connection per page load
  of 4 multiplexed requests), like phones on the timer page;
- the board's outbound gate (POST /api/v1/gate) for the same duration;
- an SD write-sync-read of 1 MiB before and after (POST /api/v1/gate/sd);
- /api/v1/sys sampled every 10 s for heap and TLS counters.

Pass: no reboot (uptime never goes backwards), no TLS allocation failure in
the board log, every outbound target succeeds at least once, no server TLS
handshake failures, and the largest free internal block stays >= 24 KiB.
Standard library plus HTTPX2 for HTTP/2 (Git Bash's curl has none).

    uv run tools/gate.py --url http://192.168.1.170 --ca meritledger_rs/certs/household-ca.crt
"""
import argparse
import asyncio
import http.client
import json
import socket
import ssl
import subprocess
import threading
import time
from urllib.parse import urlparse

MIN_LARGEST = 24 * 1024
SNI = "meritledger.local"


class Counter:
    def __init__(self) -> None:
        self.lock = threading.Lock()
        self.ok = 0
        self.failed = 0
        self.errors: dict[str, int] = {}
        self.max_ms = 0.0

    def record(self, ok: bool, ms: float, error: str = "") -> None:
        with self.lock:
            if ok:
                self.ok += 1
            else:
                self.failed += 1
                self.errors[error] = self.errors.get(error, 0) + 1
            self.max_ms = max(self.max_ms, ms)

    def summary(self) -> dict:
        return dict(ok=self.ok, failed=self.failed, max_ms=round(self.max_ms), errors=self.errors)


def get_json(host: str, port: int, path: str, method: str = "GET", timeout: float = 10) -> dict:
    conn = http.client.HTTPConnection(host, port, timeout=timeout)
    headers = {"Origin": f"http://{host}"} if method != "GET" else {}
    conn.request(method, path, headers=headers)
    response = conn.getresponse()
    body = response.read()
    conn.close()
    if response.status >= 400:
        raise RuntimeError(f"{method} {path}: HTTP {response.status} {body[:200]!r}")
    return json.loads(body)


def http_client(host: str, port: int, stop: threading.Event, counter: Counter) -> None:
    paths = ["/api/v1/status?fmt=json", "/", "/api/v1/status?fmt=json"]
    conn = None
    i = 0
    while not stop.is_set():
        path = paths[i % len(paths)]
        i += 1
        started = time.perf_counter()
        try:
            if conn is None:
                conn = http.client.HTTPConnection(host, port, timeout=10)
            # Like a browser: the UI is stored gzip-only.
            conn.request("GET", path, headers={"Accept": "text/html,*/*", "Accept-Encoding": "gzip"})
            response = conn.getresponse()
            response.read()
            counter.record(response.status == 200, (time.perf_counter() - started) * 1000, str(response.status))
        except Exception as e:  # noqa: BLE001 - every failure is a data point
            counter.record(False, (time.perf_counter() - started) * 1000, type(e).__name__)
            if conn is not None:
                conn.close()
            conn = None
            time.sleep(1)
        time.sleep(0.5)


class BoardHTTPS(http.client.HTTPSConnection):
    """Connects to the board's address but verifies it as meritledger.local."""

    def __init__(self, host: str, context: ssl.SSLContext) -> None:
        super().__init__(host, 443, timeout=15, context=context)
        self.tls_context = context

    def connect(self) -> None:
        raw = socket.create_connection((self.host, self.port), self.timeout)
        self.sock = self.tls_context.wrap_socket(raw, server_hostname=SNI)


def https_client(host: str, ca: str, stop: threading.Event, counter: Counter, reconnect_every: int) -> None:
    context = ssl.create_default_context(cafile=ca)
    conn = None
    sent = 0
    while not stop.is_set():
        started = time.perf_counter()
        try:
            if conn is None or sent % reconnect_every == 0:
                if conn is not None:
                    conn.close()
                conn = BoardHTTPS(host, context)
            conn.request("GET", "/api/v1/status?fmt=json", headers={"Host": SNI})
            response = conn.getresponse()
            response.read()
            sent += 1
            counter.record(response.status == 200, (time.perf_counter() - started) * 1000, str(response.status))
        except Exception as e:  # noqa: BLE001
            counter.record(False, (time.perf_counter() - started) * 1000, type(e).__name__)
            if conn is not None:
                conn.close()
            conn = None
            sent = 0
            time.sleep(1)
        time.sleep(1)


def h2_client(host: str, ca: str, stop: threading.Event, counter: Counter) -> None:
    """A page load over one HTTP/2 connection (4 requests multiplexed),
    every 3 s. Each load is a fresh connection, so a full handshake."""
    import httpx2

    context = ssl.create_default_context(cafile=ca)
    paths = ["/", "/api/v1/status?fmt=json", "/api/v1/schema", "/api/v1/gate/storage?fmt=json"]

    async def page_load() -> tuple[bool, str]:
        # Connect to the board's address but verify it as meritledger.local.
        transport = httpx2.AsyncHTTPTransport(http2=True, verify=context)
        async with httpx2.AsyncClient(transport=transport, base_url=f"https://{host}",
                                      headers={"Host": SNI}, timeout=20) as client:
            responses = await asyncio.gather(*(client.get(p, extensions={"sni_hostname": SNI}) for p in paths))
        bad = [f"{r.status_code}/{r.http_version}" for r in responses if r.status_code != 200 or r.http_version != "HTTP/2"]
        return not bad, " ".join(bad)

    while not stop.is_set():
        started = time.perf_counter()
        try:
            ok, why = asyncio.run(page_load())
            counter.record(ok, (time.perf_counter() - started) * 1000, why)
        except Exception as e:  # noqa: BLE001
            counter.record(False, (time.perf_counter() - started) * 1000, type(e).__name__)
        time.sleep(3)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--url", required=True, help="http://<board ip>")
    parser.add_argument("--ca", required=True, help="household CA (PEM)")
    parser.add_argument("--secs", type=int, default=300)
    parser.add_argument("--http-clients", type=int, default=3)
    parser.add_argument("--h2-clients", type=int, default=2)
    parser.add_argument("--no-https", action="store_true", help="plain-HTTP firmware")
    parser.add_argument("--reconnect-every", type=int, default=5)
    parser.add_argument("--out", help="write the JSON result here too")
    args = parser.parse_args()
    parsed = urlparse(args.url)
    host, port = parsed.hostname or "", parsed.port or 80

    before = get_json(host, port, "/api/v1/sys?fmt=json")
    print(f"Board: {before['chip']} {before['build']} uptime {before['uptime_ms'] / 1000:.0f} s, reset {before['reset_reason']}")
    print(f"Heap before: free {before['heap']['internal_free']:,} largest {before['heap']['internal_largest']:,} min {before['heap']['internal_min']:,}")
    storage = get_json(host, port, "/api/v1/gate/storage?fmt=json")
    print(f"SD: {storage}")
    sd_before = get_json(host, port, "/api/v1/gate/sd?fmt=json", method="POST", timeout=60) if storage.get("sd_present") else None
    print(f"SD 1 MiB round trip before load: {sd_before}")
    get_json(host, port, f"/api/v1/gate?fmt=json&secs={args.secs}", method="POST")

    stop = threading.Event()
    http_counter, https_counter, h2_counter = Counter(), Counter(), Counter()
    threads = [threading.Thread(target=http_client, args=(host, port, stop, http_counter)) for _ in range(args.http_clients)]
    if not args.no_https:
        threads.append(threading.Thread(target=https_client, args=(host, args.ca, stop, https_counter, args.reconnect_every)))
        threads += [threading.Thread(target=h2_client, args=(host, args.ca, stop, h2_counter)) for _ in range(args.h2_clients)]
    for t in threads:
        t.start()

    samples = []
    rebooted = False
    last_uptime = before["uptime_ms"]
    deadline = time.monotonic() + args.secs
    while time.monotonic() < deadline:
        time.sleep(10)
        try:
            sys_info = get_json(host, port, "/api/v1/sys?fmt=json")
        except Exception as e:  # noqa: BLE001
            print(f"  sys sample failed: {type(e).__name__}")
            continue
        if sys_info["uptime_ms"] < last_uptime:
            rebooted = True
            print(f"  REBOOT detected: reset {sys_info['reset_reason']}")
        last_uptime = sys_info["uptime_ms"]
        heap, net = sys_info["heap"], sys_info["net"]
        samples.append(dict(t=round(sys_info["uptime_ms"] / 1000), free=heap["internal_free"], largest=heap["internal_largest"],
                            min=heap["internal_min"], psram_free=heap.get("psram_free"), tls_handshakes=net["tls_handshakes"],
                            tls_failures=net["tls_failures"], tls_open=net["tls_open"], http_open=net["http_open"],
                            rejected=net["rejected"]))
        s = samples[-1]
        print(f"  t={s['t']}s free={s['free']:,} largest={s['largest']:,} min={s['min']:,} psram={s['psram_free']} "
              f"tls={s['tls_handshakes']}/{s['tls_failures']}fail http_open={s['http_open']}")
    stop.set()
    for t in threads:
        t.join(timeout=40)

    gate = get_json(host, port, "/api/v1/gate?fmt=json")
    sd_after = get_json(host, port, "/api/v1/gate/sd?fmt=json", method="POST", timeout=60) if storage.get("sd_present") else None
    after = get_json(host, port, "/api/v1/sys?fmt=json")
    log = http.client.HTTPConnection(host, port, timeout=10)
    log.request("GET", "/api/v1/log.txt")
    log_text = log.getresponse().read().decode("utf-8", "replace")
    alloc_failures = [line for line in log_text.splitlines() if "alloc" in line.lower() and ("fail" in line.lower() or "err" in line.lower())]

    smallest_largest = min((s["largest"] for s in samples), default=0)
    outbound_ok = all(t["ok"] > 0 for t in gate["targets"])
    tls_failures = samples[-1]["tls_failures"] - before["net"]["tls_failures"] if samples else 0
    result = dict(
        board=dict(build=after["build"], chip=after["chip"], reset_reason=after["reset_reason"]),
        secs=args.secs,
        http=http_counter.summary(),
        https=None if args.no_https else https_counter.summary(),
        h2=None if args.no_https else h2_counter.summary(),
        outbound=gate["targets"],
        sd=dict(info=storage, before=sd_before, after=sd_after),
        heap_min_ever=after["heap"]["internal_min"],
        psram_min_ever=after["heap"].get("psram_min"),
        smallest_largest_block=smallest_largest,
        server_tls_failures=tls_failures,
        rebooted=rebooted,
        alloc_failure_lines=alloc_failures[-10:],
        samples=samples,
    )
    passed = not rebooted and not alloc_failures and outbound_ok and smallest_largest >= MIN_LARGEST and tls_failures == 0
    result["passed"] = passed
    print(json.dumps({k: v for k, v in result.items() if k != "samples"}, indent=2))
    print("GATE PASSED" if passed else "GATE FAILED")
    if args.out:
        with open(args.out, "w", encoding="utf-8") as f:
            json.dump(result, f, indent=2)


if __name__ == "__main__":
    main()
