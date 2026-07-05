"""Theme resolution tests (pure, no display)."""

from __future__ import annotations

from merit_ledger.frontend.theme import palette_for_theme


def test_each_tradition_theme_resolves() -> None:
    for theme in ("ink_wash", "temple_gold", "dawn_gradient", "sunset_gold", "calm_blue"):
        p = palette_for_theme(theme)
        assert all(0 <= c <= 255 for c in p.bg_top)
        assert p.text != p.bg_top  # legible contrast at minimum


def test_unknown_theme_falls_back() -> None:
    default = palette_for_theme("nope")
    assert palette_for_theme("") == default
