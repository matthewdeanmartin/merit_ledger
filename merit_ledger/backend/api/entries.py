"""Ledger entry endpoints (spec §14.3)."""

from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel

from merit_ledger.backend.api.deps import get_repo
from merit_ledger.backend.domain.models import LedgerEntry
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.services import entry_service

router = APIRouter()


class EntryCreate(BaseModel):
    """Request body for creating an entry: the entry plus optional scoring hints."""

    entry: LedgerEntry
    manual_points: int | None = None
    quantity_multiplier: float = 0.0
    daily_cap: int | None = None


@router.get("/entries")
def list_entries(
    date_prefix: str | None = None,
    entry_type: str | None = None,
    repo: MeritRepository = Depends(get_repo),
) -> list[LedgerEntry]:
    """List entries, optionally filtered by ``date_prefix`` or ``entry_type``."""
    return entry_service.list_entries(repo, date_prefix=date_prefix, entry_type=entry_type)


@router.get("/entries/{entry_id}")
def get_entry(entry_id: str, repo: MeritRepository = Depends(get_repo)) -> LedgerEntry:
    """Return a single entry by id."""
    found = entry_service.find_entry(repo, entry_id)
    if found is None:
        raise HTTPException(status_code=404, detail="Unknown entry")
    return found


@router.post("/entries")
def create_entry(body: EntryCreate, repo: MeritRepository = Depends(get_repo)) -> LedgerEntry:
    """Create a ledger entry (points computed via the scoring engine)."""
    return entry_service.create_entry(
        repo,
        body.entry,
        manual_points=body.manual_points,
        quantity_multiplier=body.quantity_multiplier,
        daily_cap=body.daily_cap,
    )


@router.put("/entries/{entry_id}")
def update_entry(
    entry_id: str, entry: LedgerEntry, repo: MeritRepository = Depends(get_repo)
) -> LedgerEntry:
    """Update an existing entry (id from the path wins)."""
    existing = entry_service.find_entry(repo, entry_id)
    if existing is None:
        raise HTTPException(status_code=404, detail="Unknown entry")
    entry.entry_id = entry_id
    return entry_service.update_entry(repo, entry)


@router.delete("/entries/{entry_id}")
def delete_entry(entry_id: str, repo: MeritRepository = Depends(get_repo)) -> dict[str, str]:
    """Delete an entry by id."""
    existing = entry_service.find_entry(repo, entry_id)
    if existing is None:
        raise HTTPException(status_code=404, detail="Unknown entry")
    entry_service.delete_entry(repo, existing)
    return {"status": "deleted"}
