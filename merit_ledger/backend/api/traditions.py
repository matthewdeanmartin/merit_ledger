"""Tradition pack + tradition-selection endpoints (spec §14.3)."""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel

from merit_ledger.backend.api.deps import get_repo
from merit_ledger.backend.domain.models import TraditionId
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.services import profile_service, tradition_service

router = APIRouter()


class TraditionSelection(BaseModel):
    """Body for choosing the active tradition."""

    tradition: TraditionId


@router.get("/traditions")
def list_traditions() -> list[dict[str, str]]:
    """List available tradition packs."""
    return tradition_service.list_traditions()


@router.get("/traditions/{tradition_id}")
def get_tradition(tradition_id: str) -> dict[str, Any]:
    """Return a full tradition pack."""
    try:
        return tradition_service.get_tradition(tradition_id)
    except KeyError as exc:
        raise HTTPException(status_code=404, detail="Unknown tradition") from exc


@router.put("/settings/tradition")
def set_tradition(
    selection: TraditionSelection, repo: MeritRepository = Depends(get_repo)
) -> dict[str, str]:
    """Persist the active tradition on both profile and settings."""
    settings = profile_service.get_settings(repo)
    settings.tradition = selection.tradition
    profile_service.save_settings(repo, settings)
    profile = profile_service.get_profile(repo)
    profile.tradition = selection.tradition
    profile_service.save_profile(repo, profile)
    return {"tradition": selection.tradition}
