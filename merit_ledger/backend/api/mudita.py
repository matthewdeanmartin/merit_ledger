"""Mudita garden endpoints (spec §5.9, §16)."""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter, Depends
from pydantic import BaseModel

from merit_ledger.backend.api.deps import get_repo
from merit_ledger.backend.domain.models import LedgerEntry
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.services import mudita_service

router = APIRouter()


class RejoiceBody(BaseModel):
    """Body for rejoicing: a sample id from the feed, or explicit text."""

    sample_id: str | None = None
    text: str | None = None


@router.get("/mudita/demo-feed")
def demo_feed(repo: MeritRepository = Depends(get_repo)) -> dict[str, Any]:
    """Return the local sample feed + tradition rejoice verb."""
    return mudita_service.demo_feed(repo)


@router.post("/mudita/rejoice")
def rejoice(body: RejoiceBody, repo: MeritRepository = Depends(get_repo)) -> LedgerEntry:
    """Rejoice in a sample action → local ledger entry (no network)."""
    return mudita_service.rejoice(repo, sample_id=body.sample_id, text=body.text)
