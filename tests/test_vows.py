"""Vow state machine + service tests (spec §5.3-§5.6, §8)."""

from __future__ import annotations

import pytest

from merit_ledger.backend.domain import vow_state
from merit_ledger.backend.domain.models import Vow
from merit_ledger.backend.domain.vow_state import IllegalVowTransition
from merit_ledger.backend.repository.memory_repo import InMemoryMeritRepository
from merit_ledger.backend.services import entry_service, vow_service


# --- state machine -----------------------------------------------------------


def test_breach_moves_to_repair() -> None:
    assert vow_state.apply("breach", "active") == "repair_in_progress"


def test_cannot_pause_retired() -> None:
    assert not vow_state.can("pause", "retired")
    with pytest.raises(IllegalVowTransition):
        vow_state.apply("pause", "retired")


def test_resume_only_from_paused() -> None:
    assert vow_state.can("resume", "paused")
    assert not vow_state.can("resume", "active")


def test_retire_from_anywhere_but_retired() -> None:
    for status in ("draft", "active", "paused", "repair_in_progress", "completed"):
        assert vow_state.can("retire", status)  # type: ignore[arg-type]
    assert not vow_state.can("retire", "retired")


# --- service -----------------------------------------------------------------


def _repo() -> InMemoryMeritRepository:
    return InMemoryMeritRepository()


def test_create_records_created_entry() -> None:
    repo = _repo()
    vow = vow_service.create_vow(repo, Vow(name="Sit daily", vow_type="positive"))
    timeline = entry_service.list_entries(repo, vow_id=vow.vow_id)
    assert [e.entry_type for e in timeline] == ["vow_created"]


def test_complete_positive_bumps_streak_and_points() -> None:
    repo = _repo()
    vow = vow_service.create_vow(repo, Vow(name="Sit daily", vow_type="positive", default_points=8))
    vow, entry = vow_service.complete_vow(repo, vow)
    assert vow.streak == 1
    assert vow.status == "active"  # positive vow stays active for next occurrence
    assert entry.entry_type == "vow_completed"
    assert entry.points == 8


def test_breach_negative_enters_repair() -> None:
    repo = _repo()
    vow = vow_service.create_vow(
        repo, Vow(name="No harsh speech", vow_type="negative", repentance_category="speech")
    )
    vow, entry = vow_service.breach_vow(repo, vow, note="noticed impatience")
    assert vow.status == "repair_in_progress"
    assert entry.entry_type == "vow_breached"
    assert entry.points == 0  # no shame score
    assert entry.category == "speech"
    assert entry.linked_vow_id == vow.vow_id


def test_pause_then_resume() -> None:
    repo = _repo()
    vow = vow_service.create_vow(repo, Vow(name="Study", vow_type="positive"))
    vow, _ = vow_service.pause_vow(repo, vow, reason="travel", resume_date="2026-08-01")
    assert vow.status == "paused"
    assert vow.pause_reason == "travel"
    vow, _ = vow_service.resume_vow(repo, vow, new_points=12)
    assert vow.status == "active"
    assert vow.pause_reason is None
    assert vow.default_points == 12


def test_list_by_status_uses_gsi() -> None:
    repo = _repo()
    a = vow_service.create_vow(repo, Vow(name="A", vow_type="positive"))
    vow_service.create_vow(repo, Vow(name="B", vow_type="positive"))
    vow_service.pause_vow(repo, a, reason="rest")
    active = vow_service.list_vows(repo, status="active")
    paused = vow_service.list_vows(repo, status="paused")
    assert {v.name for v in active} == {"B"}
    assert {v.name for v in paused} == {"A"}
