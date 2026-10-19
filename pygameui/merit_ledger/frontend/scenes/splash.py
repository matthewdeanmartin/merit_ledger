"""Splash scene: calm title card that routes to onboarding or dashboard (spec §5.1)."""

from __future__ import annotations

from typing import Any

import pygame

from merit_ledger.frontend.pygame_app import PygameApp, Scene
from merit_ledger.frontend.ui import cards
from merit_ledger.frontend.ui import text as text_ui

SPLASH_SECONDS = 1.5


def next_scene_name(settings: dict[str, Any]) -> str:
    """Decide where to go after the splash (pure — unit tested).

    Returns ``"dashboard"`` if the user has completed onboarding, else ``"onboarding"``.
    """
    return "dashboard" if settings.get("onboarded") else "onboarding"


class SplashScene(Scene):
    """Shows the app name briefly, then advances."""

    def __init__(self, app: PygameApp) -> None:
        """Init timing."""
        super().__init__(app)
        self._elapsed = 0.0

    def update(self, dt: float) -> None:
        """Advance the timer and route once the splash time elapses."""
        self._elapsed += dt
        if self._elapsed >= SPLASH_SECONDS:
            self._advance()

    def _advance(self) -> None:
        """Route to the next scene based on onboarding state."""
        # Imported lazily to avoid a circular import at module load.
        from merit_ledger.frontend.scenes.dashboard import DashboardScene
        from merit_ledger.frontend.scenes.onboarding import OnboardingScene

        settings = self.app.api.get_settings()
        if next_scene_name(settings) == "dashboard":
            self.app.switch_to(DashboardScene(self.app))
        else:
            self.app.switch_to(OnboardingScene(self.app))

    def draw(self, surface: pygame.Surface) -> None:
        """Draw the centered title over the tradition gradient."""
        p = self.app.palette
        cards.draw_vertical_gradient(surface, p.bg_top, p.bg_bottom)
        cx, cy = surface.get_width() // 2, surface.get_height() // 2
        text_ui.draw_text_center(surface, "Merit Ledger", (cx, cy - 20), 64, p.text)
        text_ui.draw_text_center(
            surface, "A quiet place to record wholesome practice.", (cx, cy + 40), 26, p.text_muted
        )
