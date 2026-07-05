"""Tradition-aware color palettes (spec §6 themes, §15.2). Pure data — no pygame import.

Each tradition pack names a ``theme``; this maps that name to a :class:`Palette`. Palettes
are plain RGB tuples so this module is importable (and testable) without a display.
"""

from __future__ import annotations

from dataclasses import dataclass

RGB = tuple[int, int, int]


@dataclass(frozen=True)
class Palette:
    """A calm color set for one tradition (spec §15.2)."""

    bg_top: RGB
    bg_bottom: RGB
    card: RGB
    text: RGB
    text_muted: RGB
    accent: RGB


# Neutral fallback used when a theme name is unknown.
_DEFAULT = Palette(
    bg_top=(40, 44, 52),
    bg_bottom=(28, 30, 38),
    card=(52, 56, 66),
    text=(236, 238, 242),
    text_muted=(160, 166, 178),
    accent=(150, 190, 220),
)

_THEMES: dict[str, Palette] = {
    "ink_wash": Palette(  # Zen — stone, paper, moss
        bg_top=(238, 236, 230), bg_bottom=(214, 212, 205), card=(250, 249, 245),
        text=(44, 46, 44), text_muted=(120, 122, 118), accent=(90, 110, 96),
    ),
    "temple_gold": Palette(  # Chinese Mahayana — warm gold, deep red
        bg_top=(58, 26, 24), bg_bottom=(38, 16, 16), card=(80, 40, 34),
        text=(244, 232, 210), text_muted=(206, 176, 140), accent=(214, 168, 78),
    ),
    "dawn_gradient": Palette(  # Nichiren — dawn
        bg_top=(90, 70, 120), bg_bottom=(210, 130, 110), card=(70, 58, 92),
        text=(248, 244, 238), text_muted=(212, 200, 214), accent=(244, 196, 120),
    ),
    "sunset_gold": Palette(  # Pure Land — sunset gold, blue/gold
        bg_top=(48, 62, 110), bg_bottom=(196, 138, 78), card=(44, 54, 92),
        text=(250, 246, 236), text_muted=(206, 208, 220), accent=(240, 206, 130),
    ),
    "calm_blue": Palette(  # Secular — calm blue, green, journal
        bg_top=(232, 240, 244), bg_bottom=(210, 224, 228), card=(250, 252, 252),
        text=(38, 50, 56), text_muted=(110, 128, 134), accent=(80, 150, 150),
    ),
}


def palette_for_theme(theme: str) -> Palette:
    """Return the palette for a theme name, or the neutral default if unknown."""
    return _THEMES.get(theme, _DEFAULT)


def high_contrast(palette: Palette) -> Palette:
    """Return a higher-contrast variant (spec §17.4 accessibility)."""
    return Palette(
        bg_top=(0, 0, 0), bg_bottom=(0, 0, 0), card=(20, 20, 20),
        text=(255, 255, 255), text_muted=(220, 220, 220), accent=palette.accent,
    )
