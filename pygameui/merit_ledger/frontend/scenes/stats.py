"""Stats overview (spec §15). Simple text/bar rows — no chart library."""

from __future__ import annotations

from typing import Any

import pygame

from merit_ledger.frontend.pygame_app import PygameApp
from merit_ledger.frontend.scenes.nav import MainScene
from merit_ledger.frontend.ui import text as text_ui


class StatsScene(MainScene):
    """Show today/week/month totals, points by template, and a vow summary."""

    title = "Stats"

    def __init__(self, app: PygameApp) -> None:
        """Init empty."""
        super().__init__(app, active_tab="Stats")
        self._today: dict[str, Any] = {}
        self._week: dict[str, Any] = {}
        self._month: dict[str, Any] = {}
        self._by_template: dict[str, int] = {}
        self._vows: dict[str, Any] = {}

    def on_enter(self) -> None:
        """Load all stat views."""
        try:
            self._today = self.app.api.stats_today()
            self._week = self.app.api.stats_week()
            self._month = self.app.api.stats_month()
            self._by_template = self.app.api.stats_by_template()
            self._vows = self.app.api.stats_vows()
        except Exception:
            pass

    def draw_body(self, surface: pygame.Surface) -> None:
        """Draw the three period totals, top templates, and vow status counts."""
        p = self.app.palette
        y = self.BODY_TOP + 16

        def total(d: dict[str, Any]) -> int:
            return int(d.get("total_points", 0))

        text_ui.draw_text(
            surface,
            f"Today {total(self._today)}   ·   Week {total(self._week)}   ·   Month {total(self._month)}",
            (20, y),
            28,
            p.text,
        )
        y += 56

        text_ui.draw_text(surface, "Points by practice", (20, y), 24, p.text)
        y += 36
        top = sorted(self._by_template.items(), key=lambda kv: kv[1], reverse=True)[:6]
        max_pts = max((v for _, v in top), default=1)
        for name, pts in top:
            bar_w = int(300 * pts / max(1, max_pts))
            pygame.draw.rect(surface, p.accent, pygame.Rect(20, y + 6, bar_w, 16), border_radius=6)
            text_ui.draw_text(surface, f"{name}  {pts}", (330, y), 20, p.text_muted)
            y += 30

        y += 20
        text_ui.draw_text(surface, "Vows", (20, y), 24, p.text)
        y += 34
        by_status = self._vows.get("by_status", {})
        if not by_status:
            text_ui.draw_text(surface, "No vows yet.", (20, y), 20, p.text_muted)
        for status, count in by_status.items():
            text_ui.draw_text(surface, f"{status}: {count}", (20, y), 20, p.text_muted)
            y += 26
