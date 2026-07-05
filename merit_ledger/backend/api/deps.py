"""Shared FastAPI dependencies."""

from __future__ import annotations

from fastapi import Request

from merit_ledger.backend.repository.base import MeritRepository


def get_repo(request: Request) -> MeritRepository:
    """Return the repository stored on ``app.state`` at startup."""
    repo: MeritRepository = request.app.state.repo
    return repo
