"""Settings (spec §17): tradition, points mode, toggles, and data actions."""

from __future__ import annotations

from functools import partial
from typing import Any

import pygame

from merit_ledger.frontend.pygame_app import PygameApp
from merit_ledger.frontend.scenes.nav import MainScene, navigate
from merit_ledger.frontend.ui import text as text_ui
from merit_ledger.frontend.ui.buttons import Button

TRADITIONS = ["zen", "chinese_mahayana", "nichiren", "pure_land", "secular"]


class SettingsScene(MainScene):
    """Change tradition (live re-theme), points mode, toggles, and clear data."""

    title = "Settings"

    def __init__(self, app: PygameApp) -> None:
        """Init empty."""
        super().__init__(app, active_tab="Settings")
        self._settings: dict[str, Any] = {}
        self._buttons: list[Button] = []
        self._message = ""
        self._confirm_scope: str | None = None  # pending clear confirmation

    def on_enter(self) -> None:
        """Load settings and build tradition + data-action buttons."""
        try:
            self._settings = self.app.api.get_settings()
        except Exception:
            self._settings = {}
        self._rebuild()

    def _rebuild(self) -> None:
        """(Re)build the control buttons for the current state."""
        self._buttons = []
        x, y = 20, self.BODY_TOP + 60
        for trad in TRADITIONS:
            rect = pygame.Rect(x, y, 180, 40)
            self._buttons.append(Button(trad, rect, partial(self._set_tradition, trad), font_size=20))
            x += 190
            if x > 720:
                x, y = 20, y + 48

        # Data section: two clear scopes (spec §17.5) with a confirm step.
        dy = self.app_height() - 130
        if self._confirm_scope is None:
            self._buttons.append(Button("Clear user data", pygame.Rect(20, dy, 220, 44),
                                        partial(self._ask_clear, "user_data"), font_size=22))
            self._buttons.append(Button("Clear ALL data", pygame.Rect(252, dy, 220, 44),
                                        partial(self._ask_clear, "all"), font_size=22))
        else:
            self._buttons.append(Button("Confirm", pygame.Rect(20, dy, 160, 44),
                                        self._do_clear, font_size=22))
            self._buttons.append(Button("Cancel", pygame.Rect(192, dy, 160, 44),
                                        self._cancel_clear, font_size=22))

    def app_height(self) -> int:
        """Current window height (fallback 640)."""
        return self.app.screen.get_height() if self.app.screen else 640

    def _set_tradition(self, tradition: str) -> None:
        """Switch tradition, persist, and re-theme the app live."""
        try:
            self.app.api.set_tradition(tradition)
            self.app.refresh_palette()
            self._settings = self.app.api.get_settings()
            self._message = f"Tradition: {tradition}"
        except Exception:
            self._message = "Could not change tradition."

    def _ask_clear(self, scope: str) -> None:
        """Enter confirm mode for a clear-data action."""
        self._confirm_scope = scope
        self._rebuild()

    def _cancel_clear(self) -> None:
        """Leave confirm mode without clearing."""
        self._confirm_scope = None
        self._rebuild()

    def _do_clear(self) -> None:
        """Perform the pending clear. 'all' is a factory reset → back to splash."""
        scope = self._confirm_scope or "user_data"
        try:
            result = self.app.api.clear_data(scope=scope)
            deleted = result.get("deleted", 0)
        except Exception:
            deleted = 0
        self._confirm_scope = None
        if scope == "all":
            # Factory reset: onboarding must run again.
            navigate(self.app, "splash", "SplashScene")
            return
        self._message = f"Cleared {deleted} items (kept your settings)."
        self.on_enter()

    def handle_body_event(self, event: pygame.event.Event) -> None:
        """Forward events to control buttons."""
        for b in self._buttons:
            b.handle_event(event)

    def draw_body(self, surface: pygame.Surface) -> None:
        """Draw current tradition/points mode, controls, and any confirm prompt."""
        p = self.app.palette
        text_ui.draw_text(
            surface,
            f"Tradition: {self._settings.get('tradition', '?')}   ·   "
            f"Points: {self._settings.get('points_mode', '?')}",
            (20, self.BODY_TOP + 20),
            22,
            p.text_muted,
        )
        for b in self._buttons:
            b.draw(surface, p)
        if self._confirm_scope is not None:
            warn = (
                "This erases EVERYTHING and restarts onboarding."
                if self._confirm_scope == "all"
                else "This erases your practice ledger but keeps settings."
            )
            text_ui.draw_text(surface, warn, (20, self.app_height() - 168), 22, p.accent)
        if self._message:
            text_ui.draw_text(surface, self._message, (20, self.app_height() - 40), 22, p.text_muted)
