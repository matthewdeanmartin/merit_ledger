"""Stats service — pulls entries/vows and aggregates them (spec §14.3)."""

from __future__ import annotations

import datetime as _dt
from typing import Any

from merit_ledger.backend.domain import stats
from merit_ledger.backend.domain.models import LedgerEntry
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.services import entry_service, vow_service
from merit_ledger.local.config import DEFAULT_USER_ID


def _today() -> _dt.date:
    """Return today's UTC date (isolated for testability)."""
    return _dt.datetime.now(tz=_dt.timezone.utc).date()


def _entries_since(repo: MeritRepository, start: _dt.date, user_id: str) -> list[LedgerEntry]:
    """Return entries occurring on/after ``start`` (inclusive), by occurred_at date."""
    start_iso = start.isoformat()
    all_entries = entry_service.list_entries(repo, user_id=user_id)
    return [e for e in all_entries if e.occurred_at[:10] >= start_iso]


def today(repo: MeritRepository, user_id: str = DEFAULT_USER_ID) -> dict[str, Any]:
    """Point/count totals for the current UTC day."""
    day = _today()
    entries = entry_service.list_entries(repo, date_prefix=day.isoformat(), user_id=user_id)
    return stats.period_stats(entries).__dict__


def week(repo: MeritRepository, user_id: str = DEFAULT_USER_ID) -> dict[str, Any]:
    """Point/count totals for the trailing 7 days (including today)."""
    start = _today() - _dt.timedelta(days=6)
    return stats.period_stats(_entries_since(repo, start, user_id)).__dict__


def month(repo: MeritRepository, user_id: str = DEFAULT_USER_ID) -> dict[str, Any]:
    """Point/count totals for the current calendar month."""
    day = _today()
    entries = entry_service.list_entries(
        repo, date_prefix=day.strftime("%Y-%m"), user_id=user_id
    )
    return stats.period_stats(entries).__dict__


def by_template(repo: MeritRepository, user_id: str = DEFAULT_USER_ID) -> dict[str, int]:
    """Points grouped by template id (all time)."""
    entries = entry_service.list_entries(repo, user_id=user_id)
    return stats.points_by_key(entries, "template_id")


def by_tradition(repo: MeritRepository, user_id: str = DEFAULT_USER_ID) -> dict[str, int]:
    """Points grouped by tradition (all time)."""
    entries = entry_service.list_entries(repo, user_id=user_id)
    return stats.points_by_key(entries, "tradition")


def vows(repo: MeritRepository, user_id: str = DEFAULT_USER_ID) -> dict[str, Any]:
    """Vow status counts + active streaks."""
    return stats.vow_stats(vow_service.list_vows(repo, user_id=user_id)).__dict__
