"""Sortable id and timestamp helpers.

Ids are lexicographically sortable by creation time so that ``SK`` values built from
them order chronologically, mirroring ULID behavior without an extra dependency.
"""

from __future__ import annotations

import datetime as _dt
import uuid


def now_iso() -> str:
    """Return the current UTC time as an ISO-8601 string with a trailing ``Z``."""
    return _dt.datetime.now(tz=_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def new_id(prefix: str) -> str:
    """Return a time-sortable id like ``entry_20260705T090000Z_1a2b3c4d``.

    Args:
        prefix: Short entity prefix, e.g. ``"entry"``, ``"vow"``, ``"dedication"``.
    """
    stamp = _dt.datetime.now(tz=_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    return f"{prefix}_{stamp}_{uuid.uuid4().hex[:8]}"
