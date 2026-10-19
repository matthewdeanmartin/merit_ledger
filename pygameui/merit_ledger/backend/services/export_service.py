"""Export (JSON + Markdown) and JSON import (spec §19)."""

from __future__ import annotations

from typing import Any

from merit_ledger.backend.domain.models import Dedication, LedgerEntry, PracticeTemplate, Profile, Settings, Vow
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.repository.item_keys import (
    to_dedication_item,
    to_entry_item,
    to_profile_item,
    to_settings_item,
    to_template_item,
    to_vow_item,
)
from merit_ledger.backend.services import profile_service
from merit_ledger.local.config import DEFAULT_USER_ID

PRIVACY_WARNING = "Exported files may contain private reflections. Review before sharing."

_ENTITY_BUCKETS = {
    "TEMPLATE": "templates",
    "ENTRY": "entries",
    "VOW": "vows",
    "DEDICATION": "dedications",
}


def export_json(repo: MeritRepository, user_id: str = DEFAULT_USER_ID) -> dict[str, Any]:
    """Return a coherent JSON snapshot of the ledger.

    Scans the whole table (cheap locally) and buckets items by entity type, so new
    entity types added in later sprints are picked up automatically once they land in
    :data:`_ENTITY_BUCKETS`.
    """
    profile = profile_service.get_profile(repo, user_id)
    settings = profile_service.get_settings(repo, user_id)

    snapshot: dict[str, Any] = {
        "version": 1,
        "profile": profile.model_dump(),
        "settings": settings.model_dump(),
        "tradition": settings.tradition,
        "templates": [],
        "entries": [],
        "vows": [],
        "dedications": [],
    }
    for item in repo.scan_all():
        bucket = _ENTITY_BUCKETS.get(item.entity_type)
        if bucket:
            snapshot[bucket].append(item.item)
    return snapshot


def export_markdown(repo: MeritRepository, user_id: str = DEFAULT_USER_ID) -> str:
    """Return a human-readable Markdown export (spec §19.2)."""
    snap = export_json(repo, user_id)
    profile = snap["profile"]
    settings = snap["settings"]

    lines: list[str] = ["# Merit Ledger Export", "", f"> {PRIVACY_WARNING}", "", "## Profile", ""]
    lines.append(f"Mode: {profile.get('tradition', 'secular')}")
    lines.append(f"Points mode: {settings.get('points_mode', 'points')}")
    lines.append("")
    lines.append("## Recent Entries")
    lines.append("")

    # newest first
    entries = sorted(snap["entries"], key=lambda e: e.get("occurred_at", ""), reverse=True)
    for e in entries:
        date = e.get("occurred_at", "")[:10]
        lines.append(f"### {date} - {e.get('title', '(untitled)')}")
        lines.append("")
        qty = e.get("quantity")
        if qty is not None:
            unit = e.get("quantity_unit") or ""
            lines.append(f"Quantity: {qty} {unit}".rstrip() + "  ")
        lines.append(f"Points: {e.get('points', 0)}  ")
        if e.get("category"):
            lines.append(f"Category: {e['category']}  ")
        if e.get("repair_intention"):
            lines.append(f"Repair intention: {e['repair_intention']}  ")
        if e.get("reflection"):
            lines.append("")
            lines.append("Reflection:  ")
            lines.append(str(e["reflection"]))
        lines.append("")
    return "\n".join(lines)


def import_json(repo: MeritRepository, snapshot: dict[str, Any], user_id: str = DEFAULT_USER_ID) -> dict[str, int]:
    """Import a snapshot produced by :func:`export_json`, upserting all entities.

    Returns a per-bucket count of imported items. Idempotent on primary keys (upsert).

    Raises:
        ValueError: If the snapshot version is unsupported.
    """
    version = snapshot.get("version")
    if version != 1:
        raise ValueError(f"Unsupported export version: {version!r}")

    counts = {"profile": 0, "settings": 0, "templates": 0, "vows": 0, "entries": 0, "dedications": 0}

    if snapshot.get("profile"):
        repo.put_item(to_profile_item(Profile.model_validate(snapshot["profile"])))
        counts["profile"] = 1
    if snapshot.get("settings"):
        repo.put_item(to_settings_item(Settings.model_validate(snapshot["settings"])))
        counts["settings"] = 1
    for raw in snapshot.get("templates", []):
        repo.put_item(to_template_item(PracticeTemplate.model_validate(raw), user_id))
        counts["templates"] += 1
    for raw in snapshot.get("vows", []):
        repo.put_item(to_vow_item(Vow.model_validate(raw)))
        counts["vows"] += 1
    for raw in snapshot.get("entries", []):
        repo.put_item(to_entry_item(LedgerEntry.model_validate(raw)))
        counts["entries"] += 1
    for raw in snapshot.get("dedications", []):
        repo.put_item(to_dedication_item(Dedication.model_validate(raw)))
        counts["dedications"] += 1
    return counts
