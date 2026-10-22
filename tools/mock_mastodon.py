# /// script
# requires-python = ">=3.11"
# ///
"""A tiny stand-in Mastodon instance for trying Mastodon sign-in, linking and
publishing without a real one (after gamematcher's tools/mock_mastodon.py,
plus statuses).

It does what Merit Ledger needs and nothing else:

- OAuth discovery, app registration, an authorize page that approves at once,
  the token exchange with PKCE checked, verify_credentials and revoke;
- posting, editing and deleting statuses (a token needs `write:statuses`);
- `GET /_mock/statuses`: what's been posted, for the walk to check.

One account: @sam (id 42).

    uv run --no-project python tools/mock_mastodon.py --port 8099
"""

import argparse
import base64
import hashlib
import json
import secrets
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlencode, urlparse

apps: dict[str, dict] = {}
codes: dict[str, dict] = {}
# token -> scopes granted
tokens: dict[str, set[str]] = {}
# status id -> {"status", "visibility", "deleted"}
statuses: dict[str, dict] = {}


class Handler(BaseHTTPRequestHandler):
    def base(self) -> str:
        return f"http://{self.headers.get('Host')}"

    def send_json(self, status: int, body: object) -> None:
        data = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def raw(self) -> bytes:
        length = int(self.headers.get("Content-Length") or 0)
        return self.rfile.read(length)

    def form(self) -> dict[str, str]:
        return {k: v[0] for k, v in parse_qs(self.raw().decode()).items()}

    def scopes(self) -> set[str] | None:
        token = self.headers.get("Authorization", "").removeprefix("Bearer ")
        return tokens.get(token)

    def do_GET(self) -> None:
        url = urlparse(self.path)
        q = {k: v[0] for k, v in parse_qs(url.query).items()}
        if url.path == "/.well-known/oauth-authorization-server":
            b = self.base()
            return self.send_json(200, {
                "issuer": b + "/",
                "authorization_endpoint": b + "/oauth/authorize",
                "token_endpoint": b + "/oauth/token",
                "revocation_endpoint": b + "/oauth/revoke",
                "scopes_supported": ["read", "write", "profile", "write:statuses"],
                "code_challenge_methods_supported": ["S256"],
            })
        if url.path == "/oauth/authorize":
            app = apps.get(q.get("client_id", ""))
            if not app or q.get("redirect_uri") not in app["redirect_uris"]:
                return self.send_json(400, {"error": "invalid_request"})
            back = q["redirect_uri"]
            asked = set(q.get("scope", "").split())
            if not asked <= app["scopes"]:
                target = back + "?" + urlencode({"error": "invalid_scope", "state": q.get("state", "")})
            else:
                code = secrets.token_urlsafe(16)
                codes[code] = {"client_id": q["client_id"], "challenge": q.get("code_challenge", ""),
                               "redirect_uri": back, "scopes": asked}
                target = back + "?" + urlencode({"code": code, "state": q.get("state", "")})
            self.send_response(302)
            self.send_header("Location", target)
            self.send_header("Content-Length", "0")
            self.end_headers()
            return
        if url.path == "/api/v1/accounts/verify_credentials":
            if self.scopes() is None:
                return self.send_json(401, {"error": "The access token is invalid"})
            return self.send_json(200, {"id": "42", "username": "sam", "acct": "sam", "display_name": "Sam (mock)",
                                        "avatar": "", "emojis": [], "fields": []})
        if url.path == "/_mock/statuses":
            return self.send_json(200, statuses)
        return self.send_json(404, {"error": "Record not found"})

    def do_POST(self) -> None:
        url = urlparse(self.path)
        if url.path == "/api/v1/statuses":
            if "write:statuses" not in (self.scopes() or set()):
                return self.send_json(403, {"error": "This action is outside the authorized scopes"})
            body = json.loads(self.raw() or b"{}")
            sid = str(1000 + len(statuses))
            statuses[sid] = {"status": body.get("status", ""), "visibility": body.get("visibility", "public"), "deleted": False}
            return self.send_json(200, {"id": sid, "url": f"{self.base()}/@sam/{sid}"})
        f = self.form()
        if url.path == "/api/v1/apps":
            cid, secret = secrets.token_urlsafe(12), secrets.token_urlsafe(24)
            apps[cid] = {"secret": secret, "redirect_uris": f.get("redirect_uris", "").split("\n"),
                         "scopes": set(f.get("scopes", "read").split())}
            return self.send_json(200, {"id": str(len(apps)), "name": f.get("client_name"), "client_id": cid, "client_secret": secret})
        if url.path == "/oauth/token":
            grant = codes.pop(f.get("code", ""), None)
            app = apps.get(f.get("client_id", ""))
            verifier = f.get("code_verifier", "")
            challenge = base64.urlsafe_b64encode(hashlib.sha256(verifier.encode()).digest()).rstrip(b"=").decode()
            ok = (grant and app and app["secret"] == f.get("client_secret") and grant["client_id"] == f.get("client_id")
                  and grant["redirect_uri"] == f.get("redirect_uri") and grant["challenge"] == challenge)
            if not ok:
                return self.send_json(400, {"error": "invalid_grant"})
            token = secrets.token_urlsafe(24)
            tokens[token] = grant["scopes"]
            return self.send_json(200, {"access_token": token, "token_type": "Bearer",
                                        "scope": " ".join(sorted(grant["scopes"])), "created_at": 0})
        if url.path == "/oauth/revoke":
            tokens.pop(f.get("token", ""), None)
            return self.send_json(200, {})
        return self.send_json(404, {"error": "Record not found"})

    def do_PUT(self) -> None:
        url = urlparse(self.path)
        sid = url.path.removeprefix("/api/v1/statuses/")
        if "write:statuses" not in (self.scopes() or set()) or sid not in statuses:
            return self.send_json(404, {"error": "Record not found"})
        statuses[sid]["status"] = json.loads(self.raw() or b"{}").get("status", "")
        return self.send_json(200, {"id": sid, "url": ""})

    def do_DELETE(self) -> None:
        url = urlparse(self.path)
        sid = url.path.removeprefix("/api/v1/statuses/")
        if "write:statuses" not in (self.scopes() or set()) or sid not in statuses:
            return self.send_json(404, {"error": "Record not found"})
        statuses[sid]["deleted"] = True
        return self.send_json(200, {"id": sid})

    def log_message(self, fmt: str, *args) -> None:  # quiet
        pass


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, default=8099)
    parser.add_argument("--host", default="127.0.0.1")
    args = parser.parse_args()
    print(f"mock Mastodon on http://{args.host}:{args.port}", flush=True)
    ThreadingHTTPServer((args.host, args.port), Handler).serve_forever()


if __name__ == "__main__":
    main()
