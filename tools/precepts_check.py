"""Check every citation in meritledger_rs/catalog/precepts.json against CBETA.

For each precept, the `quote` must appear in the source text starting at its
first citation (allowing for the passage to run on over the next lines). Other
citations, and `label_cite`, must contain `label_zh`. Punctuation is ignored,
and 床/牀 are treated as the same character (editions differ).

Uses the CBETA data API (cbdata.dila.edu.tw, Dharma Drum / DILA, Taiwan).
Needs the network; stdlib only:

    uv run --no-project python tools/precepts_check.py
"""

import html
import json
import re
import ssl
import sys
import urllib.parse
import urllib.request
from pathlib import Path

API = "https://cbdata.dila.edu.tw/stable/lines"
CATALOG = Path(__file__).resolve().parent.parent / "meritledger_rs" / "catalog" / "precepts.json"
# CBETA's certificate lacks a Subject Key Identifier, which Python 3.13's
# strict X.509 mode rejects. The chain and hostname are still verified.
TLS = ssl.create_default_context()
TLS.verify_flags &= ~ssl.VERIFY_X509_STRICT
PUNCT = re.compile(r"[\s、，。！？：；「」『』（）〔〕【】]")


def norm(text: str) -> str:
    return PUNCT.sub("", text).replace("牀", "床")


def lines(head: str, after: int) -> str:
    query = urllib.parse.urlencode({"linehead": head, "before": 0, "after": after})
    with urllib.request.urlopen(f"{API}?{query}", timeout=30, context=TLS) as resp:
        results = json.load(resp)["results"]
    if not results or results[0]["linehead"] != head:
        raise LookupError(f"{head}: no such line")
    return norm("".join(html.unescape(re.sub(r"<[^>]+>", "", r["html"])) for r in results))


def main() -> int:
    sys.stdout.reconfigure(encoding="utf-8")
    sets = json.loads(CATALOG.read_text(encoding="utf-8"))["sets"]
    failures = checked = 0
    for s in sets:
        for item in s["items"]:
            name = f'{s["key"]} {item["kind"]} {item["n"]}'
            checks = [(item["cite"][0], 16, item["quote"])]
            checks += [(c, 3, item["label_zh"]) for c in item["cite"][1:]]
            if "label_cite" in item:
                checks.append((item["label_cite"], 2, item["label_zh"]))
            for head, after, want in checks:
                checked += 1
                try:
                    ok = norm(want) in lines(head, after)
                except LookupError as err:
                    print(f"FAIL {name}: {err}")
                    failures += 1
                    continue
                except OSError as err:
                    print(f"CBETA unreachable: {err}")
                    return 2
                if not ok:
                    print(f"FAIL {name}: {want!r} not at {head}")
                    failures += 1
    print(f"{checked} citations checked, {failures} failed")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
