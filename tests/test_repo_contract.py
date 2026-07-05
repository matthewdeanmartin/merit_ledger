"""Shared contract tests run against every repository implementation (spec §23.2)."""

from __future__ import annotations

from collections.abc import Iterator

import pytest

from merit_ledger.backend.repository.base import MeritItem, MeritRepository
from merit_ledger.backend.repository.memory_repo import InMemoryMeritRepository
from merit_ledger.backend.repository.sqlite_repo import SqliteMeritRepository


@pytest.fixture(params=["memory", "sqlite"])
def repo(request: pytest.FixtureRequest, tmp_path) -> Iterator[MeritRepository]:
    """Yield each repository implementation in turn."""
    if request.param == "memory":
        yield InMemoryMeritRepository()
    else:
        r = SqliteMeritRepository(tmp_path / "test.sqlite3")
        yield r
        r.close()


def _item(pk: str, sk: str, **kw) -> MeritItem:
    """Build a MeritItem with sensible defaults for tests."""
    return MeritItem(
        pk=pk,
        sk=sk,
        entity_type=kw.pop("entity_type", "TEST"),
        created_at="2026-07-05T00:00:00Z",
        updated_at="2026-07-05T00:00:00Z",
        **kw,
    )


def test_put_get_roundtrip(repo: MeritRepository) -> None:
    repo.put_item(_item("USER#u", "PROFILE", item={"name": "Ananda"}))
    got = repo.get_item("USER#u", "PROFILE")
    assert got is not None
    assert got.item == {"name": "Ananda"}


def test_get_missing_returns_none(repo: MeritRepository) -> None:
    assert repo.get_item("USER#u", "NOPE") is None


def test_put_replaces(repo: MeritRepository) -> None:
    repo.put_item(_item("USER#u", "PROFILE", item={"name": "old"}))
    repo.put_item(_item("USER#u", "PROFILE", item={"name": "new"}))
    got = repo.get_item("USER#u", "PROFILE")
    assert got is not None and got.item["name"] == "new"


def test_query_pk_prefix_and_sort(repo: MeritRepository) -> None:
    repo.put_item(_item("USER#u", "ENTRY#2026-07-05#b"))
    repo.put_item(_item("USER#u", "ENTRY#2026-07-05#a"))
    repo.put_item(_item("USER#u", "VOW#x"))
    entries = repo.query_pk("USER#u", "ENTRY#")
    assert [e.sk for e in entries] == ["ENTRY#2026-07-05#a", "ENTRY#2026-07-05#b"]


def test_query_pk_all(repo: MeritRepository) -> None:
    repo.put_item(_item("USER#u", "A"))
    repo.put_item(_item("USER#u", "B"))
    repo.put_item(_item("USER#other", "A"))
    assert len(repo.query_pk("USER#u")) == 2


def test_query_gsi1(repo: MeritRepository) -> None:
    repo.put_item(_item("USER#u", "VOW#1", gsi1pk="USER#u#VOW_STATUS#active", gsi1sk="VOW#1"))
    repo.put_item(_item("USER#u", "VOW#2", gsi1pk="USER#u#VOW_STATUS#paused", gsi1sk="VOW#2"))
    active = repo.query_gsi1("USER#u#VOW_STATUS#active")
    assert [i.sk for i in active] == ["VOW#1"]


def test_query_gsi2(repo: MeritRepository) -> None:
    repo.put_item(_item("USER#u", "E#1", gsi2pk="USER#u#TRADITION#zen", gsi2sk="ENTRY#1"))
    repo.put_item(_item("USER#u", "E#2", gsi2pk="USER#u#TRADITION#pure_land", gsi2sk="ENTRY#2"))
    zen = repo.query_gsi2("USER#u#TRADITION#zen")
    assert [i.sk for i in zen] == ["E#1"]


def test_delete(repo: MeritRepository) -> None:
    repo.put_item(_item("USER#u", "X"))
    repo.delete_item("USER#u", "X")
    assert repo.get_item("USER#u", "X") is None
    repo.delete_item("USER#u", "X")  # idempotent, no error


def test_scan_all(repo: MeritRepository) -> None:
    repo.put_item(_item("USER#u", "B"))
    repo.put_item(_item("USER#a", "A"))
    scanned = repo.scan_all()
    assert [(i.pk, i.sk) for i in scanned] == [("USER#a", "A"), ("USER#u", "B")]


def test_clear(repo: MeritRepository) -> None:
    repo.put_item(_item("USER#u", "A"))
    repo.put_item(_item("USER#u", "B"))
    removed = repo.clear()
    assert removed == 2
    assert repo.scan_all() == []
    assert repo.clear() == 0  # idempotent on an empty store
