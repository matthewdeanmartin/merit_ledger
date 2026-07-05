"""Export + import endpoints (spec §14.3, §19)."""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter, Depends, HTTPException
from fastapi.responses import PlainTextResponse

from merit_ledger.backend.api.deps import get_repo
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.services import export_service

router = APIRouter()


@router.get("/export/json")
def export_json(repo: MeritRepository = Depends(get_repo)) -> dict[str, Any]:
    """Return a full JSON snapshot of the ledger."""
    return export_service.export_json(repo)


@router.get("/export/markdown", response_class=PlainTextResponse)
def export_markdown(repo: MeritRepository = Depends(get_repo)) -> str:
    """Return a human-readable Markdown export."""
    return export_service.export_markdown(repo)


@router.post("/import/json")
def import_json(snapshot: dict[str, Any], repo: MeritRepository = Depends(get_repo)) -> dict[str, int]:
    """Import a JSON snapshot (from /export/json). Upserts entities."""
    try:
        return export_service.import_json(repo, snapshot)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
