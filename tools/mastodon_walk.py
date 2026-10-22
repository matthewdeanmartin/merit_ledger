"""Walks Mastodon linking, publishing and sign-in on a FRESH desktop Merit
Ledger against tools/mock_mastodon.py, over HTTP (no browser):

1. set up the board (Dad, PIN 1234);
2. link Dad's Mastodon account (the mock's @sam), as Settings does;
3. record a sit, share it, and see the status on the mock;
4. delete the sit and see the status deleted on the mock;
5. sign out, then "Sign in with Mastodon".

    uv run --no-project python tools/mock_mastodon.py --port 8099 &
    MERITLEDGER_MASTODON_MOCK=http://127.0.0.1:8099 MERITLEDGER_DATA=<empty dir> \\
        MERITLEDGER_SD=none cargo run   # in meritledger_rs
    uv run --no-project python tools/mastodon_walk.py http://127.0.0.1:8089 http://127.0.0.1:8099
"""

import json
import sys
import time
import urllib.error
import urllib.request
from http.cookiejar import CookieJar

sys.stdout.reconfigure(encoding="utf-8")
board, instance = sys.argv[1].rstrip("/"), sys.argv[2].rstrip("/")
jar = CookieJar()


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None


opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar), NoRedirect)


def call(method: str, url: str, body: object | None = None) -> tuple[int, dict, dict]:
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Origin", board)
    req.add_header("Content-Type", "application/json")
    try:
        with opener.open(req, timeout=15) as r:
            raw = r.read()
            return r.status, dict(r.headers), json.loads(raw) if raw.strip().startswith(b"{") else {}
    except urllib.error.HTTPError as e:
        raw = e.read()
        return e.code, dict(e.headers), json.loads(raw) if raw.strip().startswith(b"{") else {}


def api(method: str, path: str, body: object | None = None, want: int | None = None) -> dict:
    status, _, data = call(method, f"{board}{path}?fmt=json" if "?" not in path else f"{board}{path}&fmt=json", body)
    if want is not None and status != want:
        raise SystemExit(f"{method} {path}: HTTP {status} {data}")
    return data


def oauth(provider: str, link: bool) -> str:
    """Runs one sign-in or link through the mock; returns the result."""
    ticket = api("POST", "/api/v1/auth/oauth/start", {"provider": provider, "link": link}, 200)["ticket"]
    for _ in range(50):
        s = api("GET", f"/api/v1/auth/oauth/{ticket}")
        if s.get("state") == "redirect":
            break
        time.sleep(0.1)
    else:
        raise SystemExit(f"no redirect: {s}")
    # The instance approves at once and sends the browser back to the board.
    status, headers, _ = call("GET", s["location"])
    back = headers.get("Location", "")
    assert status == 302 and back.startswith(board + "/auth/callback"), (status, back)
    status, headers, _ = call("GET", back)
    assert status == 302 and "/signin/finishing?t=" in headers.get("Location", ""), (status, headers)
    for _ in range(50):
        s = api("GET", f"/api/v1/auth/oauth/{ticket}")
        if s.get("state") in ("done", "failed"):
            break
        time.sleep(0.1)
    assert s["state"] == "done", s
    return s["result"]


def mock_statuses() -> dict:
    with urllib.request.urlopen(f"{instance}/_mock/statuses", timeout=10) as r:
        return json.load(r)


def wait(check, what: str) -> None:
    for _ in range(100):
        if check():
            return
        time.sleep(0.1)
    raise SystemExit(f"timed out: {what}")


providers = api("GET", "/api/v1/auth/providers", None, 200)["providers"]
assert any(p["id"] == "mock" for p in providers), providers
api("POST", "/api/v1/setup", {"display_name": "Dad", "pin": "1234", "setup_code": ""}, 201)
print("1. set up: Dad")

assert oauth("mock", link=True) == "linked"
me = api("GET", "/api/v1/me", None, 200)
assert me["linked"].startswith("@sam@"), me
print(f"2. linked {me['linked']}")

api("POST", "/api/v1/entries", {"person": 0, "kind": "meditation", "amount": 25, "note": "Walked through the mock"}, 201)
# The mock may hold statuses from earlier walks: watch for a new one.
before = set(mock_statuses())
shared = api("POST", "/api/v1/entries/1/publish", None, 202)
assert shared["state"] == "queued", shared


def new_status() -> dict | None:
    fresh = {k: v for k, v in mock_statuses().items() if k not in before}
    return next(iter(fresh.items()), None)


wait(lambda: new_status() is not None, "the status")
sid, status = new_status()
assert "Meditation · 25 min" in status["status"] and "Walked through the mock" in status["status"], status
print(f"3. shared: status {sid} says {status['status'].splitlines()[0]!r} ({status['visibility']})")

api("DELETE", "/api/v1/entries/1", None, 200)
wait(lambda: mock_statuses()[sid]["deleted"], "the delete")
print("4. deleted on the board, deleted on the mock")

api("POST", "/api/v1/auth/logout", None, 200)
assert call("GET", f"{board}/api/v1/me?fmt=json")[0] == 401
assert oauth("mock", link=False) == "signed_in"
me = api("GET", "/api/v1/me", None, 200)
assert me["display_name"] == "Dad", me
print("5. signed in with Mastodon as", me["display_name"])
print("walk OK")
