"""Settings + clear-local-data endpoint tests (spec §17.5)."""

from __future__ import annotations

from fastapi.testclient import TestClient

from merit_ledger.backend.main import create_app
from merit_ledger.backend.repository.memory_repo import InMemoryMeritRepository
from merit_ledger.frontend.api_client import ApiClient


def test_clear_requires_confirmation(client: TestClient) -> None:
    resp = client.post("/settings/clear", json={"confirm": False})
    assert resp.status_code == 400


def test_clear_wipes_all_data(client: TestClient) -> None:
    client.put("/settings/tradition", json={"tradition": "zen"})
    client.post("/entries", json={"entry": {"template_id": "zen.zazen", "title": "Zazen"}})
    client.post("/vows", json={"name": "Sit daily", "vow_type": "positive"})

    before = client.get("/export/json").json()
    assert before["entries"] and before["vows"]

    resp = client.post("/settings/clear", json={"confirm": True})
    assert resp.status_code == 200
    assert resp.json()["deleted"] > 0

    after = client.get("/export/json").json()
    assert after["entries"] == []
    assert after["vows"] == []
    # factory reset: settings revert to defaults (onboarding will show again)
    assert after["settings"]["onboarded"] is False
    assert after["settings"]["tradition"] == "secular"


def test_clear_user_data_keeps_profile_and_settings(client: TestClient) -> None:
    client.put("/settings/tradition", json={"tradition": "zen"})
    settings = client.get("/settings").json()
    settings["onboarded"] = True
    client.put("/settings", json=settings)
    client.post("/entries", json={"entry": {"template_id": "zen.zazen", "title": "Zazen"}})
    client.post("/vows", json={"name": "Sit daily", "vow_type": "positive"})

    resp = client.post("/settings/clear", json={"confirm": True, "scope": "user_data"})
    assert resp.status_code == 200
    assert resp.json()["deleted"] > 0

    after = client.get("/export/json").json()
    # ledger wiped...
    assert after["entries"] == []
    assert after["vows"] == []
    # ...but identity/config preserved (no re-onboarding)
    assert after["settings"]["onboarded"] is True
    assert after["settings"]["tradition"] == "zen"
    assert after["profile"]["tradition"] == "zen"


def test_clear_user_data_still_requires_confirmation(client: TestClient) -> None:
    resp = client.post("/settings/clear", json={"confirm": False, "scope": "user_data"})
    assert resp.status_code == 400


def test_api_client_clear_user_data_scope() -> None:
    api = ApiClient(client=TestClient(create_app(repo=InMemoryMeritRepository())))
    api.set_tradition("zen")
    s = api.get_settings()
    s["onboarded"] = True
    api.put_settings(s)
    api.create_entry({"template_id": "zen.zazen", "title": "Zazen"})
    result = api.clear_data(scope="user_data")
    assert result["deleted"] > 0
    assert api.list_entries() == []
    assert api.get_settings()["onboarded"] is True  # kept


def test_api_client_clear_data() -> None:
    api = ApiClient(client=TestClient(create_app(repo=InMemoryMeritRepository())))
    api.set_tradition("pure_land")
    api.create_entry({"template_id": "pureland.nembutsu", "title": "Nembutsu"})
    result = api.clear_data()
    assert result["deleted"] > 0
    assert api.list_entries() == []
