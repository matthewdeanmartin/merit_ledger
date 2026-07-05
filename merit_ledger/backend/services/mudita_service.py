"""Mudita garden — local sample feed + rejoicing (spec §5.9, §16).

Single-user MVP has no real social feed, so the garden is a set of anonymous, sample
wholesome actions the user can rejoice in. Rejoicing creates a local ledger entry with no
network call, previewing future multiuser behavior.
"""

from __future__ import annotations

from typing import Any

from merit_ledger.backend.domain.models import LedgerEntry
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.services import entry_service, profile_service, tradition_service
from merit_ledger.local.config import DEFAULT_USER_ID

# Anonymous, non-identifying sample entries (spec §5.9).
_SAMPLES: list[dict[str, str]] = [
    {"sample_id": "s1", "text": "Someone practiced patience during a difficult conversation."},
    {"sample_id": "s2", "text": "Someone fed birds and dedicated the merit to all beings."},
    {"sample_id": "s3", "text": "Someone recited the Lotus Sutra today."},
    {"sample_id": "s4", "text": "Someone sat zazen even though they felt restless."},
    {"sample_id": "s5", "text": "Someone called a lonely relative."},
    {"sample_id": "s6", "text": "Someone returned to practice after a difficult day."},
    {"sample_id": "s7", "text": "Someone helped a stranger carry a heavy load."},
]


def _rejoice_verb(repo: MeritRepository, user_id: str) -> str:
    """Return the tradition-appropriate rejoicing verb (spec §5.9)."""
    settings = profile_service.get_settings(repo, user_id)
    try:
        pack = tradition_service.get_tradition(settings.tradition)
        return str(pack.get("rejoicing_language", {}).get("verb", "Rejoice"))
    except KeyError:
        return "Rejoice"


def _rejoice_points(repo: MeritRepository, user_id: str) -> int:
    """Default points awarded for rejoicing, from the active tradition pack."""
    settings = profile_service.get_settings(repo, user_id)
    try:
        pack = tradition_service.get_tradition(settings.tradition)
        return int(pack.get("point_defaults", {}).get("rejoicing", 3))
    except KeyError:
        return 3


def demo_feed(repo: MeritRepository, user_id: str = DEFAULT_USER_ID) -> dict[str, Any]:
    """Return the sample feed plus the active tradition's rejoice verb."""
    return {"verb": _rejoice_verb(repo, user_id), "entries": _SAMPLES}


def rejoice(
    repo: MeritRepository,
    *,
    text: str | None = None,
    sample_id: str | None = None,
    user_id: str = DEFAULT_USER_ID,
) -> LedgerEntry:
    """Record rejoicing in another's wholesome action as a local ledger entry.

    Either ``sample_id`` (looked up in the feed) or an explicit ``text`` may be given.
    """
    resolved = text
    if resolved is None and sample_id is not None:
        resolved = next((s["text"] for s in _SAMPLES if s["sample_id"] == sample_id), None)
    title = "Rejoiced in another's wholesome action"
    settings = profile_service.get_settings(repo, user_id)
    entry = LedgerEntry(
        user_id=user_id,
        entry_type="mudita_rejoiced",
        tradition=settings.tradition,
        title=title,
        reflection=resolved,
    )
    return entry_service.create_entry(
        repo, entry, manual_points=_rejoice_points(repo, user_id), user_id=user_id
    )
