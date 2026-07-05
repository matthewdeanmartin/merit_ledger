"""Dedicate merit / intention (spec §5.8)."""

from __future__ import annotations

from functools import partial
from typing import Any

import pygame

from merit_ledger.frontend.pygame_app import PygameApp
from merit_ledger.frontend.scenes.nav import MainScene
from merit_ledger.frontend.ui import cards
from merit_ledger.frontend.ui import text as text_ui
from merit_ledger.frontend.ui.buttons import Button


class DedicationScene(MainScene):
    """Choose a preset target and dedicate merit to it."""

    title = "Dedicate"

    def __init__(self, app: PygameApp) -> None:
        """Init empty."""
        super().__init__(app, active_tab="Dedicate")
        self._targets: list[dict[str, Any]] = []
        self._default_text = ""
        self._buttons: list[Button] = []
        self._message = ""

    def on_enter(self) -> None:
        """Load preset targets + default text and build a button per target."""
        try:
            presets = self.app.api.dedication_presets()
            self._targets = presets.get("targets", [])
            self._default_text = presets.get("default_text", "")
        except Exception:
            self._targets = []
        self._buttons = []
        y = self.BODY_TOP + 70
        for target in self._targets:
            rect = pygame.Rect(20, y, 420, 42)
            self._buttons.append(Button(target["target_name"], rect, partial(self._save, target), font_size=22))
            y += 50

    def _save(self, target: dict[str, Any]) -> None:
        """Create a dedication to the chosen target."""
        body = {
            "target_type": target.get("target_type", "generic_group"),
            "target_name": target["target_name"],
            "dedication_text": self._default_text,
        }
        try:
            self.app.api.create_dedication(body)
            self._message = f"Dedicated to {target['target_name']}."
        except Exception:
            self._message = "Could not save."

    def handle_body_event(self, event: pygame.event.Event) -> None:
        """Forward events to target buttons."""
        for b in self._buttons:
            b.handle_event(event)

    def draw_body(self, surface: pygame.Surface) -> None:
        """Draw default text, target buttons, and confirmation."""
        p = self.app.palette
        if self._default_text:
            text_ui.draw_text(surface, self._default_text, (20, self.BODY_TOP + 24), 22, p.text_muted)
        for b in self._buttons:
            b.draw(surface, p)
        if self._message:
            rect = pygame.Rect(20, surface.get_height() - 70, surface.get_width() - 40, 46)
            cards.draw_card(surface, rect, p, radius=10)
            text_ui.draw_text(surface, self._message, (36, surface.get_height() - 58), 26, p.accent)
