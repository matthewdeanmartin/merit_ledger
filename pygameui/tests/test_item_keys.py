"""Key construction + roundtrip tests (spec §13.3)."""

from __future__ import annotations

from merit_ledger.backend.domain.models import LedgerEntry, PracticeTemplate, Profile
from merit_ledger.backend.repository.item_keys import (
    entry_from_item,
    entry_sk,
    profile_from_item,
    template_from_item,
    to_entry_item,
    to_profile_item,
    to_template_item,
)


def test_profile_key_and_roundtrip() -> None:
    p = Profile(user_id="local_user", name="Ananda")
    item = to_profile_item(p)
    assert item.pk == "USER#local_user"
    assert item.sk == "PROFILE"
    assert profile_from_item(item).name == "Ananda"


def test_template_key() -> None:
    t = PracticeTemplate(template_id="zen.zazen", tradition="zen", name="Zazen")
    item = to_template_item(t, "local_user")
    assert item.sk == "TEMPLATE#zen.zazen"
    assert template_from_item(item).name == "Zazen"


def test_entry_keys_and_gsis() -> None:
    e = LedgerEntry(
        entry_id="entry_x",
        occurred_at="2026-07-05T09:00:00Z",
        entry_type="practice_completed",
        tradition="pure_land",
    )
    item = to_entry_item(e)
    assert item.pk == "USER#local_user"
    assert item.sk == "ENTRY#2026-07-05T09:00:00Z#entry_x"
    assert item.sk == entry_sk(e)
    assert item.gsi1pk == "USER#local_user#ENTRY_TYPE#practice_completed"
    assert item.gsi2pk == "USER#local_user#TRADITION#pure_land"
    assert entry_from_item(item).entry_id == "entry_x"


def test_entry_without_tradition_has_no_gsi2() -> None:
    e = LedgerEntry(entry_id="e2", occurred_at="2026-07-05T09:00:00Z", tradition=None)
    item = to_entry_item(e)
    assert item.gsi2pk is None
    assert item.gsi2sk is None
