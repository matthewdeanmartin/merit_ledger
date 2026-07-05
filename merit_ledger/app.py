"""Desktop app orchestrator (spec §22).

Startup: resolve data dir → ensure SQLite db → start FastAPI backend → wait for /health →
launch Pygame frontend → on close, shut down backend.
"""

from __future__ import annotations

from merit_ledger.frontend.api_client import ApiClient
from merit_ledger.local import data_dir, server_process


def run(with_ui: bool = True) -> int:
    """Run the full desktop app.

    Args:
        with_ui: When False, start the backend and wait for health, then stop (useful for
            smoke-testing startup without a display).

    Returns:
        Process exit code (0 on success, 1 if the backend never became healthy).
    """
    data_dir.db_path()  # ensure the data dir + db path exist before the server opens it

    server = server_process.BackendServer()
    server.start()
    try:
        if not server_process.wait_for_health():
            return 1
        if not with_ui:
            return 0
        _run_ui()
        return 0
    finally:
        server.stop()


def _run_ui() -> None:
    """Construct the Pygame app and enter the splash scene."""
    # Imported here so a headless/backend-only run never imports pygame.
    from merit_ledger.frontend.pygame_app import PygameApp
    from merit_ledger.frontend.scenes.splash import SplashScene

    api = ApiClient()
    app = PygameApp(api)
    try:
        app.run(SplashScene)
    finally:
        api.close()
