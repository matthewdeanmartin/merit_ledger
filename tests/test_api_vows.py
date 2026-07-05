"""Vow API tests (spec §14.3)."""

from __future__ import annotations

from fastapi.testclient import TestClient


def _create(client: TestClient, **fields) -> dict:
    body = {"name": "A vow", "vow_type": "positive", **fields}
    resp = client.post("/vows", json=body)
    assert resp.status_code == 200, resp.text
    return resp.json()


def test_create_and_get(client: TestClient) -> None:
    vow = _create(client, name="Sit daily")
    got = client.get(f"/vows/{vow['vow_id']}")
    assert got.status_code == 200
    assert got.json()["name"] == "Sit daily"
    assert got.json()["status"] == "active"


def test_complete_positive(client: TestClient) -> None:
    vow = _create(client, name="Sit daily", default_points=6)
    resp = client.post(f"/vows/{vow['vow_id']}/complete", json={"reflection": "calm"})
    assert resp.status_code == 200
    data = resp.json()
    assert data["vow"]["streak"] == 1
    assert data["entry"]["entry_type"] == "vow_completed"
    assert data["entry"]["points"] == 6


def test_breach_negative(client: TestClient) -> None:
    vow = _create(client, name="No harsh speech", vow_type="negative")
    resp = client.post(
        f"/vows/{vow['vow_id']}/breach",
        json={"category": "speech", "repair_intention": "pause before replying"},
    )
    assert resp.status_code == 200
    assert resp.json()["vow"]["status"] == "repair_in_progress"
    assert resp.json()["entry"]["repair_intention"] == "pause before replying"


def test_pause_resume(client: TestClient) -> None:
    vow = _create(client, name="Study")
    p = client.post(f"/vows/{vow['vow_id']}/pause", json={"reason": "illness"})
    assert p.json()["vow"]["status"] == "paused"
    r = client.post(f"/vows/{vow['vow_id']}/resume", json={"new_points": 12})
    assert r.json()["vow"]["status"] == "active"
    assert r.json()["vow"]["default_points"] == 12


def test_retire(client: TestClient) -> None:
    vow = _create(client, name="Old vow")
    resp = client.post(f"/vows/{vow['vow_id']}/retire")
    assert resp.json()["vow"]["status"] == "retired"


def test_illegal_transition_returns_409(client: TestClient) -> None:
    vow = _create(client, name="Study")
    client.post(f"/vows/{vow['vow_id']}/retire")
    # cannot pause a retired vow
    resp = client.post(f"/vows/{vow['vow_id']}/pause", json={})
    assert resp.status_code == 409


def test_list_by_status(client: TestClient) -> None:
    a = _create(client, name="A")
    _create(client, name="B")
    client.post(f"/vows/{a['vow_id']}/pause", json={"reason": "rest"})
    paused = client.get("/vows", params={"status": "paused"}).json()
    assert {v["name"] for v in paused} == {"A"}


def test_vows_appear_in_export(client: TestClient) -> None:
    _create(client, name="Exported vow")
    snap = client.get("/export/json").json()
    assert any(v["name"] == "Exported vow" for v in snap["vows"])


def test_unknown_vow_404(client: TestClient) -> None:
    assert client.get("/vows/nope").status_code == 404
