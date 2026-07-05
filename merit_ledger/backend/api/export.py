"""Export endpoints (spec §14.3, §19). Markdown lands in Sprint 4."""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter, Depends

from merit_ledger.backend.api.deps import get_repo
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.services import export_service

router = APIRouter()


@router.get("/export/json")
def export_json(repo: MeritRepository = Depends(get_repo)) -> dict[str, Any]:
    """Return a full JSON snapshot of the ledger."""
    return export_service.export_json(repo)
