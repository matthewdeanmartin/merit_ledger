"""Mudita garden tests (spec §5.9, §16)."""

from __future__ import annotations

from fastapi.testclient import TestClient


def test_demo_feed_has_samples_and_verb(client: TestClient) -> None:
    feed = client.get("/mudita/demo-feed").json()
    assert feed["entries"]
    assert feed["verb"]  # tradition rejoice verb


def test_rejoice_creates_entry(client: TestClient) -> None:
    entry = client.post("/mudita/rejoice", json={"sample_id": "s1"}).json()
    assert entry["entry_type"] == "mudita_rejoiced"
    assert entry["points"] == 3  # secular rejoicing default
    assert "patience" in entry["reflection"]


def test_rejoice_with_explicit_text(client: TestClient) -> None:
    entry = client.post("/mudita/rejoice", json={"text": "Someone was kind."}).json()
    assert entry["reflection"] == "Someone was kind."


def test_nichiren_verb_and_no_network(client: TestClient) -> None:
    client.put("/settings/tradition", json={"tradition": "nichiren"})
    feed = client.get("/mudita/demo-feed").json()
    assert feed["verb"] == "Rejoice"
