"""Profile and settings endpoints (spec §14.3)."""

from __future__ import annotations

from fastapi import APIRouter, Depends

from merit_ledger.backend.api.deps import get_repo
from merit_ledger.backend.domain.models import Profile, Settings
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.services import profile_service

router = APIRouter()


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
