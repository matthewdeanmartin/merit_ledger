"""Record a positive practice (spec §5.2)."""

from __future__ import annotations

from functools import partial
from typing import Any

import pygame

from merit_ledger.frontend.pygame_app import PygameApp
from merit_ledger.frontend.scenes.nav import MainScene
from merit_ledger.frontend.ui import cards
from merit_ledger.frontend.ui import text as text_ui
from merit_ledger.frontend.ui.buttons import Button


class RecordPracticeScene(MainScene):
    """Pick a practice template and record it as a ledger entry."""

    title = "Record"

    def __init__(self, app: PygameApp) -> None:
        """Init with no templates loaded and no confirmation."""
        super().__init__(app, active_tab="Record")
        self._templates: list[dict[str, Any]] = []
        self._buttons: list[Button] = []
        self._message = ""

    def on_enter(self) -> None:
        """Load templates for the active tradition and build a button per template."""
        try:
            self._templates = self.app.api.list_templates()
        except Exception:
            self._templates = []
        self._buttons = []
        y = self.BODY_TOP + 20
        for tmpl in self._templates[:8]:
            rect = pygame.Rect(20, y, 520, 44)
            self._buttons.append(
                Button(
                    f"{tmpl['name']}  (+{tmpl.get('default_points', 0)})",
                    rect,
                    partial(self._record, tmpl),
                    font_size=24,
                )
            )
            y += 52

    def _record(self, tmpl: dict[str, Any]) -> None:
        """Create an entry from a template and show confirmation."""
        entry = {
            "entry_type": "practice_completed",
            "template_id": tmpl["template_id"],
            "title": tmpl["name"],
            "quantity": tmpl.get("default_quantity"),
            "quantity_unit": tmpl.get("quantity_unit"),
        }
        try:
            saved = self.app.api.create_entry(entry)
            self._message = f"Recorded: {tmpl['name']} (+{saved.get('points', 0)})"
        except Exception:
            self._message = "Could not record — is the backend running?"

    def handle_body_event(self, event: pygame.event.Event) -> None:
        """Forward events to the template buttons."""
        for b in self._buttons:
            b.handle_event(event)

    def draw_body(self, surface: pygame.Surface) -> None:
        """Draw the template buttons and any confirmation message."""
        p = self.app.palette
        for b in self._buttons:
            b.draw(surface, p)
        if not self._templates:
            text_ui.draw_text(surface, "No practices available.", (20, self.BODY_TOP + 20), 24, p.text_muted)
        if self._message:
            rect = pygame.Rect(20, surface.get_height() - 70, surface.get_width() - 40, 46)
            cards.draw_card(surface, rect, p, radius=10)
            text_ui.draw_text(surface, self._message, (36, surface.get_height() - 58), 24, p.accent)
