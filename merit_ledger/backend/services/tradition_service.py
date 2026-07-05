"""Load and serve tradition packs (spec §6).

Packs are JSON files shipped inside the package. They configure labels, suggested
practices/vows, repentance categories, dedication language, and point defaults. We read
them via :mod:`importlib.resources` so they work from a wheel too.
"""

from __future__ import annotations

import json
from functools import lru_cache
from importlib import resources
from typing import Any

from merit_ledger.backend.domain.models import PracticeTemplate

_PACKAGE = "merit_ledger.backend.tradition_packs"
_TRADITION_IDS = ("zen", "chinese_mahayana", "nichiren", "pure_land", "secular")


@lru_cache(maxsize=None)
def _load_pack(tradition_id: str) -> dict[str, Any]:
    """Read and cache a single pack's JSON, raising KeyError if unknown."""
    if tradition_id not in _TRADITION_IDS:
        raise KeyError(tradition_id)
    text = resources.files(_PACKAGE).joinpath(f"{tradition_id}.json").read_text(encoding="utf-8")
    data: dict[str, Any] = json.loads(text)
    return data


def list_traditions() -> list[dict[str, str]]:
    """Return a lightweight list of available traditions (id + display name + theme)."""
    out: list[dict[str, str]] = []
    for tid in _TRADITION_IDS:
        pack = _load_pack(tid)
        out.append(
            {
                "tradition_id": pack["tradition_id"],
                "display_name": pack["display_name"],
                "theme": pack.get("theme", ""),
            }
        )
    return out


def get_tradition(tradition_id: str) -> dict[str, Any]:
    """Return the full pack dict for ``tradition_id``.

    Raises:
        KeyError: If the tradition is unknown.
    """
    return _load_pack(tradition_id)


def suggested_templates(tradition_id: str) -> list[PracticeTemplate]:
    """Return the pack's suggested practices as :class:`PracticeTemplate` objects."""
    pack = _load_pack(tradition_id)
    templates: list[PracticeTemplate] = []
    for raw in pack.get("suggested_practices", []):
        data = {"tradition": tradition_id, "is_custom": False, **raw}
        templates.append(PracticeTemplate.model_validate(data))
    return templates


def find_template(tradition_id: str, template_id: str) -> PracticeTemplate | None:
    """Return a suggested template by id from the given tradition, or None."""
    for tmpl in suggested_templates(tradition_id):
        if tmpl.template_id == template_id:
            return tmpl
    return None
