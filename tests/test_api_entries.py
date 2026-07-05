"""Entry API tests (spec §14.3)."""

from __future__ import annotations

from fastapi.testclient import TestClient


def _make_entry(client: TestClient, **entry_fields) -> dict:
    body = {"entry": {"entry_type": "practice_completed", "title": "Test", **entry_fields}}
    resp = client.post("/entries", json=body)
    assert resp.status_code == 200, resp.text
    return resp.json()


def test_create_entry_defaults_points_from_template(client: TestClient) -> None:
    # secular is the default tradition; secular.help defaults to 5 points
    entry = _make_entry(client, template_id="secular.help", title="Helped someone")
    assert entry["points"] == 5
    assert entry["entry_id"]


def test_create_entry_manual_points(client: TestClient) -> None:
    body = {
        "entry": {"entry_type": "practice_completed", "title": "Custom"},
        "manual_points": 7,
    }
    resp = client.post("/entries", json=body)
    assert resp.json()["points"] == 7


def test_count_only_mode_zero_points(client: TestClient) -> None:
    settings = client.get("/settings").json()
    settings["points_mode"] = "count_only"
    client.put("/settings", json=settings)
    entry = _make_entry(client, template_id="secular.help")
    assert entry["points"] == 0


def test_list_and_get_entry(client: TestClient) -> None:
    created = _make_entry(client, template_id="secular.help")
    listed = client.get("/entries").json()
    assert any(e["entry_id"] == created["entry_id"] for e in listed)
    got = client.get(f"/entries/{created['entry_id']}")
    assert got.status_code == 200
    assert got.json()["entry_id"] == created["entry_id"]


def test_list_filter_by_type(client: TestClient) -> None:
    _make_entry(client, entry_type="practice_completed", template_id="secular.help")
    _make_entry(client, entry_type="reflection", title="thought")
    reflections = client.get("/entries", params={"entry_type": "reflection"}).json()
    assert all(e["entry_type"] == "reflection" for e in reflections)
    assert len(reflections) == 1


def test_delete_entry(client: TestClient) -> None:
    created = _make_entry(client, template_id="secular.help")
    assert client.delete(f"/entries/{created['entry_id']}").status_code == 200
    assert client.get(f"/entries/{created['entry_id']}").status_code == 404
