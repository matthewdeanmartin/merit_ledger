"""Profile and settings endpoints (spec §14.3)."""

from __future__ import annotations

from typing import Literal

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel

from merit_ledger.backend.api.deps import get_repo
from merit_ledger.backend.domain.models import Profile, Settings
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.services import profile_service

router = APIRouter()


class ClearDataRequest(BaseModel):
    """Body for clearing local data. ``confirm`` must be true to proceed (safety).

    ``scope`` selects how much to remove:

    * ``"all"`` — factory reset (also wipes profile + settings; onboarding reruns).
    * ``"user_data"`` — practice ledger only (entries/vows/dedications/custom templates);
      profile + settings are kept. Useful for manual testing.
    """

    confirm: bool = False
    scope: Literal["all", "user_data"] = "all"


@router.get("/profile")
def get_profile(repo: MeritRepository = Depends(get_repo)) -> Profile:
    """Return the local profile (defaults if never saved)."""
    return profile_service.get_profile(repo)


@router.put("/profile")
def put_profile(profile: Profile, repo: MeritRepository = Depends(get_repo)) -> Profile:
    """Create or update the local profile."""
    return profile_service.save_profile(repo, profile)


@router.get("/settings")
def get_settings(repo: MeritRepository = Depends(get_repo)) -> Settings:
    """Return settings (defaults if never saved)."""
    return profile_service.get_settings(repo)


@router.put("/settings")
def put_settings(settings: Settings, repo: MeritRepository = Depends(get_repo)) -> Settings:
    """Create or update settings."""
    return profile_service.save_settings(repo, settings)


@router.post("/settings/clear")
def clear_data(body: ClearDataRequest, repo: MeritRepository = Depends(get_repo)) -> dict[str, int]:
    """Delete local data. Requires ``confirm: true`` (spec §17.5).

    ``scope="all"`` is a factory reset; ``scope="user_data"`` keeps profile + settings.
    """
    if not body.confirm:
        raise HTTPException(status_code=400, detail="Set confirm=true to clear local data.")
    deleted = (
        profile_service.clear_user_data(repo)
        if body.scope == "user_data"
        else profile_service.clear_data(repo)
    )
    return {"deleted": deleted}
