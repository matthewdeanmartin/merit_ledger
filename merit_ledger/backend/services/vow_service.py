"""Vow lifecycle service (spec §5.3-§5.6, §8).

Every lifecycle action re-persists the vow (so its GSI1 status key stays current) and
records a ledger entry, so a vow's timeline is just the entries carrying its id.
"""

from __future__ import annotations

from merit_ledger.backend.domain import vow_state
from merit_ledger.backend.domain.ids import now_iso
from merit_ledger.backend.domain.models import LedgerEntry, RepentanceCategory, Vow
from merit_ledger.backend.domain.vow_state import VowAction
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.repository.item_keys import (
    VOW_SK_PREFIX,
    to_vow_item,
    vow_from_item,
    vow_sk,
    vow_status_gsi1pk,
    user_pk,
)
from merit_ledger.backend.services import entry_service, profile_service
from merit_ledger.local.config import DEFAULT_USER_ID


def _save(repo: MeritRepository, vow: Vow) -> Vow:
    """Persist a vow, bumping updated_at."""
    vow.updated_at = now_iso()
    repo.put_item(to_vow_item(vow))
    return vow


def create_vow(repo: MeritRepository, vow: Vow, user_id: str = DEFAULT_USER_ID) -> Vow:
    """Create a vow and record a ``vow_created`` entry."""
    vow.user_id = user_id
    if vow.tradition is None:
        vow.tradition = profile_service.get_settings(repo, user_id).tradition
    if vow.start_date is None:
        vow.start_date = now_iso()[:10]
    _save(repo, vow)
    _record(repo, vow, "vow_created", title=f"Vow created: {vow.name}")
    return vow


def get_vow(repo: MeritRepository, vow_id: str, user_id: str = DEFAULT_USER_ID) -> Vow | None:
    """Return a vow by id, or None."""
    item = repo.get_item(user_pk(user_id), vow_sk(vow_id))
    return vow_from_item(item) if item else None


def list_vows(
    repo: MeritRepository, *, status: str | None = None, user_id: str = DEFAULT_USER_ID
) -> list[Vow]:
    """List vows, optionally filtered by status via GSI1."""
    if status is not None:
        items = repo.query_gsi1(vow_status_gsi1pk(user_id, status))
    else:
        items = repo.query_pk(user_pk(user_id), VOW_SK_PREFIX)
    return [vow_from_item(i) for i in items]


def update_vow(repo: MeritRepository, vow: Vow) -> Vow:
    """Persist edits to an existing vow (status/keys handled by caller)."""
    return _save(repo, vow)


def _record(
    repo: MeritRepository,
    vow: Vow,
    entry_type: str,
    *,
    title: str,
    points: int = 0,
    manual_points: int | None = 0,
    reflection: str | None = None,
    category: RepentanceCategory | None = None,
    repair_intention: str | None = None,
    linked: bool = False,
) -> LedgerEntry:
    """Create a ledger entry tied to ``vow`` for a lifecycle event."""
    entry = LedgerEntry(
        user_id=vow.user_id,
        entry_type=entry_type,  # type: ignore[arg-type]
        tradition=vow.tradition,
        vow_id=vow.vow_id,
        linked_vow_id=vow.vow_id if linked else None,
        title=title,
        points=points,
        reflection=reflection,
        category=category,
        repair_intention=repair_intention,
    )
    return entry_service.create_entry(repo, entry, manual_points=manual_points)


def _transition(vow: Vow, action: VowAction) -> None:
    """Apply a state-machine transition to ``vow`` in place (may raise)."""
    vow.status = vow_state.apply(action, vow.status)


def complete_vow(
    repo: MeritRepository, vow: Vow, *, reflection: str | None = None
) -> tuple[Vow, LedgerEntry]:
    """Complete a positive vow: award points, bump streak, record entry (spec §5.3).

    Completion is scored through the entry service so the user's points mode is honored;
    the vow keeps a simple streak (consecutive distinct completion days).
    """
    today = now_iso()[:10]
    if vow.last_completed_date == today:
        pass  # already completed today; streak unchanged
    elif vow.last_completed_date is None:
        vow.streak = 1
    else:
        vow.streak += 1
    vow.last_completed_date = today
    # A positive vow marked complete returns to active for its next occurrence; a repair
    # completion (negative vow) closes the repair loop.
    if vow.vow_type == "positive":
        entry = _record(
            repo, vow, "vow_completed", title=f"Completed: {vow.name}",
            manual_points=None, points=vow.default_points, reflection=reflection,
        )
        _save(repo, vow)  # streak/date persisted; status stays active
    else:
        _transition(vow, "complete")
        entry = _record(
            repo, vow, "vow_completed", title=f"Repair completed: {vow.name}",
            reflection=reflection,
        )
        _save(repo, vow)
    return vow, entry


def breach_vow(
    repo: MeritRepository,
    vow: Vow,
    *,
    category: RepentanceCategory | None = None,
    note: str | None = None,
    repair_intention: str | None = None,
) -> tuple[Vow, LedgerEntry]:
    """Record a negative-vow breach → ``repair_in_progress`` (spec §5.4).

    No shame score. Negative points are only applied if the user enabled them (spec §12.3);
    even then the actual point math lands with repentance in Sprint 4 — here the breach
    entry is recorded at 0 points.
    """
    _transition(vow, "breach")
    entry = _record(
        repo,
        vow,
        "vow_breached",
        title=f"Breach recorded: {vow.name}",
        reflection=note,
        category=category or vow.repentance_category,
        repair_intention=repair_intention,
        linked=True,
    )
    _save(repo, vow)
    return vow, entry


def pause_vow(
    repo: MeritRepository, vow: Vow, *, reason: str | None = None, resume_date: str | None = None
) -> tuple[Vow, LedgerEntry]:
    """Pause a vow with care (spec §5.5). Paused vows never count as failed."""
    _transition(vow, "pause")
    vow.pause_reason = reason
    vow.resume_date = resume_date
    entry = _record(repo, vow, "vow_paused", title=f"Paused with care: {vow.name}", reflection=reason)
    _save(repo, vow)
    return vow, entry


def resume_vow(
    repo: MeritRepository,
    vow: Vow,
    *,
    new_frequency: str | None = None,
    new_points: int | None = None,
    renewed_intention: str | None = None,
) -> tuple[Vow, LedgerEntry]:
    """Resume a paused vow, optionally with a new frequency/points/intention (spec §5.6)."""
    _transition(vow, "resume")
    vow.pause_reason = None
    vow.resume_date = None
    if new_frequency is not None:
        vow.frequency = new_frequency
    if new_points is not None:
        vow.default_points = new_points
    entry = _record(
        repo, vow, "vow_resumed", title=f"Resumed: {vow.name}", reflection=renewed_intention
    )
    _save(repo, vow)
    return vow, entry


def retire_vow(repo: MeritRepository, vow: Vow) -> tuple[Vow, LedgerEntry]:
    """Retire a vow (spec §8 status)."""
    _transition(vow, "retire")
    entry = _record(repo, vow, "vow_retired", title=f"Retired: {vow.name}")
    _save(repo, vow)
    return vow, entry
