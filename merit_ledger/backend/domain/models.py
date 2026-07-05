"""Domain models (pydantic) mirroring the spec's JSON shapes.

These models are storage-agnostic: they know nothing about SQLite, DynamoDB, or the
single-table keys. :mod:`merit_ledger.backend.repository.item_keys` maps them to
:class:`~merit_ledger.backend.repository.base.MeritItem` rows and back.
"""

from __future__ import annotations

from typing import Literal

from pydantic import BaseModel, Field

from merit_ledger.backend.domain.ids import new_id, now_iso

# --- enums (as literals) -----------------------------------------------------

PracticeType = Literal[
    "positive",
    "negative_vow_breach",
    "repentance",
    "dedication",
    "rejoicing",
    "neutral_reflection",
]

EntryType = Literal[
    "practice_completed",
    "vow_completed",
    "vow_breached",
    "repentance_completed",
    "merit_dedicated",
    "mudita_rejoiced",
    "vow_created",
    "vow_paused",
    "vow_resumed",
    "vow_retired",
    "reflection",
]

Privacy = Literal["private", "local_demo_visible"]

PointsMode = Literal["points", "count_only", "reflection_only"]

TraditionId = Literal["zen", "chinese_mahayana", "nichiren", "pure_land", "secular", "custom"]

RepentanceCategory = Literal[
    "speech",
    "harm",
    "anger",
    "greed",
    "neglect",
    "intoxication",
    "carelessness",
    "broken_commitment",
    "general",
]

DedicationTarget = Literal[
    "all_beings",
    "person",
    "animal",
    "ancestor",
    "family",
    "sick",
    "deceased",
    "generic_group",
    "custom",
]


# --- entities ----------------------------------------------------------------


class PracticeTemplate(BaseModel):
    """A recordable practice (spec §7)."""

    template_id: str
    tradition: str
    name: str
    description: str = ""
    practice_type: PracticeType = "positive"
    default_points: int = 10
    quantity_unit: str | None = None
    default_quantity: int | None = None
    allows_dedication: bool = True
    allows_reflection: bool = True
    visibility_default: Privacy = "private"
    tags: list[str] = Field(default_factory=list)
    is_custom: bool = False


class LedgerEntry(BaseModel):
    """A single ledger entry (spec §9). Repentance/dedication reuse the optional fields."""

    entry_id: str = Field(default_factory=lambda: new_id("entry"))
    user_id: str = "local_user"
    entry_type: EntryType = "practice_completed"
    tradition: str | None = None
    template_id: str | None = None
    vow_id: str | None = None
    title: str = ""
    occurred_at: str = Field(default_factory=now_iso)
    quantity: int | None = None
    quantity_unit: str | None = None
    points: int = 0
    dedication_id: str | None = None
    reflection: str | None = None
    # repentance-specific (spec §10)
    category: RepentanceCategory | None = None
    repentance_style: str | None = None
    repair_intention: str | None = None
    linked_vow_id: str | None = None
    privacy: Privacy = "private"
    tags: list[str] = Field(default_factory=list)
    created_at: str = Field(default_factory=now_iso)
    updated_at: str = Field(default_factory=now_iso)


class Profile(BaseModel):
    """Local single-user profile."""

    user_id: str = "local_user"
    name: str = "Practitioner"
    tradition: TraditionId = "secular"
    created_at: str = Field(default_factory=now_iso)
    updated_at: str = Field(default_factory=now_iso)


class Settings(BaseModel):
    """User settings (spec §17)."""

    user_id: str = "local_user"
    tradition: TraditionId = "secular"
    points_mode: PointsMode = "points"
    negative_points_enabled: bool = False
    dedication_reduces_balance: bool = False
    reduced_motion: bool = False
    high_contrast: bool = False
    sound_enabled: bool = True
    font_scale: float = 1.0
    hide_reflections_in_stats: bool = False
    confirm_before_export: bool = True
    updated_at: str = Field(default_factory=now_iso)
