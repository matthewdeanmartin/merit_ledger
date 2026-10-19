"""Repentance endpoints (spec §5.7, §14.3)."""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter, Depends
from pydantic import BaseModel

from merit_ledger.backend.api.deps import get_repo
from merit_ledger.backend.domain.models import LedgerEntry, RepentanceCategory
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.services import repentance_service

router = APIRouter()


class RepentanceCreate(BaseModel):
    """Body for recording repentance. No secret/identifying fields by design (spec §2.4)."""

    category: RepentanceCategory
    title: str | None = None
    reflection: str | None = None
    repair_intention: str | None = None
    linked_vow_id: str | None = None


@router.get("/repentance/categories")
def get_categories() -> dict[str, Any]:
    """Return repentance categories plus the privacy reminder (spec §5.7)."""
    return {
        "categories": repentance_service.categories(),
        "privacy_reminder": repentance_service.PRIVACY_REMINDER,
    }


@router.post("/repentance")
def create_repentance(body: RepentanceCreate, repo: MeritRepository = Depends(get_repo)) -> LedgerEntry:
    """Record a repentance ('return to practice') entry."""
    return repentance_service.create_repentance(
        repo,
        category=body.category,
        title=body.title,
        reflection=body.reflection,
        repair_intention=body.repair_intention,
        linked_vow_id=body.linked_vow_id,
    )
