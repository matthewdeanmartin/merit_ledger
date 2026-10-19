"""Profile and settings persistence (single items per user)."""

from __future__ import annotations

from merit_ledger.backend.domain.ids import now_iso
from merit_ledger.backend.domain.models import Profile, Settings
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.repository.item_keys import (
    profile_from_item,
    settings_from_item,
    to_profile_item,
    to_settings_item,
    user_pk,
)
from merit_ledger.local.config import DEFAULT_USER_ID


def get_profile(repo: MeritRepository, user_id: str = DEFAULT_USER_ID) -> Profile:
    """Return the stored profile, or a default one if none exists yet."""
    item = repo.get_item(user_pk(user_id), "PROFILE")
    return profile_from_item(item) if item else Profile(user_id=user_id)


def save_profile(repo: MeritRepository, profile: Profile) -> Profile:
    """Persist ``profile`` (bumping updated_at) and return it."""
    profile.updated_at = now_iso()
    repo.put_item(to_profile_item(profile))
    return profile


def get_settings(repo: MeritRepository, user_id: str = DEFAULT_USER_ID) -> Settings:
    """Return the stored settings, or defaults if none exist yet."""
    item = repo.get_item(user_pk(user_id), "SETTINGS")
    return settings_from_item(item) if item else Settings(user_id=user_id)


def save_settings(repo: MeritRepository, settings: Settings) -> Settings:
    """Persist ``settings`` (bumping updated_at) and return it."""
    settings.updated_at = now_iso()
    repo.put_item(to_settings_item(settings))
    return settings


def clear_data(repo: MeritRepository) -> int:
    """Delete all local data (spec §17.5).

    This is a factory reset: profile, settings, templates, vows, entries, and dedications
    are all removed. Onboarding will run again on next launch since the ``onboarded`` flag
    is gone. Returns the number of items deleted.
    """
    return repo.clear()


# Entity types considered "user data" — the practice ledger, not identity/config.
# PROFILE and SETTINGS are deliberately preserved by clear_user_data.
_USER_DATA_ENTITY_TYPES = frozenset({"ENTRY", "VOW", "DEDICATION", "TEMPLATE"})


def clear_user_data(repo: MeritRepository) -> int:
    """Delete practice data but keep profile + settings (handy for manual testing).

    Removes ledger entries, vows, dedications, and custom templates, leaving the profile
    and settings (including the ``onboarded`` flag and chosen tradition) intact — so the
    app stays configured and does not re-run onboarding. Returns the number deleted.
    """
    deleted = 0
    for item in repo.scan_all():
        if item.entity_type in _USER_DATA_ENTITY_TYPES:
            repo.delete_item(item.pk, item.sk)
            deleted += 1
    return deleted
