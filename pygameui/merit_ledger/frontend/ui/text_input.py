"""A simple single-line text field: click to focus, type, backspace, blinking caret."""

from __future__ import annotations

import pygame

from merit_ledger.frontend.theme import Palette
from merit_ledger.frontend.ui import text as text_ui


class TextInput:
    """A focusable single-line text field. Rendering + key handling only, no app logic."""

    def __init__(
        self,
        rect: pygame.Rect,
        *,
        text: str = "",
        placeholder: str = "",
        max_length: int = 60,
        font_size: int = 26,
    ) -> None:
        """Create a text field.

        Args:
            rect: Screen rectangle.
            text: Initial text.
            placeholder: Shown (dimmed) when empty and unfocused.
            max_length: Maximum number of characters accepted.
            font_size: Text font size.
        """
        self.rect = rect
        self.text = text
        self.placeholder = placeholder
        self.max_length = max_length
        self.font_size = font_size
        self.focused = False
        self._caret_timer = 0.0

    def handle_event(self, event: pygame.event.Event) -> None:
        """Focus on click-inside (blur on click-outside) and edit text while focused."""
        if event.type == pygame.MOUSEBUTTONDOWN and event.button == 1:
            self.focused = self.rect.collidepoint(event.pos)
        elif event.type == pygame.KEYDOWN and self.focused:
            if event.key == pygame.K_BACKSPACE:
                self.text = self.text[:-1]
            elif event.key in (pygame.K_RETURN, pygame.K_KP_ENTER, pygame.K_ESCAPE):
                self.focused = False
            elif event.unicode and event.unicode.isprintable() and len(self.text) < self.max_length:
                self.text += event.unicode

    def update(self, dt: float) -> None:
        """Advance the caret blink timer."""
        self._caret_timer = (self._caret_timer + dt) % 1.0

    def draw(self, surface: pygame.Surface, palette: Palette) -> None:
        """Draw the field box, its text (or placeholder), and a blinking caret when focused."""
        pygame.draw.rect(surface, palette.card, self.rect, border_radius=8)
        border = palette.accent if self.focused else palette.text_muted
        pygame.draw.rect(surface, border, self.rect, width=2, border_radius=8)

        if self.text:
            content, color = self.text, palette.text
        else:
            content, color = self.placeholder, palette.text_muted
        text_ui.draw_text(surface, content, (self.rect.x + 10, self.rect.y + 8), self.font_size, color)

        if self.focused and self._caret_timer < 0.5:
            width = text_ui.render(self.text, self.font_size, palette.text).get_width()
            caret_x = self.rect.x + 10 + width + 2
            pygame.draw.line(
                surface,
                palette.text,
                (caret_x, self.rect.y + 8),
                (caret_x, self.rect.bottom - 8),
                2,
            )
