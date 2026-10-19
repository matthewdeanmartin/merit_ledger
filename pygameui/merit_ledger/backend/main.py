"""FastAPI application factory for the local Merit Ledger backend (spec §14)."""

from __future__ import annotations

from fastapi import FastAPI

from merit_ledger.backend.api import (
    dedication,
    entries,
    export,
    health,
    mudita,
    profile,
    repentance,
    stats,
    templates,
    traditions,
    vows,
)
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.repository.sqlite_repo import SqliteMeritRepository
from merit_ledger.local.data_dir import db_path


def create_app(repo: MeritRepository | None = None) -> FastAPI:
    """Build the FastAPI app.

    Args:
        repo: Repository to use. Defaults to a SQLite repo at the local data path.
            Tests pass an :class:`InMemoryMeritRepository` here.

    Returns:
        A configured :class:`FastAPI` instance with ``app.state.repo`` set.
    """
    app = FastAPI(title="Merit Ledger", version="0.1.0")
    app.state.repo = repo if repo is not None else SqliteMeritRepository(db_path())
    app.include_router(health.router)
    app.include_router(profile.router)
    app.include_router(traditions.router)
    app.include_router(templates.router)
    app.include_router(entries.router)
    app.include_router(vows.router)
    app.include_router(repentance.router)
    app.include_router(dedication.router)
    app.include_router(mudita.router)
    app.include_router(stats.router)
    app.include_router(export.router)
    return app


app = create_app()
