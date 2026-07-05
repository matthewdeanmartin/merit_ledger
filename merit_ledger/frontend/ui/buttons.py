"""Simple button widget with hover + click hit-testing."""

from __future__ import annotations

from collections.abc import Callable

import pygame

from merit_ledger.frontend.theme import Palette
from merit_ledger.frontend.ui import text as text_ui


class Button:
    """A rounded, labeled button. Rendering and hit-testing only — no app logic."""

    def __init__(
        self,
        label: str,
        rect: pygame.Rect,
        on_click: Callable[[], None],
        *,
        font_size: int = 28,
    ) -> None:
        """Create a button.

        Args:
            label: Text shown on the button.
            rect: Screen rectangle.
            on_click: Callback invoked when the button is clicked.
            font_size: Label font size.
        """
        self.label = label
        self.rect = rect
        self.on_click = on_click
        self.font_size = font_size
        self._hover = False

    def handle_event(self, event: pygame.event.Event) -> None:
        """Update hover state and fire ``on_click`` on a left-button release inside the rect."""
        if event.type == pygame.MOUSEMOTION:
            self._hover = self.rect.collidepoint(event.pos)
        elif event.type == pygame.MOUSEBUTTONUP and event.button == 1:
            if self.rect.collidepoint(event.pos):
                self.on_click()

    def draw(self, surface: pygame.Surface, palette: Palette) -> None:
        """Draw the button, brightening the accent slightly on hover."""
        base = palette.accent
        color = tuple(min(255, c + 25) for c in base) if self._hover else base
        pygame.draw.rect(surface, color, self.rect, border_radius=12)
        text_ui.draw_text_center(
            surface, self.label, self.rect.center, self.font_size, palette.card
        )
