"""Run the FastAPI backend in-process on a background thread (spec §14.1, §22).

An in-process uvicorn thread avoids child-process/packaging pain for a desktop app. The
SQLite repository is thread-safe, so sharing it between the server thread and the frontend
is fine. The frontend still talks to the backend over HTTP, exactly as the future cloud
version will.
"""

from __future__ import annotations

import threading
import time

import httpx
import uvicorn

from merit_ledger.local.config import BACKEND_HOST, BACKEND_PORT, BACKEND_URL


class BackendServer:
    """Owns a uvicorn server running the app on a daemon thread."""

    def __init__(self, host: str = BACKEND_HOST, port: int = BACKEND_PORT) -> None:
        """Configure (but do not start) the server."""
        config = uvicorn.Config(
            "merit_ledger.backend.main:app",
            host=host,
            port=port,
            log_level="warning",
        )
        self._server = uvicorn.Server(config)
        self._thread: threading.Thread | None = None

    def start(self) -> None:
        """Start the server thread (idempotent)."""
        if self._thread is not None:
            return
        self._thread = threading.Thread(target=self._server.run, name="merit-backend", daemon=True)
        self._thread.start()

    def stop(self) -> None:
        """Ask the server to exit and join its thread."""
        self._server.should_exit = True
        if self._thread is not None:
            self._thread.join(timeout=5)
            self._thread = None


def wait_for_health(url: str = BACKEND_URL, timeout: float = 10.0) -> bool:
    """Poll ``{url}/health`` until it responds ok or ``timeout`` seconds elapse.

    Returns:
        True if the backend became healthy in time, else False.
    """
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            resp = httpx.get(f"{url}/health", timeout=1.0)
            if resp.status_code == 200 and resp.json().get("status") == "ok":
                return True
        except httpx.HTTPError:
            pass
        time.sleep(0.1)
    return False
