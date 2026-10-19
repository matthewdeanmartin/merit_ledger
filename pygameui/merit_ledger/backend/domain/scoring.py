"""Pure scoring logic (spec §12). No I/O.

The scoring engine turns a practice (template defaults + user input) into a point value,
honoring the user's points mode, quantity multipliers, daily caps, and manual overrides.
"""

from __future__ import annotations

from dataclasses import dataclass

from merit_ledger.backend.domain.models import PointsMode


@dataclass(frozen=True)
class ScoreRequest:
    """Inputs to a scoring decision."""

    points_mode: PointsMode
    base_points: int
    quantity: int | None = None
    quantity_multiplier: float = 0.0
    daily_cap: int | None = None
    manual_override: int | None = None
    points_already_today: int = 0


def compute_points(req: ScoreRequest) -> int:
    """Return the points to award for a single practice entry.

    Rules (spec §12):

    * ``count_only`` / ``reflection_only`` modes never award points → 0.
    * A ``manual_override`` (when allowed by the caller) wins over computed points.
    * Otherwise points = ``base_points`` + ``quantity * quantity_multiplier``.
    * A ``daily_cap`` limits the *cumulative* daily total, so the awarded amount is
      clamped by how much headroom remains under the cap. Never negative.

    Args:
        req: The scoring request.

    Returns:
        Non-negative integer points to award.
    """
    if req.points_mode != "points":
        return 0

    if req.manual_override is not None:
        raw = req.manual_override
    else:
        raw = req.base_points
        if req.quantity is not None and req.quantity_multiplier:
            raw += int(req.quantity * req.quantity_multiplier)

    raw = max(0, raw)

    if req.daily_cap is not None:
        remaining = max(0, req.daily_cap - max(0, req.points_already_today))
        raw = min(raw, remaining)

    return raw
