"""Cached font loading and text rendering."""

from __future__ import annotations

from functools import lru_cache

import pygame

RGB = tuple[int, int, int]


@lru_cache(maxsize=16)
def _font(size: int) -> pygame.font.Font:
    """Return a cached default font at ``size`` (pygame's bundled font)."""
    return pygame.font.Font(None, size)


def render(text: str, size: int, color: RGB) -> pygame.Surface:
    """Render a single line of anti-aliased text to a surface."""
    return _font(size).render(text, True, color)


def draw_text(
    surface: pygame.Surface, text: str, pos: tuple[int, int], size: int, color: RGB
) -> pygame.Rect:
    """Blit ``text`` at ``pos`` (top-left) and return its bounding rect."""
    surf = render(text, size, color)
    rect = surf.get_rect(topleft=pos)
    surface.blit(surf, rect)
    return rect


def draw_text_center(
    surface: pygame.Surface, text: str, center: tuple[int, int], size: int, color: RGB
) -> pygame.Rect:
    """Blit ``text`` centered at ``center`` and return its bounding rect."""
    surf = render(text, size, color)
    rect = surf.get_rect(center=center)
    surface.blit(surf, rect)
    return rect
