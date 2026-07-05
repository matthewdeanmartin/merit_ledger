"""Markdown export + JSON import tests (spec §19)."""

from __future__ import annotations

from fastapi.testclient import TestClient


def test_markdown_export_renders(client: TestClient) -> None:
    client.put("/settings/tradition", json={"tradition": "pure_land"})
    client.post(
        "/entries",
        json={"entry": {"template_id": "pureland.nembutsu", "title": "Nembutsu recitation",
                        "quantity": 108, "reflection": "Felt scattered but returned."}},
    )
    md = client.get("/export/markdown").text
    assert "# Merit Ledger Export" in md
    assert "Nembutsu recitation" in md
    assert "Review before sharing" in md  # privacy warning
    assert "Reflection:" in md


def test_import_roundtrips_snapshot(client: TestClient) -> None:
    client.put("/settings/tradition", json={"tradition": "zen"})
    client.post("/entries", json={"entry": {"template_id": "zen.zazen", "title": "Zazen"}})
    client.post("/vows", json={"name": "Sit daily", "vow_type": "positive"})
    snapshot = client.get("/export/json").json()

    # import into a fresh client (fresh in-memory repo)
    from merit_ledger.backend.main import create_app
    from merit_ledger.backend.repository.memory_repo import InMemoryMeritRepository

    fresh = TestClient(create_app(repo=InMemoryMeritRepository()))
    counts = fresh.post("/import/json", json=snapshot).json()
    # 2 entries: the zazen practice + the vow_created lifecycle entry
    assert counts["entries"] == 2
    assert counts["vows"] == 1

    reexport = fresh.get("/export/json").json()
    titles = {e["title"] for e in reexport["entries"]}
    assert "Zazen" in titles
    assert reexport["vows"][0]["name"] == "Sit daily"


def test_import_rejects_bad_version(client: TestClient) -> None:
    resp = client.post("/import/json", json={"version": 99})
    assert resp.status_code == 400
