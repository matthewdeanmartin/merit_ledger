"""Health-check endpoint."""

from __future__ import annotations

from fastapi import APIRouter

router = APIRouter()


@router.get("/health")
def health() -> dict[str, str]:
    """Return a simple liveness payload the launcher polls before showing the UI."""
    return {"status": "ok"}
