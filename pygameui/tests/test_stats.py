"""Stats aggregation + API tests (spec §14.3)."""

from __future__ import annotations

from fastapi.testclient import TestClient

from merit_ledger.backend.domain import stats
from merit_ledger.backend.domain.models import LedgerEntry, Vow


def test_period_stats_pure() -> None:
    entries = [
        LedgerEntry(entry_type="practice_completed", points=5),
        LedgerEntry(entry_type="practice_completed", points=3),
        LedgerEntry(entry_type="reflection", points=0),
    ]
    result = stats.period_stats(entries)
    assert result.total_points == 8
    assert result.entry_count == 3
    assert result.by_type["practice_completed"] == 2


def test_points_by_key() -> None:
    entries = [
        LedgerEntry(points=5, template_id="a"),
        LedgerEntry(points=2, template_id="a"),
        LedgerEntry(points=4, template_id="b"),
    ]
    assert stats.points_by_key(entries, "template_id") == {"a": 7, "b": 4}


def test_vow_stats() -> None:
    vows = [
        Vow(name="A", status="active", streak=3),
        Vow(name="B", status="paused"),
    ]
    result = stats.vow_stats(vows)
    assert result.by_status == {"active": 1, "paused": 1}
    assert result.streaks == {"A": 3}


def test_api_stats_today_reflects_entry(client: TestClient) -> None:
    client.post("/entries", json={"entry": {"template_id": "secular.help", "title": "Helped"}})
    today = client.get("/stats/today").json()
    assert today["total_points"] == 5
    assert today["entry_count"] == 1


def test_api_stats_vows(client: TestClient) -> None:
    client.post("/vows", json={"name": "Sit", "vow_type": "positive"})
    vs = client.get("/stats/vows").json()
    assert vs["by_status"]["active"] == 1
