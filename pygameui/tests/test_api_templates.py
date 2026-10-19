"""Template + tradition API tests."""

from __future__ import annotations

from fastapi.testclient import TestClient


def test_list_traditions(client: TestClient) -> None:
    ids = {t["tradition_id"] for t in client.get("/traditions").json()}
    assert "zen" in ids and "secular" in ids


def test_set_tradition_changes_templates(client: TestClient) -> None:
    client.put("/settings/tradition", json={"tradition": "zen"})
    ids = {t["template_id"] for t in client.get("/templates").json()}
    assert "zen.zazen" in ids


def test_custom_template_crud(client: TestClient) -> None:
    tmpl = {
        "template_id": "custom.tea",
        "tradition": "secular",
        "name": "Made tea mindfully",
        "default_points": 4,
    }
    assert client.post("/templates", json=tmpl).status_code == 200
    got = client.get("/templates/custom.tea").json()
    assert got["name"] == "Made tea mindfully"
    assert got["is_custom"] is True
    assert client.delete("/templates/custom.tea").status_code == 200
    assert client.get("/templates/custom.tea").status_code == 404


def test_get_unknown_template_404(client: TestClient) -> None:
    assert client.get("/templates/nope.nope").status_code == 404
