"""Dedication service (spec §5.8, §11, §18).

Dedications record the direction of merit; by default they do not reduce any balance
(the MVP has no persistent balance entity). Preset targets and language come from the
active tradition pack, with secular replacements applied for secular mode.
"""

from __future__ import annotations

from typing import Any

from merit_ledger.backend.domain.ids import now_iso
from merit_ledger.backend.domain.models import Dedication
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.repository.item_keys import (
    DEDICATION_SK_PREFIX,
    to_dedication_item,
    dedication_from_item,
    user_pk,
)
from merit_ledger.backend.services import entry_service, profile_service, tradition_service
from merit_ledger.local.config import DEFAULT_USER_ID

# Generic preset targets offered in every tradition (spec §5.8 dedication targets).
_PRESET_TARGETS: list[dict[str, str]] = [
    {"target_type": "all_beings", "target_name": "All sentient beings"},
    {"target_type": "family", "target_name": "Family"},
    {"target_type": "ancestor", "target_name": "Ancestors"},
    {"target_type": "sick", "target_name": "The sick"},
    {"target_type": "deceased", "target_name": "The dead"},
    {"target_type": "animal", "target_name": "Animals"},
    {"target_type": "generic_group", "target_name": "Beings suffering from war"},
    {"target_type": "generic_group", "target_name": "Beings suffering from hunger"},
]

# Secular mode relabels "all sentient beings" (spec §18).
_SECULAR_TARGETS: list[dict[str, str]] = [
    {"target_type": "generic_group", "target_name": "Others"},
    {"target_type": "family", "target_name": "Family"},
    {"target_type": "sick", "target_name": "People who are unwell"},
    {"target_type": "animal", "target_name": "Animals"},
    {"target_type": "generic_group", "target_name": "My community"},
]


def presets(repo: MeritRepository, user_id: str = DEFAULT_USER_ID) -> dict[str, Any]:
    """Return preset dedication targets + default text for the active tradition."""
    settings = profile_service.get_settings(repo, user_id)
    targets = _SECULAR_TARGETS if settings.tradition == "secular" else _PRESET_TARGETS
    try:
        pack = tradition_service.get_tradition(settings.tradition)
        default_text = pack.get("dedication_language", {}).get("default_text", "")
    except KeyError:
        default_text = ""
    return {"targets": targets, "default_text": default_text}


def create_dedication(
    repo: MeritRepository, dedication: Dedication, user_id: str = DEFAULT_USER_ID
) -> Dedication:
    """Persist a dedication and, if it names a source entry, link it back on that entry.

    Records only — it never subtracts from a balance (spec §5.8 default). The
    ``settings.dedication_reduces_balance`` flag is honored for storage but there is no
    running balance to reduce in the MVP; see the design note in sprints/sprint4.md.
    """
    dedication.user_id = user_id
    dedication.created_at = dedication.created_at or now_iso()
    repo.put_item(to_dedication_item(dedication))

    if dedication.source_entry_id:
        entry = entry_service.find_entry(repo, dedication.source_entry_id, user_id)
        if entry is not None:
            entry.dedication_id = dedication.dedication_id
            entry_service.update_entry(repo, entry)
    return dedication


def list_dedications(repo: MeritRepository, user_id: str = DEFAULT_USER_ID) -> list[Dedication]:
    """List the user's dedications, oldest first."""
    items = repo.query_pk(user_pk(user_id), DEDICATION_SK_PREFIX)
    return [dedication_from_item(i) for i in items]
