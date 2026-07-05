"""Repentance / return to practice (spec §5.7). Non-secret, category-based."""

from __future__ import annotations

from functools import partial
from typing import Any

import pygame

from merit_ledger.frontend.pygame_app import PygameApp
from merit_ledger.frontend.scenes.nav import MainScene
from merit_ledger.frontend.ui import cards
from merit_ledger.frontend.ui import text as text_ui
from merit_ledger.frontend.ui.buttons import Button


class RepentanceScene(MainScene):
    """Choose a category and record a return-to-practice entry.

    When navigated from a vow breach, ``linked_vow_id`` and ``category`` are prefilled.
    """

    title = "Return to practice"

    def __init__(
        self,
        app: PygameApp,
        linked_vow_id: str | None = None,
        category: str | None = None,
    ) -> None:
        """Init, optionally prefilled from a breach."""
        super().__init__(app, active_tab="Repent")
        self.linked_vow_id = linked_vow_id
        self.category = category
        self._categories: list[str] = []
        self._privacy = ""
        self._buttons: list[Button] = []
        self._message = ""

    def on_enter(self) -> None:
        """Load categories + privacy reminder and build a button per category."""
        try:
            data = self.app.api.repentance_categories()
            self._categories = data.get("categories", [])
            self._privacy = data.get("privacy_reminder", "")
        except Exception:
            self._categories = []
        self._buttons = []
        x, y = 20, self.BODY_TOP + 70
        for cat in self._categories:
            rect = pygame.Rect(x, y, 180, 40)
            self._buttons.append(Button(cat, rect, partial(self._save, cat), font_size=20))
            x += 190
            if x > 720:
                x = 20
                y += 48

    def _save(self, category: str) -> None:
        """Record a repentance entry in ``category`` (+ link if from a breach)."""
        body: dict[str, Any] = {"category": category}
        if self.linked_vow_id:
            body["linked_vow_id"] = self.linked_vow_id
        try:
            self.app.api.create_repentance(body)
            self._message = "Returned to practice."
        except Exception:
            self._message = "Could not save."

    def handle_body_event(self, event: pygame.event.Event) -> None:
        """Forward events to category buttons."""
        for b in self._buttons:
            b.handle_event(event)

    def draw_body(self, surface: pygame.Surface) -> None:
        """Draw the privacy reminder, category buttons, and any confirmation."""
        p = self.app.palette
        # Privacy reminder shown prominently (spec §5.7).
        text_ui.draw_text(surface, self._privacy, (20, self.BODY_TOP + 24), 20, p.text_muted)
        for b in self._buttons:
            b.draw(surface, p)
        if self._message:
            rect = pygame.Rect(20, surface.get_height() - 70, surface.get_width() - 40, 46)
            cards.draw_card(surface, rect, p, radius=10)
            text_ui.draw_text(surface, self._message, (36, surface.get_height() - 58), 26, p.accent)
