"""Repentance / return to practice (spec §5.7). Non-secret, category-based.

Pick a category, optionally add a *general* reflection and a repair intention, then save.
There are deliberately no who/what/where fields (spec §2.4) — only the category, a
non-specific reflection, and an intention.
"""

from __future__ import annotations

from functools import partial
from typing import Any

import pygame

from merit_ledger.frontend.pygame_app import PygameApp
from merit_ledger.frontend.scenes.nav import MainScene
from merit_ledger.frontend.ui import cards
from merit_ledger.frontend.ui import text as text_ui
from merit_ledger.frontend.ui.buttons import Button
from merit_ledger.frontend.ui.text_input import TextInput


class RepentanceScene(MainScene):
    """Choose a category, optionally reflect + set a repair intention, then save.

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
        self._cat_buttons: list[Button] = []
        self._reflection: TextInput | None = None
        self._intention: TextInput | None = None
        self._save_button: Button | None = None
        self._message = ""

    def on_enter(self) -> None:
        """Load categories + privacy reminder and build the category picker + fields."""
        try:
            data = self.app.api.repentance_categories()
            self._categories = data.get("categories", [])
            self._privacy = data.get("privacy_reminder", "")
        except Exception:
            self._categories = []

        self._cat_buttons = []
        x, y = 20, self.BODY_TOP + 60
        for cat in self._categories:
            rect = pygame.Rect(x, y, 170, 38)
            self._cat_buttons.append(Button(cat, rect, partial(self._pick, cat), font_size=20))
            x += 180
            if x > 700:
                x = 20
                y += 44

        self._reflection = TextInput(
            pygame.Rect(20, self.BODY_TOP + 230, 900, 40),
            placeholder="optional reflection — keep it general, no names or specifics",
            max_length=140, font_size=22,
        )
        self._intention = TextInput(
            pygame.Rect(20, self.BODY_TOP + 290, 900, 40),
            placeholder="repair intention — e.g. 'I will pause before replying'",
            max_length=140, font_size=22,
        )
        self._save_button = Button(
            "Return to practice", pygame.Rect(20, self.BODY_TOP + 344, 280, 46), self._save, font_size=24
        )

    def _pick(self, category: str) -> None:
        """Select a repentance category."""
        self.category = category
        self._message = ""

    def _save(self) -> None:
        """Record a repentance entry with the selected category + optional fields."""
        if not self.category:
            self._message = "Choose a category first."
            return
        body: dict[str, Any] = {"category": self.category}
        if self.linked_vow_id:
            body["linked_vow_id"] = self.linked_vow_id
        if self._reflection is not None and self._reflection.text.strip():
            body["reflection"] = self._reflection.text.strip()
        if self._intention is not None and self._intention.text.strip():
            body["repair_intention"] = self._intention.text.strip()
        try:
            self.app.api.create_repentance(body)
            self._message = "Returned to practice."
        except Exception:
            self._message = "Could not save."

    def handle_body_event(self, event: pygame.event.Event) -> None:
        """Forward events to category buttons, fields, and save."""
        for b in self._cat_buttons:
            b.handle_event(event)
        for field in (self._reflection, self._intention):
            if field is not None:
                field.handle_event(event)
        if self._save_button is not None:
            self._save_button.handle_event(event)

    def update(self, dt: float) -> None:
        """Blink the field carets."""
        for field in (self._reflection, self._intention):
            if field is not None:
                field.update(dt)

    def draw_body(self, surface: pygame.Surface) -> None:
        """Draw the privacy reminder, category picker, fields, and confirmation."""
        p = self.app.palette
        # Privacy reminder shown prominently (spec §5.7).
        text_ui.draw_text(surface, self._privacy, (20, self.BODY_TOP + 20), 20, p.text_muted)
        for b in self._cat_buttons:
            # highlight the selected category by drawing a ring
            if b.label == self.category:
                pygame.draw.rect(surface, p.text, b.rect.inflate(6, 6), width=2, border_radius=12)
            b.draw(surface, p)
        selected = self.category or "none"
        text_ui.draw_text(surface, f"Category: {selected}", (20, self.BODY_TOP + 200), 20, p.text)
        for field in (self._reflection, self._intention):
            if field is not None:
                field.draw(surface, p)
        if self._save_button is not None:
            self._save_button.draw(surface, p)

        if self._message:
            rect = pygame.Rect(340, self.BODY_TOP + 344, surface.get_width() - 360, 46)
            cards.draw_card(surface, rect, p, radius=10)
            text_ui.draw_text(surface, self._message, (356, self.BODY_TOP + 356), 24, p.accent)
