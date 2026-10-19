"""Single-table key construction and entity<->MeritItem mapping (spec §13.3).

Services build keys *only* through this module so the single-table discipline lives in
one place. Each ``to_*_item`` returns a :class:`MeritItem`; each ``*_from_item`` rebuilds
the domain model from a stored row.
"""

from __future__ import annotations

from merit_ledger.backend.domain.models import (
    Dedication,
    LedgerEntry,
    PracticeTemplate,
    Profile,
    Settings,
    Vow,
)
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


# --- vows --------------------------------------------------------------------

VOW_SK_PREFIX = "VOW#"


def vow_sk(vow_id: str) -> str:
    """Sort key for a vow: ``VOW#<vow_id>`` (spec §13.3)."""
    return f"{VOW_SK_PREFIX}{vow_id}"


def vow_status_gsi1pk(user_id: str, status: str) -> str:
    """GSI1 partition for vows in a given status (spec §13.3 'Vows by status')."""
    return f"{user_pk(user_id)}#VOW_STATUS#{status}"


def to_vow_item(vow: Vow) -> MeritItem:
    """Serialize a Vow with a GSI1 status key so status queries work."""
    sk = vow_sk(vow.vow_id)
    return MeritItem(
        pk=user_pk(vow.user_id),
        sk=sk,
        entity_type="VOW",
        gsi1pk=vow_status_gsi1pk(vow.user_id, vow.status),
        gsi1sk=sk,
        item=vow.model_dump(),
        created_at=vow.created_at,
        updated_at=vow.updated_at,
    )


def vow_from_item(item: MeritItem) -> Vow:
    """Rebuild a Vow from a stored item."""
    return Vow.model_validate(item.item)


# --- dedications -------------------------------------------------------------

DEDICATION_SK_PREFIX = "DEDICATION#"


def dedication_sk(dedication: Dedication) -> str:
    """Sort key: ``DEDICATION#<created_at>#<dedication_id>`` (spec §13.3)."""
    return f"{DEDICATION_SK_PREFIX}{dedication.created_at}#{dedication.dedication_id}"


def to_dedication_item(dedication: Dedication) -> MeritItem:
    """Serialize a Dedication."""
    return MeritItem(
        pk=user_pk(dedication.user_id),
        sk=dedication_sk(dedication),
        entity_type="DEDICATION",
        item=dedication.model_dump(),
        created_at=dedication.created_at,
        updated_at=dedication.created_at,
    )


def dedication_from_item(item: MeritItem) -> Dedication:
    """Rebuild a Dedication from a stored item."""
    return Dedication.model_validate(item.item)
