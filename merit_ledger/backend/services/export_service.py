"""Full-snapshot export (spec §19.1). Markdown export lands in Sprint 4."""

from __future__ import annotations

from typing import Any

from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.services import profile_service
from merit_ledger.local.config import DEFAULT_USER_ID

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
