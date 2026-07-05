"""Rounded-card and gradient-background drawing (spec §15.2)."""

from __future__ import annotations

import pygame

from merit_ledger.frontend.theme import RGB, Palette


def draw_vertical_gradient(surface: pygame.Surface, top: RGB, bottom: RGB) -> None:
    """Fill ``surface`` with a smooth top→bottom vertical gradient."""
    height = surface.get_height()
    width = surface.get_width()
    for y in range(height):
        t = y / max(1, height - 1)
        color = (
            int(top[0] + (bottom[0] - top[0]) * t),
            int(top[1] + (bottom[1] - top[1]) * t),
            int(top[2] + (bottom[2] - top[2]) * t),
        )
        pygame.draw.line(surface, color, (0, y), (width, y))


def draw_card(
    surface: pygame.Surface, rect: pygame.Rect, palette: Palette, radius: int = 14
) -> None:
    """Draw a rounded card background using the palette's card color."""
    pygame.draw.rect(surface, palette.card, rect, border_radius=radius)
