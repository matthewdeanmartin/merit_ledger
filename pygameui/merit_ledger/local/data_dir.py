"""Resolve and create the local data directory (spec §22)."""

from __future__ import annotations

from pathlib import Path

import platformdirs

from merit_ledger.local.config import APP_NAME, DB_FILENAME


def data_dir() -> Path:
    """Return the per-user local data directory, creating it if needed.

    Uses platformdirs so the location is correct on Windows, macOS, and Linux.

    Returns:
        Path to the (existing) data directory.
    """
    directory = Path(platformdirs.user_data_dir(APP_NAME, appauthor=False))
    directory.mkdir(parents=True, exist_ok=True)
    return directory


def db_path() -> Path:
    """Return the path to the SQLite database file (parent dir is created)."""
    return data_dir() / DB_FILENAME
