"""Ledger entry CRUD with scoring applied on create (spec §5.2, §9, §12)."""

from __future__ import annotations

from merit_ledger.backend.domain.ids import now_iso
from merit_ledger.backend.domain.models import LedgerEntry
from merit_ledger.backend.domain.scoring import ScoreRequest, compute_points
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.repository.item_keys import (
    ENTRY_SK_PREFIX,
    entry_from_item,
    entry_sk,
    entry_type_gsi1pk,
    to_entry_item,
    user_pk,
)
from merit_ledger.backend.services import profile_service, template_service
from merit_ledger.local.config import DEFAULT_USER_ID


def _points_today(repo: MeritRepository, user_id: str, day: str) -> int:
    """Sum points already recorded on the calendar day ``day`` (YYYY-MM-DD)."""
    items = repo.query_pk(user_pk(user_id), f"{ENTRY_SK_PREFIX}{day}")
    return sum(int(i.item.get("points", 0)) for i in items)


def create_entry(
    repo: MeritRepository,
    entry: LedgerEntry,
    *,
    manual_points: int | None = None,
    quantity_multiplier: float = 0.0,
    daily_cap: int | None = None,
    user_id: str = DEFAULT_USER_ID,
) -> LedgerEntry:
    """Score and persist a new ledger entry.

    If ``entry.points`` was not supplied by the caller, points are computed from the
    entry's template defaults, the user's points mode, and the optional scoring args.

    Args:
        repo: Storage.
        entry: The (unsaved) entry; missing ids/timestamps are filled in.
        manual_points: Explicit override that wins over computed points.
        quantity_multiplier: Per-unit multiplier applied to ``entry.quantity``.
        daily_cap: Cumulative daily cap for this entry's contribution.
        user_id: Owning user.

    Returns:
        The saved entry with points and ids populated.
    """
    entry.user_id = user_id
    settings = profile_service.get_settings(repo, user_id)

    base_points = entry.points
    if entry.template_id and base_points == 0 and manual_points is None:
        tmpl = template_service.get_template(repo, entry.template_id, settings.tradition, user_id)
        if tmpl:
            base_points = tmpl.default_points
            if entry.tradition is None:
                entry.tradition = tmpl.tradition

    day = entry.occurred_at[:10]
    points = compute_points(
        ScoreRequest(
            points_mode=settings.points_mode,
            base_points=base_points,
            quantity=entry.quantity,
            quantity_multiplier=quantity_multiplier,
            daily_cap=daily_cap,
            manual_override=manual_points,
            points_already_today=_points_today(repo, user_id, day),
        )
    )
    entry.points = points
    entry.created_at = entry.created_at or now_iso()
    entry.updated_at = now_iso()
    repo.put_item(to_entry_item(entry))
    return entry


def get_entry(repo: MeritRepository, entry: LedgerEntry) -> LedgerEntry | None:
    """Fetch an entry by reconstructing its sort key (needs occurred_at + id)."""
    item = repo.get_item(user_pk(entry.user_id), entry_sk(entry))
    return entry_from_item(item) if item else None


def find_entry(repo: MeritRepository, entry_id: str, user_id: str = DEFAULT_USER_ID) -> LedgerEntry | None:
    """Find an entry by id alone (scans the user's entries; fine for a local ledger)."""
    for item in repo.query_pk(user_pk(user_id), ENTRY_SK_PREFIX):
        if item.item.get("entry_id") == entry_id:
            return entry_from_item(item)
    return None


def list_entries(
    repo: MeritRepository,
    *,
    date_prefix: str | None = None,
    entry_type: str | None = None,
    user_id: str = DEFAULT_USER_ID,
) -> list[LedgerEntry]:
    """List entries, optionally filtered by date prefix or by entry type (GSI1)."""
    if entry_type is not None:
        items = repo.query_gsi1(entry_type_gsi1pk(user_id, entry_type))
    else:
        sk_prefix = f"{ENTRY_SK_PREFIX}{date_prefix}" if date_prefix else ENTRY_SK_PREFIX
        items = repo.query_pk(user_pk(user_id), sk_prefix)
    return [entry_from_item(i) for i in items]


def update_entry(repo: MeritRepository, entry: LedgerEntry) -> LedgerEntry:
    """Persist changes to an existing entry (keys unchanged; bumps updated_at)."""
    entry.updated_at = now_iso()
    repo.put_item(to_entry_item(entry))
    return entry


def delete_entry(repo: MeritRepository, entry: LedgerEntry) -> None:
    """Delete an entry by its keys."""
    repo.delete_item(user_pk(entry.user_id), entry_sk(entry))
