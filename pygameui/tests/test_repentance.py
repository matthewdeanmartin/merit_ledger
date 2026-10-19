"""Repentance service + API tests (spec §5.7, §10)."""

from __future__ import annotations

from fastapi.testclient import TestClient

from merit_ledger.backend.repository.memory_repo import InMemoryMeritRepository
from merit_ledger.backend.services import repentance_service


def test_categories_include_expected() -> None:
    cats = repentance_service.categories()
    assert "speech" in cats and "general" in cats and len(cats) == 9


def test_create_repentance_awards_points() -> None:
    repo = InMemoryMeritRepository()
    entry = repentance_service.create_repentance(
        repo, category="speech", reflection="noticed impatience", repair_intention="pause first"
    )
    assert entry.entry_type == "repentance_completed"
    assert entry.category == "speech"
    assert entry.repentance_style == "category"
    assert entry.points == 3  # secular default repentance points


def test_api_categories_include_privacy_reminder(client: TestClient) -> None:
    body = client.get("/repentance/categories").json()
    assert "speech" in body["categories"]
    assert "secrets" in body["privacy_reminder"].lower()


def test_api_create_repentance(client: TestClient) -> None:
    resp = client.post("/repentance", json={"category": "anger", "reflection": "felt heat, breathed"})
    assert resp.status_code == 200
    assert resp.json()["category"] == "anger"
