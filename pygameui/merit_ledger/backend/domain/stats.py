"""Pure statistics aggregation (spec §14.3 stats). No I/O."""

from __future__ import annotations

from collections import Counter
from dataclasses import dataclass, field

from merit_ledger.backend.domain.models import LedgerEntry, Vow


@dataclass
class PeriodStats:
    """Totals for a time period."""

    total_points: int = 0
    entry_count: int = 0
    by_type: dict[str, int] = field(default_factory=dict)


def period_stats(entries: list[LedgerEntry]) -> PeriodStats:
    """Aggregate a list of entries into point/count totals and a per-type breakdown."""
    by_type: Counter[str] = Counter()
    total = 0
    for e in entries:
        total += e.points
        by_type[e.entry_type] += 1
    return PeriodStats(total_points=total, entry_count=len(entries), by_type=dict(by_type))


def points_by_key(entries: list[LedgerEntry], key: str) -> dict[str, int]:
    """Sum points grouped by an entry attribute (e.g. ``template_id`` or ``tradition``)."""
    out: Counter[str] = Counter()
    for e in entries:
        value = getattr(e, key, None)
        if value:
            out[str(value)] += e.points
    return dict(out)


@dataclass
class VowStats:
    """Summary of vows by status plus active streaks."""

    by_status: dict[str, int] = field(default_factory=dict)
    streaks: dict[str, int] = field(default_factory=dict)


def vow_stats(vows: list[Vow]) -> VowStats:
    """Aggregate vows into status counts and a name→streak map."""
    by_status: Counter[str] = Counter()
    streaks: dict[str, int] = {}
    for v in vows:
        by_status[v.status] += 1
        if v.streak:
            streaks[v.name] = v.streak
    return VowStats(by_status=dict(by_status), streaks=streaks)
