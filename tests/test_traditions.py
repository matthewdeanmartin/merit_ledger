"""Tradition pack loading tests (spec §6)."""

from __future__ import annotations

import pytest

from merit_ledger.backend.services import tradition_service


def test_lists_five_traditions() -> None:
    ids = {t["tradition_id"] for t in tradition_service.list_traditions()}
    assert ids == {"zen", "chinese_mahayana", "nichiren", "pure_land", "secular"}


def test_get_pure_land_pack() -> None:
    pack = tradition_service.get_tradition("pure_land")
    assert pack["display_name"] == "Pure Land"
    assert any(t["template_id"] == "pureland.nembutsu" for t in pack["suggested_practices"])


def test_secular_relabels_merit() -> None:
    pack = tradition_service.get_tradition("secular")
    assert pack["labels"]["merit"] == "Practice points"
    assert pack["labels"]["repentance"] == "Repair"


def test_unknown_tradition_raises() -> None:
    with pytest.raises(KeyError):
        tradition_service.get_tradition("jedi")


def test_suggested_templates_are_models() -> None:
    templates = tradition_service.suggested_templates("zen")
    assert templates
    zazen = tradition_service.find_template("zen", "zen.zazen")
    assert zazen is not None and zazen.default_points == 5
