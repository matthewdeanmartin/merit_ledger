"""Local application configuration constants."""

from __future__ import annotations

APP_NAME = "MeritLedger"
"""Application name used for the local data directory."""

BACKEND_HOST = "127.0.0.1"
"""Loopback host the local FastAPI backend binds to."""

BACKEND_PORT = 8765
"""Default backend port (see spec §14.1)."""

BACKEND_URL = f"http://{BACKEND_HOST}:{BACKEND_PORT}"
"""Base URL the Pygame frontend uses to reach the backend."""

DEFAULT_USER_ID = "local_user"
"""Single-user local profile id (see spec §13.3)."""

DB_FILENAME = "merit_ledger.sqlite3"
"""SQLite database filename inside the local data directory."""
