"""Dashboard: today's total, active vows, recent entries (spec §15.4)."""

from __future__ import annotations

from typing import Any

import pygame

from merit_ledger.frontend.pygame_app import PygameApp
from merit_ledger.frontend.scenes.nav import MainScene
from merit_ledger.frontend.ui import cards
from merit_ledger.frontend.ui import text as text_ui


class DashboardScene(MainScene):
    """Home screen. Pulls today's stats, active vows, and recent entries via the API."""

    title = "Today"

    def __init__(self, app: PygameApp) -> None:
        """Init empty view state."""
        super().__init__(app, active_tab="Today")
        self._today: dict[str, Any] = {}
        self._active_vows = 0
        self._recent: list[dict[str, Any]] = []

    def on_enter(self) -> None:
        """Refresh dashboard data."""
        self.refresh()

    def refresh(self) -> None:
        """Load today's stats, active vow count, and recent entries."""
        try:
            self._today = self.app.api.stats_today()
            self._active_vows = len(self.app.api.list_vows(status="active"))
            entries = self.app.api.list_entries()
            self._recent = list(reversed(entries))[:6]
        except Exception:
            self._today = {"total_points": 0, "entry_count": 0}

    def draw_body(self, surface: pygame.Surface) -> None:
        """Draw today's totals, active-vows, and the recent entries list."""
        p = self.app.palette
        w = surface.get_width()

        points = self._today.get("total_points", 0)
        count = self._today.get("entry_count", 0)
        text_ui.draw_text(
            surface, f"{points} practice points · {count} entries today", (20, 132), 26, p.text_muted
        )
        text_ui.draw_text(
            surface, f"Active vows: {self._active_vows}", (w - 260, 132), 24, p.text_muted
        )

        y = 200
        text_ui.draw_text(surface, "Recent", (20, y), 30, p.text)
        y += 44
        if not self._recent:
            text_ui.draw_text(
                surface, "No entries yet — record your first practice.", (20, y), 24, p.text_muted
            )
        for e in self._recent:
            rect = pygame.Rect(20, y, w - 40, 48)
            cards.draw_card(surface, rect, p, radius=10)
            title = e.get("title") or e.get("entry_type", "entry")
            text_ui.draw_text(surface, str(title), (36, y + 12), 24, p.text)
            text_ui.draw_text(surface, f"+{e.get('points', 0)}", (w - 90, y + 12), 24, p.accent)
            y += 58
