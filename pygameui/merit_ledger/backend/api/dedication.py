"""Dedication endpoints (spec §5.8, §14.3)."""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter, Depends

from merit_ledger.backend.api.deps import get_repo
from merit_ledger.backend.domain.models import Dedication
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.services import dedication_service

router = APIRouter()


@router.get("/dedications")
def list_dedications(repo: MeritRepository = Depends(get_repo)) -> list[Dedication]:
    """List the user's dedications."""
    return dedication_service.list_dedications(repo)


@router.get("/dedications/presets")
def dedication_presets(repo: MeritRepository = Depends(get_repo)) -> dict[str, Any]:
    """Return preset targets + default text for the active tradition."""
    return dedication_service.presets(repo)


@router.post("/dedications")
def create_dedication(dedication: Dedication, repo: MeritRepository = Depends(get_repo)) -> Dedication:
    """Create a dedication (records only; does not reduce balance by default)."""
    return dedication_service.create_dedication(repo, dedication)
