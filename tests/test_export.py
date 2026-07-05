"""JSON export tests (spec §19.1)."""

from __future__ import annotations

from fastapi.testclient import TestClient


def test_export_snapshot_shape(client: TestClient) -> None:
    client.put("/settings/tradition", json={"tradition": "pure_land"})
    client.post("/entries", json={"entry": {"template_id": "pureland.nembutsu", "title": "Nembutsu"}})

    snap = client.get("/export/json").json()
    assert snap["version"] == 1
    assert snap["tradition"] == "pure_land"
    assert "profile" in snap and "settings" in snap
    assert len(snap["entries"]) == 1
    assert snap["entries"][0]["title"] == "Nembutsu"
    # buckets exist even when empty
    assert snap["vows"] == []
    assert snap["dedications"] == []
