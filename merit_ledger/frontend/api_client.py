"""Thin HTTP client the Pygame scenes use to reach the backend (spec §15, §23.4).

Scenes touch ONLY this client for data — no direct repository/service imports — so business
logic stays server-side and the frontend stays a pure view layer.
"""

from __future__ import annotations

from typing import Any

import httpx

from merit_ledger.local.config import BACKEND_URL


class ApiClient:
    """Small wrapper over the local backend's HTTP API."""

    def __init__(self, base_url: str = BACKEND_URL, client: httpx.Client | None = None) -> None:
        """Create a client.

        Args:
            base_url: Backend base URL.
            client: Optional injected httpx client (tests pass a TestClient-like object).
        """
        self._base = base_url.rstrip("/")
        self._client = client or httpx.Client(base_url=self._base, timeout=5.0)

    # --- profile / settings / traditions ------------------------------------

    def get_profile(self) -> dict[str, Any]:
        """Return the profile."""
        return self._get("/profile")

    def put_profile(self, profile: dict[str, Any]) -> dict[str, Any]:
        """Save the profile."""
        return self._put("/profile", profile)

    def get_settings(self) -> dict[str, Any]:
        """Return settings."""
        return self._get("/settings")

    def put_settings(self, settings: dict[str, Any]) -> dict[str, Any]:
        """Save settings."""
        return self._put("/settings", settings)

    def clear_data(self, scope: str = "all") -> dict[str, Any]:
        """Clear local data. ``scope`` is ``"all"`` (factory reset) or ``"user_data"``.

        ``"user_data"`` keeps profile + settings and only removes the practice ledger.
        """
        return self._post("/settings/clear", {"confirm": True, "scope": scope})

    def list_traditions(self) -> list[dict[str, Any]]:
        """List available traditions."""
        return self._get("/traditions")

    def get_tradition(self, tradition_id: str) -> dict[str, Any]:
        """Return a full tradition pack."""
        return self._get(f"/traditions/{tradition_id}")

    def set_tradition(self, tradition: str) -> dict[str, Any]:
        """Set the active tradition."""
        return self._put("/settings/tradition", {"tradition": tradition})

    # --- templates / entries / vows / stats ---------------------------------

    def list_templates(self) -> list[dict[str, Any]]:
        """List templates for the active tradition."""
        return self._get("/templates")

    def list_entries(self, **params: Any) -> list[dict[str, Any]]:
        """List ledger entries (optional date_prefix/entry_type/vow_id params)."""
        return self._get("/entries", params={k: v for k, v in params.items() if v is not None})

    def create_entry(self, entry: dict[str, Any], **scoring: Any) -> dict[str, Any]:
        """Create a ledger entry with optional scoring hints."""
        return self._post("/entries", {"entry": entry, **scoring})

    def list_vows(self, status: str | None = None) -> list[dict[str, Any]]:
        """List vows, optionally filtered by status."""
        params = {"status": status} if status else None
        return self._get("/vows", params=params)

    def get_vow(self, vow_id: str) -> dict[str, Any]:
        """Return a single vow."""
        return self._get(f"/vows/{vow_id}")

    def create_vow(self, vow: dict[str, Any]) -> dict[str, Any]:
        """Create a vow."""
        return self._post("/vows", vow)

    def vow_action(self, vow_id: str, action: str, body: dict[str, Any] | None = None) -> dict[str, Any]:
        """Run a vow lifecycle action: pause/resume/retire/complete/breach."""
        return self._post(f"/vows/{vow_id}/{action}", body or {})

    # --- repentance / dedication / mudita -----------------------------------

    def repentance_categories(self) -> dict[str, Any]:
        """Return repentance categories + the privacy reminder."""
        return self._get("/repentance/categories")

    def create_repentance(self, body: dict[str, Any]) -> dict[str, Any]:
        """Record a repentance ('return to practice') entry."""
        return self._post("/repentance", body)

    def list_dedications(self) -> list[dict[str, Any]]:
        """List dedications."""
        return self._get("/dedications")

    def dedication_presets(self) -> dict[str, Any]:
        """Return preset dedication targets + default text for the active tradition."""
        return self._get("/dedications/presets")

    def create_dedication(self, body: dict[str, Any]) -> dict[str, Any]:
        """Create a dedication."""
        return self._post("/dedications", body)

    def mudita_feed(self) -> dict[str, Any]:
        """Return the local sample feed + rejoice verb."""
        return self._get("/mudita/demo-feed")

    def mudita_rejoice(self, body: dict[str, Any]) -> dict[str, Any]:
        """Rejoice in a sample action → local ledger entry."""
        return self._post("/mudita/rejoice", body)

    # --- stats ---------------------------------------------------------------

    def stats_today(self) -> dict[str, Any]:
        """Return today's point/count totals."""
        return self._get("/stats/today")

    def stats_week(self) -> dict[str, Any]:
        """Return the trailing-week totals."""
        return self._get("/stats/week")

    def stats_month(self) -> dict[str, Any]:
        """Return the current-month totals."""
        return self._get("/stats/month")

    def stats_by_template(self) -> dict[str, int]:
        """Return points grouped by template."""
        return self._get("/stats/by-template")

    def stats_vows(self) -> dict[str, Any]:
        """Return vow status counts + streaks."""
        return self._get("/stats/vows")

    # --- low-level helpers ---------------------------------------------------

    def _get(self, path: str, params: dict[str, Any] | None = None) -> Any:
        resp = self._client.get(path, params=params)
        resp.raise_for_status()
        return resp.json()

    def _put(self, path: str, body: dict[str, Any]) -> Any:
        resp = self._client.put(path, json=body)
        resp.raise_for_status()
        return resp.json()

    def _post(self, path: str, body: dict[str, Any]) -> Any:
        resp = self._client.post(path, json=body)
        resp.raise_for_status()
        return resp.json()

    def close(self) -> None:
        """Close the underlying HTTP client."""
        self._client.close()
