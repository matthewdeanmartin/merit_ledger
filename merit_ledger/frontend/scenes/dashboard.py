"""Dashboard: today's total, active vows, quick nav, recent entries (spec §15.4)."""

from __future__ import annotations

from functools import partial
from typing import Any

import pygame

from merit_ledger.frontend.pygame_app import PygameApp, Scene
from merit_ledger.frontend.ui import cards
from merit_ledger.frontend.ui import text as text_ui
from merit_ledger.frontend.ui.buttons import Button

# Quick-nav tabs (spec §15.3). Scenes for these land in Sprint 6; unimplemented ones are
# no-ops for now so the shell is navigable without crashing.
NAV = ["Record", "Vows", "Repent", "Dedicate", "Mudita", "Stats", "Settings"]


class DashboardScene(Scene):
    """Home screen. Pulls today's stats, active vows, and recent entries via the API."""

    def __init__(self, app: PygameApp) -> None:
        """Init empty view state."""
        super().__init__(app)
        self._today: dict[str, Any] = {}
        self._active_vows = 0
        self._recent: list[dict[str, Any]] = []
        self._buttons: list[Button] = []

    def on_enter(self) -> None:
        """Refresh dashboard data and (re)build the nav buttons."""
        self.refresh()
        self._buttons = []
        x = 40
        for name in NAV:
            rect = pygame.Rect(x, 150, 118, 44)
            self._buttons.append(Button(name, rect, partial(self._nav, name), font_size=22))
            x += 128

    def refresh(self) -> None:
        """Load today's stats, active vow count, and recent entries."""
        try:
            self._today = self.app.api.stats_today()
            self._active_vows = len(self.app.api.list_vows(status="active"))
            entries = self.app.api.list_entries()
            self._recent = list(reversed(entries))[:6]
        except Exception:  # noqa: BLE001 - keep the dashboard drawable on API hiccups
            self._today = {"total_points": 0, "entry_count": 0}

    def _nav(self, name: str) -> None:
        """Navigate to a tab (Sprint 6 wires the remaining scenes)."""
        # Placeholder: real scene switches added in Sprint 6. Kept as a no-op so the
        # shell is complete and clickable now.

    def handle_event(self, event: pygame.event.Event) -> None:
        """Forward events to nav buttons."""
        for b in self._buttons:
            b.handle_event(event)

    def draw(self, surface: pygame.Surface) -> None:
        """Draw the header, today's total card, active-vows, nav, and recent entries."""
        p = self.app.palette
        cards.draw_vertical_gradient(surface, p.bg_top, p.bg_bottom)
        w = surface.get_width()

        text_ui.draw_text(surface, "Today", (40, 40), 48, p.text)
        points = self._today.get("total_points", 0)
        count = self._today.get("entry_count", 0)
        text_ui.draw_text(
            surface, f"{points} practice points · {count} entries today", (40, 100), 26, p.text_muted
        )
        text_ui.draw_text(
            surface, f"Active vows: {self._active_vows}", (w - 260, 100), 26, p.text_muted
        )

        for b in self._buttons:
            b.draw(surface, p)

        # Recent entries list
        y = 230
        text_ui.draw_text(surface, "Recent", (40, y), 30, p.text)
        y += 44
        if not self._recent:
            text_ui.draw_text(surface, "No entries yet — record your first practice.", (40, y), 24, p.text_muted)
        for e in self._recent:
            rect = pygame.Rect(40, y, w - 80, 48)
            cards.draw_card(surface, rect, p, radius=10)
            title = e.get("title") or e.get("entry_type", "entry")
            text_ui.draw_text(surface, str(title), (56, y + 12), 24, p.text)
            text_ui.draw_text(surface, f"+{e.get('points', 0)}", (w - 110, y + 12), 24, p.accent)
            y += 58
