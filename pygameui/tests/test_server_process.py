"""Backend server lifecycle: start → /health → stop.

Marked integration because it binds a real socket and spins a uvicorn thread.
"""

from __future__ import annotations

import socket

import pytest

from merit_ledger.local import server_process
from merit_ledger.local.config import BACKEND_HOST


def _free_port() -> int:
    """Grab an unused TCP port from the OS."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind((BACKEND_HOST, 0))
        return int(s.getsockname()[1])


@pytest.mark.integration
def test_start_health_stop() -> None:
    port = _free_port()
    server = server_process.BackendServer(port=port)
    server.start()
    try:
        url = f"http://{BACKEND_HOST}:{port}"
        assert server_process.wait_for_health(url, timeout=10.0) is True
    finally:
        server.stop()
