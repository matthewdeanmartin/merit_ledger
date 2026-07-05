"""Single-table key construction and entity<->MeritItem mapping (spec §13.3).

Services build keys *only* through this module so the single-table discipline lives in
one place. Each ``to_*_item`` returns a :class:`MeritItem`; each ``*_from_item`` rebuilds
the domain model from a stored row.
"""

from __future__ import annotations

from merit_ledger.backend.domain.models import LedgerEntry, PracticeTemplate, Profile, Settings
from merit_ledger.backend.repository.base import MeritItem


def user_pk(user_id: str) -> str:
    """Partition key for everything owned by a user."""
    return f"USER#{user_id}"


# --- profile -----------------------------------------------------------------


def to_profile_item(profile: Profile) -> MeritItem:
    """Serialize a Profile to its single-table item at SK=PROFILE."""
    return MeritItem(
        pk=user_pk(profile.user_id),
        sk="PROFILE",
        entity_type="PROFILE",
        item=profile.model_dump(),
        created_at=profile.created_at,
        updated_at=profile.updated_at,
    )


def profile_from_item(item: MeritItem) -> Profile:
    """Rebuild a Profile from a stored item."""
    return Profile.model_validate(item.item)


# --- settings ----------------------------------------------------------------


def to_settings_item(settings: Settings) -> MeritItem:
    """Serialize Settings to its single-table item at SK=SETTINGS."""
    return MeritItem(
        pk=user_pk(settings.user_id),
        sk="SETTINGS",
        entity_type="SETTINGS",
        item=settings.model_dump(),
        created_at=settings.updated_at,
        updated_at=settings.updated_at,
    )


def settings_from_item(item: MeritItem) -> Settings:
    """Rebuild Settings from a stored item."""
    return Settings.model_validate(item.item)


# --- templates ---------------------------------------------------------------


def to_template_item(template: PracticeTemplate, user_id: str = "local_user") -> MeritItem:
    """Serialize a custom PracticeTemplate to SK=TEMPLATE#<id>."""
    return MeritItem(
        pk=user_pk(user_id),
        sk=f"TEMPLATE#{template.template_id}",
        entity_type="TEMPLATE",
        item=template.model_dump(),
        created_at="",
        updated_at="",
    )


def template_from_item(item: MeritItem) -> PracticeTemplate:
    """Rebuild a PracticeTemplate from a stored item."""
    return PracticeTemplate.model_validate(item.item)


TEMPLATE_SK_PREFIX = "TEMPLATE#"


# --- ledger entries ----------------------------------------------------------

ENTRY_SK_PREFIX = "ENTRY#"


def entry_sk(entry: LedgerEntry) -> str:
    """Sort key for a ledger entry: ``ENTRY#<occurred_at>#<entry_id>`` (spec §13.3)."""
    return f"ENTRY#{entry.occurred_at}#{entry.entry_id}"


def to_entry_item(entry: LedgerEntry) -> MeritItem:
    """Serialize a LedgerEntry with GSI1 (by type) and GSI2 (by tradition) keys."""
    uid = entry.user_id
    sk = entry_sk(entry)
    gsi1pk = f"{user_pk(uid)}#ENTRY_TYPE#{entry.entry_type}"
    gsi2pk = f"{user_pk(uid)}#TRADITION#{entry.tradition}" if entry.tradition else None
    return MeritItem(
        pk=user_pk(uid),
        sk=sk,
        entity_type="ENTRY",
        gsi1pk=gsi1pk,
        gsi1sk=sk,
        gsi2pk=gsi2pk,
        gsi2sk=sk if gsi2pk else None,
        item=entry.model_dump(),
        created_at=entry.created_at,
        updated_at=entry.updated_at,
    )


def entry_from_item(item: MeritItem) -> LedgerEntry:
    """Rebuild a LedgerEntry from a stored item."""
    return LedgerEntry.model_validate(item.item)


def entry_type_gsi1pk(user_id: str, entry_type: str) -> str:
    """GSI1 partition for entries of a given type."""
    return f"{user_pk(user_id)}#ENTRY_TYPE#{entry_type}"
