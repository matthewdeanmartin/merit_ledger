"""Mudita garden — rejoice in sample wholesome actions (spec §5.9, §16)."""

from __future__ import annotations

from functools import partial
from typing import Any

import pygame

from merit_ledger.frontend.pygame_app import PygameApp
from merit_ledger.frontend.scenes.nav import MainScene
from merit_ledger.frontend.ui import cards
from merit_ledger.frontend.ui import text as text_ui
from merit_ledger.frontend.ui.buttons import Button


class MuditaGardenScene(MainScene):
    """A local garden of anonymous wholesome actions the user can rejoice in."""

    title = "Mudita Garden"

    def __init__(self, app: PygameApp) -> None:
        """Init empty."""
        super().__init__(app, active_tab="Mudita")
        self._feed: list[dict[str, Any]] = []
        self._verb = "Rejoice"
        self._buttons: list[Button] = []
        self._flowers: set[str] = set()  # sample_ids the user has rejoiced in this session

    def on_enter(self) -> None:
        """Load the sample feed + rejoice verb and build a button per entry."""
        try:
            data = self.app.api.mudita_feed()
            self._feed = data.get("entries", [])
            self._verb = data.get("verb", "Rejoice")
        except Exception:
            self._feed = []
        self._buttons = []
        y = self.BODY_TOP + 20
        for sample in self._feed[:6]:
            rect = pygame.Rect(self.app_width() - 160, y + 6, 120, 36)
            self._buttons.append(Button(self._verb, rect, partial(self._rejoice, sample), font_size=20))
            y += 58

    def app_width(self) -> int:
        """Current window width (fallback 960)."""
        return self.app.screen.get_width() if self.app.screen else 960

    def _rejoice(self, sample: dict[str, Any]) -> None:
        """Rejoice in a sample → local ledger entry + a flower marker."""
        try:
            self.app.api.mudita_rejoice({"sample_id": sample["sample_id"]})
            self._flowers.add(sample["sample_id"])
        except Exception:
            pass

    def handle_body_event(self, event: pygame.event.Event) -> None:
        """Forward events to rejoice buttons."""
        for b in self._buttons:
            b.handle_event(event)

    def draw_body(self, surface: pygame.Surface) -> None:
        """Draw each sample as a card; a small flower marks ones already rejoiced in."""
        p = self.app.palette
        y = self.BODY_TOP + 20
        for sample in self._feed[:6]:
            rect = pygame.Rect(20, y, self.app_width() - 200, 48)
            cards.draw_card(surface, rect, p, radius=10)
            text_ui.draw_text(surface, str(sample.get("text", "")), (36, y + 14), 22, p.text)
            if sample.get("sample_id") in self._flowers:
                # a simple "flower": a small accent circle (real animation is Sprint 7)
                pygame.draw.circle(surface, p.accent, (rect.right + 24, y + 24), 8)
            y += 58
        for b in self._buttons:
            b.draw(surface, p)
