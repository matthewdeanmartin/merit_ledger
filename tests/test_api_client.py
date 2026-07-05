"""API client tests, driven against an in-process TestClient app."""

from __future__ import annotations

import pytest
from fastapi.testclient import TestClient

from merit_ledger.backend.main import create_app
from merit_ledger.backend.repository.memory_repo import InMemoryMeritRepository
from merit_ledger.frontend.api_client import ApiClient


@pytest.fixture
def api() -> ApiClient:
    """An ApiClient wired to a TestClient (httpx-compatible) over a fresh app."""
    app = create_app(repo=InMemoryMeritRepository())
    return ApiClient(client=TestClient(app))


def test_get_default_profile(api: ApiClient) -> None:
    assert api.get_profile()["user_id"] == "local_user"


def test_set_tradition_and_templates(api: ApiClient) -> None:
    api.set_tradition("zen")
    ids = {t["template_id"] for t in api.list_templates()}
    assert "zen.zazen" in ids


def test_create_entry_and_stats(api: ApiClient) -> None:
    api.set_tradition("secular")
    entry = api.create_entry({"template_id": "secular.help", "title": "Helped"})
    assert entry["points"] == 5
    assert api.stats_today()["total_points"] == 5


def test_list_vows_by_status(api: ApiClient) -> None:
    api._post("/vows", {"name": "Sit", "vow_type": "positive"})
    assert len(api.list_vows(status="active")) == 1
