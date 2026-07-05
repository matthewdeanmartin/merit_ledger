"""Stats endpoints (spec §14.3)."""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter, Depends

from merit_ledger.backend.api.deps import get_repo
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.services import stats_service

router = APIRouter()


@router.get("/stats/today")
def stats_today(repo: MeritRepository = Depends(get_repo)) -> dict[str, Any]:
    """Point/count totals for today."""
    return stats_service.today(repo)


@router.get("/stats/week")
def stats_week(repo: MeritRepository = Depends(get_repo)) -> dict[str, Any]:
    """Point/count totals for the trailing week."""
    return stats_service.week(repo)


@router.get("/stats/month")
def stats_month(repo: MeritRepository = Depends(get_repo)) -> dict[str, Any]:
    """Point/count totals for the current month."""
    return stats_service.month(repo)


@router.get("/stats/by-template")
def stats_by_template(repo: MeritRepository = Depends(get_repo)) -> dict[str, int]:
    """Points grouped by template."""
    return stats_service.by_template(repo)


@router.get("/stats/by-tradition")
def stats_by_tradition(repo: MeritRepository = Depends(get_repo)) -> dict[str, int]:
    """Points grouped by tradition."""
    return stats_service.by_tradition(repo)


@router.get("/stats/vows")
def stats_vows(repo: MeritRepository = Depends(get_repo)) -> dict[str, Any]:
    """Vow status counts + streaks."""
    return stats_service.vows(repo)
