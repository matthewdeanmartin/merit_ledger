"""Health endpoint test via FastAPI TestClient."""

from __future__ import annotations

from fastapi.testclient import TestClient

from merit_ledger.backend.main import create_app
from merit_ledger.backend.repository.memory_repo import InMemoryMeritRepository


def test_health() -> None:
    app = create_app(repo=InMemoryMeritRepository())
    client = TestClient(app)
    resp = client.get("/health")
    assert resp.status_code == 200
    assert resp.json() == {"status": "ok"}
