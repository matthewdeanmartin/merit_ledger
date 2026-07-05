"""Repentance service (spec §5.7, §10).

Repentance is category-based and non-secret. It is implemented as a
``repentance_completed`` ledger entry with structured fields — never a free-form diary of
who/what/where. A repentance may link to a broken vow but does not, on its own, close the
vow's repair; the user completes repair explicitly (see sprints/sprint4.md design note).
"""

from __future__ import annotations

from typing import get_args

from merit_ledger.backend.domain.models import LedgerEntry, RepentanceCategory
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.services import entry_service, profile_service, tradition_service
from merit_ledger.local.config import DEFAULT_USER_ID

PRIVACY_REMINDER = (
    "Keep this general. Do not record secrets, names, identifying details, illegal "
    "details, or private third-party information."
)


def categories() -> list[str]:
    """Return the repentance categories (spec §10)."""
    return list(get_args(RepentanceCategory))


def _repentance_points(repo: MeritRepository, user_id: str) -> int:
    """Default points for returning to practice, from the active tradition pack."""
    settings = profile_service.get_settings(repo, user_id)
    try:
        pack = tradition_service.get_tradition(settings.tradition)
        return int(pack.get("point_defaults", {}).get("repentance", 5))
    except KeyError:
        return 5


def create_repentance(
    repo: MeritRepository,
    *,
    category: RepentanceCategory,
    title: str | None = None,
    reflection: str | None = None,
    repair_intention: str | None = None,
    linked_vow_id: str | None = None,
    user_id: str = DEFAULT_USER_ID,
) -> LedgerEntry:
    """Create a repentance ('return to practice') entry (spec §5.7, §10).

    Points may be awarded for returning to practice (spec §12.3), pulled from the active
    tradition pack and scored through the entry service so the user's points mode applies.
    """
    settings = profile_service.get_settings(repo, user_id)
    entry = LedgerEntry(
        user_id=user_id,
        entry_type="repentance_completed",
        tradition=settings.tradition,
        title=title or "Returned to practice",
        category=category,
        repentance_style="category",
        reflection=reflection,
        repair_intention=repair_intention,
        linked_vow_id=linked_vow_id,
        vow_id=linked_vow_id,
    )
    return entry_service.create_entry(
        repo, entry, manual_points=_repentance_points(repo, user_id), user_id=user_id
    )
