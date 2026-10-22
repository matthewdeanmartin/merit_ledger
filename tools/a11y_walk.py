# /// script
# requires-python = ">=3.11"
# dependencies = ["playwright>=1.48"]
# ///
"""Accessibility walk (Sprint 14): every page, in the light and dark
themes, checked by axe-core (WCAG 2.1 A and AA: labels, names, contrast,
landmarks), signed in as an admin of a small seeded household.

Runs the installed Microsoft Edge (no browser download) against a desktop
server with throwaway data:

    MERITLEDGER_DATA=$(mktemp -d) MERITLEDGER_SD=none make run-bundle &
    uv run tools/a11y_walk.py [http://127.0.0.1:8089]

Prints each page's violations and exits 1 if there are any. axe-core comes
from meritledger_ui/node_modules (npm install).
"""

import json
import sys
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parent.parent
AXE = ROOT / "meritledger_ui" / "node_modules" / "axe-core" / "axe.min.js"
BASE = sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8089"
PAGES = [
    "/", "/house", "/p/1", "/me", "/books", "/events", "/events/1", "/notifications",
    "/build", "/texts", "/precepts", "/calendar", "/timer", "/practices", "/admin",
    "/admin/messages",
]
SIGNED_OUT = ["/signin", "/signup", "/guide", "/house", "/build", "/books", "/timer", "/calendar"]


def seed(request) -> None:
    """Dad (admin, PIN 1234), Kid, an entry, an event and a vow."""
    post = lambda path, body: request.post(BASE + path, data=json.dumps(body), headers={"Content-Type": "application/json", "Origin": BASE})
    r = post("/api/v1/setup", {"display_name": "Dad", "pin": "1234", "setup_code": ""})
    if r.status != 201:
        post("/api/v1/auth/pin", {"person": 1, "pin": "1234"})
        return
    post("/api/v1/admin/people", {"display_name": "Kid", "role": "member", "pin": "1111", "email": ""})
    post("/api/v1/entries", {"person": 1, "kind": "meditation", "amount": 20, "note": "Morning sit", "client_id": ""})
    post("/api/v1/events", {"title": "Uposatha", "start": 1_900_000_000, "end": 0, "place": "Home", "notes": "", "invitees": [2], "rev": 0})
    post("/api/v1/vows", {"text": "Kind speech", "started": "", "ends": "", "visibility": "household", "precept_set": "", "precept_kind": "", "precept_n": 0})


def check(page, path: str) -> list[dict]:
    page.goto(BASE + path, wait_until="networkidle")
    page.wait_for_timeout(300)
    page.add_script_tag(path=str(AXE))
    result = page.evaluate(
        """() => axe.run(document, { runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'] } })
             .then(r => r.violations.map(v => ({ id: v.id, impact: v.impact, help: v.help,
                 nodes: v.nodes.map(n => n.target.join(' ') + ' :: ' + (n.failureSummary || '').split('\\n').slice(1, 2).join('')) })))"""
    )
    return result


def main() -> int:
    if not AXE.exists():
        print(f"missing {AXE}: run npm install in meritledger_ui", file=sys.stderr)
        return 2
    failures = 0
    with sync_playwright() as p:
        browser = p.chromium.launch(channel="msedge")
        for scheme in ("light", "dark"):
            # Signed out first, then signed in.
            anon = browser.new_context(color_scheme=scheme, viewport={"width": 390, "height": 844})
            pages = [(anon.new_page(), path) for path in SIGNED_OUT]
            ctx = browser.new_context(color_scheme=scheme, viewport={"width": 390, "height": 844})
            seed(ctx.request)
            pages += [(ctx.new_page(), path) for path in PAGES]
            for page, path in pages:
                violations = check(page, path)
                page.close()
                mark = "ok" if not violations else f"{len(violations)} problem(s)"
                print(f"[{scheme}] {path}: {mark}")
                for v in violations:
                    failures += 1
                    print(f"    {v['impact']}: {v['id']}: {v['help']}")
                    for n in v["nodes"][:4]:
                        print(f"      - {n}")
            anon.close()
            ctx.close()
        browser.close()
    print("clean" if not failures else f"{failures} violation(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
