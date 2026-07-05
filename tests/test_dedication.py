"""Dedication service + API tests (spec §5.8, §11, §18)."""

from __future__ import annotations

from fastapi.testclient import TestClient


def test_create_dedication_links_to_entry(client: TestClient) -> None:
    entry = client.post(
        "/entries", json={"entry": {"template_id": "secular.help", "title": "Helped"}}
    ).json()
    ded = client.post(
        "/dedications",
        json={
            "source_entry_id": entry["entry_id"],
            "target_type": "all_beings",
            "target_name": "All beings",
            "dedication_text": "May this help others.",
            "points_dedicated": 5,
        },
    ).json()
    # entry now references the dedication
    updated = client.get(f"/entries/{entry['entry_id']}").json()
    assert updated["dedication_id"] == ded["dedication_id"]


def test_list_dedications(client: TestClient) -> None:
    client.post("/dedications", json={"target_name": "Family", "points_dedicated": 2})
    listed = client.get("/dedications").json()
    assert len(listed) == 1
    assert listed[0]["target_name"] == "Family"


def test_secular_presets_relabel(client: TestClient) -> None:
    client.put("/settings/tradition", json={"tradition": "secular"})
    presets = client.get("/dedications/presets").json()
    names = {t["target_name"] for t in presets["targets"]}
    assert "Others" in names  # secular relabel of 'all sentient beings'


def test_buddhist_presets_include_all_beings(client: TestClient) -> None:
    client.put("/settings/tradition", json={"tradition": "pure_land"})
    presets = client.get("/dedications/presets").json()
    names = {t["target_name"] for t in presets["targets"]}
    assert "All sentient beings" in names
    assert presets["default_text"]
