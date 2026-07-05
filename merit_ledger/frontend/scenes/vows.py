"""Vows list + quick create (spec §5.3-§5.6, §15.5)."""

from __future__ import annotations

from functools import partial
from typing import Any

import pygame

from merit_ledger.frontend.pygame_app import PygameApp
from merit_ledger.frontend.scenes.nav import MainScene
from merit_ledger.frontend.ui import cards
from merit_ledger.frontend.ui import text as text_ui
from merit_ledger.frontend.ui.buttons import Button

# Paused vows are dimmed, not red (spec §15.5); repair shows "repair available".
_DIMMED = {"paused", "retired"}


class VowsScene(MainScene):
    """List vows (active first) and create a new one."""

    title = "Vows"

    def __init__(self, app: PygameApp) -> None:
        """Init empty."""
        super().__init__(app, active_tab="Vows")
        self._vows: list[dict[str, Any]] = []
        self._row_buttons: list[Button] = []
        self._create_buttons: list[Button] = []

    def on_enter(self) -> None:
        """Load all vows and build row + create buttons."""
        try:
            self._vows = self.app.api.list_vows()
        except Exception:
            self._vows = []
        self._vows.sort(key=lambda v: (v.get("status") != "active", v.get("name", "")))

        self._row_buttons = []
        y = self.BODY_TOP + 20
        for vow in self._vows[:7]:
            self._row_buttons.append(Button("Open", pygame.Rect(560, y + 6, 80, 34),
                                            partial(self._open, vow["vow_id"]), font_size=20))
            y += 54

        # Two quick-create buttons at the bottom.
        self._create_buttons = [
            Button("+ Positive vow", pygame.Rect(20, y + 10, 200, 44),
                   partial(self._create, "positive"), font_size=22),
            Button("+ Negative vow", pygame.Rect(232, y + 10, 200, 44),
                   partial(self._create, "negative"), font_size=22),
        ]

    def _open(self, vow_id: str) -> None:
        """Navigate to the vow detail scene for ``vow_id``."""
        from merit_ledger.frontend.scenes.vow_detail import VowDetailScene

        self.app.switch_to(VowDetailScene(self.app, vow_id))

    def _create(self, vow_type: str) -> None:
        """Create a starter vow, then open its detail scene so the user can name it."""
        name = "New commitment" if vow_type == "positive" else "New restraint"
        try:
            vow = self.app.api.create_vow({"name": name, "vow_type": vow_type})
        except Exception:
            self.on_enter()
            return
        self._open(vow["vow_id"])

    def handle_body_event(self, event: pygame.event.Event) -> None:
        """Forward events to row + create buttons."""
        for b in (*self._row_buttons, *self._create_buttons):
            b.handle_event(event)

    def draw_body(self, surface: pygame.Surface) -> None:
        """Draw each vow as a card with name, status, streak."""
        p = self.app.palette
        y = self.BODY_TOP + 20
        if not self._vows:
            text_ui.draw_text(surface, "No vows yet — create one below.", (20, y), 24, p.text_muted)
        for vow in self._vows[:7]:
            rect = pygame.Rect(20, y, 620, 46)
            cards.draw_card(surface, rect, p, radius=10)
            dim = vow.get("status") in _DIMMED
            color = p.text_muted if dim else p.text
            text_ui.draw_text(surface, str(vow.get("name", "")), (36, y + 12), 24, color)
            status = vow.get("status", "")
            label = "repair available" if status == "repair_in_progress" else status
            text_ui.draw_text(surface, label, (300, y + 14), 20, p.text_muted)
            text_ui.draw_text(surface, f"{vow.get('default_points', 0)} pts", (420, y + 14), 20, p.text_muted)
            if vow.get("streak"):
                text_ui.draw_text(surface, f"streak {vow['streak']}", (500, y + 14), 20, p.accent)
            y += 54
        for b in self._row_buttons + self._create_buttons:
            b.draw(surface, p)
