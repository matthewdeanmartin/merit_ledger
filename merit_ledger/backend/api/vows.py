"""Vow endpoints (spec §14.3, flows §5.3-§5.6)."""

from __future__ import annotations

from collections.abc import Callable
from typing import Any

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel

from merit_ledger.backend.api.deps import get_repo
from merit_ledger.backend.domain.models import LedgerEntry, RepentanceCategory, Vow
from merit_ledger.backend.domain.vow_state import IllegalVowTransition
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.services import vow_service

router = APIRouter()


class VowEventResult(BaseModel):
    """Response for a lifecycle action: the updated vow + the recorded entry."""

    vow: Vow
    entry: LedgerEntry


class PauseBody(BaseModel):
    """Body for pausing a vow (spec §5.5)."""

    reason: str | None = None
    resume_date: str | None = None


class ResumeBody(BaseModel):
    """Body for resuming a vow (spec §5.6)."""

    new_frequency: str | None = None
    new_points: int | None = None
    renewed_intention: str | None = None


class BreachBody(BaseModel):
    """Body for recording a breach (spec §5.4). Deliberately no 'what did you do' field."""

    category: RepentanceCategory | None = None
    note: str | None = None
    repair_intention: str | None = None


class CompleteBody(BaseModel):
    """Body for completing a vow (spec §5.3)."""

    reflection: str | None = None


def _require(repo: MeritRepository, vow_id: str) -> Vow:
    """Fetch a vow or raise 404."""
    vow = vow_service.get_vow(repo, vow_id)
    if vow is None:
        raise HTTPException(status_code=404, detail="Unknown vow")
    return vow


@router.get("/vows")
def list_vows(status: str | None = None, repo: MeritRepository = Depends(get_repo)) -> list[Vow]:
    """List vows, optionally filtered by ``status``."""
    return vow_service.list_vows(repo, status=status)


@router.get("/vows/{vow_id}")
def get_vow(vow_id: str, repo: MeritRepository = Depends(get_repo)) -> Vow:
    """Return a single vow by id."""
    return _require(repo, vow_id)


@router.post("/vows")
def create_vow(vow: Vow, repo: MeritRepository = Depends(get_repo)) -> Vow:
    """Create a vow."""
    return vow_service.create_vow(repo, vow)


@router.put("/vows/{vow_id}")
def update_vow(vow_id: str, vow: Vow, repo: MeritRepository = Depends(get_repo)) -> Vow:
    """Update a vow's editable fields (path id wins)."""
    _require(repo, vow_id)
    vow.vow_id = vow_id
    return vow_service.update_vow(repo, vow)


def _run(
    action: Callable[..., tuple[Vow, LedgerEntry]],
    repo: MeritRepository,
    vow_id: str,
    **kwargs: Any,
) -> VowEventResult:
    """Run a lifecycle action, translating illegal transitions to HTTP 409."""
    vow = _require(repo, vow_id)
    try:
        updated, entry = action(repo, vow, **kwargs)
    except IllegalVowTransition as exc:
        raise HTTPException(status_code=409, detail=str(exc)) from exc
    return VowEventResult(vow=updated, entry=entry)


@router.post("/vows/{vow_id}/pause")
def pause_vow(vow_id: str, body: PauseBody, repo: MeritRepository = Depends(get_repo)) -> VowEventResult:
    """Pause a vow with care."""
    return _run(vow_service.pause_vow, repo, vow_id, reason=body.reason, resume_date=body.resume_date)


@router.post("/vows/{vow_id}/resume")
def resume_vow(vow_id: str, body: ResumeBody, repo: MeritRepository = Depends(get_repo)) -> VowEventResult:
    """Resume a paused vow."""
    return _run(
        vow_service.resume_vow,
        repo,
        vow_id,
        new_frequency=body.new_frequency,
        new_points=body.new_points,
        renewed_intention=body.renewed_intention,
    )


@router.post("/vows/{vow_id}/retire")
def retire_vow(vow_id: str, repo: MeritRepository = Depends(get_repo)) -> VowEventResult:
    """Retire a vow."""
    return _run(vow_service.retire_vow, repo, vow_id)


@router.post("/vows/{vow_id}/complete")
def complete_vow(
    vow_id: str, body: CompleteBody, repo: MeritRepository = Depends(get_repo)
) -> VowEventResult:
    """Complete a positive vow (or close a repair loop)."""
    return _run(vow_service.complete_vow, repo, vow_id, reflection=body.reflection)


@router.post("/vows/{vow_id}/breach")
def breach_vow(vow_id: str, body: BreachBody, repo: MeritRepository = Depends(get_repo)) -> VowEventResult:
    """Record a breach of a negative vow → repair in progress."""
    return _run(
        vow_service.breach_vow,
        repo,
        vow_id,
        category=body.category,
        note=body.note,
        repair_intention=body.repair_intention,
    )
