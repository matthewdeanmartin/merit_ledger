"""Pure vow state-transition logic (spec §8 status, §5.3-§5.6). No I/O.

The transition table is the single source of truth for which lifecycle actions are legal
from a given status. Services call :func:`apply` to compute the next status and get a
clear error when a caller attempts an illegal move.
"""

from __future__ import annotations

from typing import Literal

from merit_ledger.backend.domain.models import VowStatus

VowAction = Literal["activate", "complete", "breach", "pause", "resume", "retire"]

# For each action, the set of statuses it is legal from, and the resulting status.
_TRANSITIONS: dict[VowAction, tuple[frozenset[VowStatus], VowStatus]] = {
    "activate": (frozenset({"draft"}), "active"),
    "complete": (frozenset({"active", "repair_in_progress"}), "completed"),
    # A breach never "fails" a vow — it opens repair.
    "breach": (frozenset({"active", "paused", "repair_in_progress"}), "repair_in_progress"),
    "pause": (frozenset({"active", "repair_in_progress"}), "paused"),
    "resume": (frozenset({"paused"}), "active"),
    "retire": (frozenset({"draft", "active", "paused", "repair_in_progress", "completed"}), "retired"),
}


class IllegalVowTransition(ValueError):
    """Raised when an action is not allowed from the vow's current status."""


def can(action: VowAction, status: VowStatus) -> bool:
    """Return whether ``action`` is legal from ``status``."""
    allowed, _ = _TRANSITIONS[action]
    return status in allowed


def apply(action: VowAction, status: VowStatus) -> VowStatus:
    """Return the status after applying ``action`` to ``status``.

    Raises:
        IllegalVowTransition: If the action is not allowed from ``status``.
    """
    allowed, result = _TRANSITIONS[action]
    if status not in allowed:
        raise IllegalVowTransition(f"Cannot {action} a vow in status {status!r}")
    return result
