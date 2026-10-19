"""Shared pytest fixtures for API tests."""

from __future__ import annotations

from collections.abc import Iterator

import pytest
from fastapi.testclient import TestClient

from merit_ledger.backend.main import create_app
from merit_ledger.backend.repository.memory_repo import InMemoryMeritRepository


@pytest.fixture
def client() -> Iterator[TestClient]:
    """A TestClient backed by a fresh in-memory repository."""
    app = create_app(repo=InMemoryMeritRepository())
    with TestClient(app) as c:
        yield c
