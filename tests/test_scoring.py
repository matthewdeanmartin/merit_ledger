"""Scoring engine tests (spec §12), including hypothesis properties."""

from __future__ import annotations

from hypothesis import given
from hypothesis import strategies as st

from merit_ledger.backend.domain.scoring import ScoreRequest, compute_points


def test_count_only_awards_zero() -> None:
    req = ScoreRequest(points_mode="count_only", base_points=10)
    assert compute_points(req) == 0


def test_reflection_only_awards_zero() -> None:
    assert compute_points(ScoreRequest(points_mode="reflection_only", base_points=10)) == 0


def test_base_points() -> None:
    assert compute_points(ScoreRequest(points_mode="points", base_points=10)) == 10


def test_quantity_multiplier() -> None:
    req = ScoreRequest(points_mode="points", base_points=0, quantity=60, quantity_multiplier=1.0)
    assert compute_points(req) == 60


def test_manual_override_wins() -> None:
    req = ScoreRequest(points_mode="points", base_points=10, manual_override=3)
    assert compute_points(req) == 3


def test_daily_cap_clamps() -> None:
    req = ScoreRequest(points_mode="points", base_points=50, daily_cap=60, points_already_today=40)
    assert compute_points(req) == 20  # only 20 headroom left


def test_daily_cap_already_full() -> None:
    req = ScoreRequest(points_mode="points", base_points=50, daily_cap=60, points_already_today=60)
    assert compute_points(req) == 0


@given(
    base=st.integers(min_value=0, max_value=1000),
    cap=st.integers(min_value=0, max_value=1000),
    already=st.integers(min_value=0, max_value=1000),
)
def test_never_exceeds_cap(base: int, cap: int, already: int) -> None:
    """Awarded points never push the daily total above the cap, and are non-negative."""
    awarded = compute_points(
        ScoreRequest(points_mode="points", base_points=base, daily_cap=cap, points_already_today=already)
    )
    assert awarded >= 0
    assert min(already, cap) + awarded <= cap or already >= cap


@given(override=st.integers(min_value=0, max_value=1000))
def test_override_respected_without_cap(override: int) -> None:
    awarded = compute_points(
        ScoreRequest(points_mode="points", base_points=99, manual_override=override)
    )
    assert awarded == override
