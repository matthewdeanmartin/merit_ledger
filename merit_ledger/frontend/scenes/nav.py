"""Shared navigation: a tab bar + a base scene that routes between main screens.

Scene classes are resolved lazily by name to avoid import cycles (scenes import this
module, and this module switches to scenes).
"""

from __future__ import annotations

import pygame

from merit_ledger.frontend.pygame_app import PygameApp, Scene
from merit_ledger.frontend.ui import cards
from merit_ledger.frontend.ui import text as text_ui
from merit_ledger.frontend.ui.buttons import Button

# Tab label → (scene module, scene class). "Today" is the dashboard.
TABS: list[tuple[str, str, str]] = [
    ("Today", "dashboard", "DashboardScene"),
    ("Record", "record_practice", "RecordPracticeScene"),
    ("Vows", "vows", "VowsScene"),
    ("Repent", "repentance", "RepentanceScene"),
    ("Dedicate", "dedication", "DedicationScene"),
    ("Mudita", "mudita_garden", "MuditaGardenScene"),
    ("Stats", "stats", "StatsScene"),
    ("Settings", "settings", "SettingsScene"),
]


def _resolve(module: str, cls: str) -> type[Scene]:
    """Import and return a scene class by module + class name (lazy)."""
    import importlib

    mod = importlib.import_module(f"merit_ledger.frontend.scenes.{module}")
    return getattr(mod, cls)  # type: ignore[no-any-return]


def navigate(app: PygameApp, module: str, cls: str) -> None:
    """Switch the app to the named scene."""
    app.switch_to(_resolve(module, cls)(app))


class MainScene(Scene):
    """Base for top-level tabbed screens. Draws a nav bar and routes tab clicks.

    Subclasses set ``title`` and implement ``draw_body`` / ``on_enter`` /
    ``handle_body_event``. Content is drawn below the nav bar (``BODY_TOP``).
    """

    NAV_Y = 20
    BODY_TOP = 84
    title = ""

    def __init__(self, app: PygameApp, active_tab: str) -> None:
        """Build the nav bar, marking ``active_tab`` as current."""
        super().__init__(app)
        self.active_tab = active_tab
        self._nav_buttons: list[Button] = []
        x = 20
        for label, module, cls in TABS:
            rect = pygame.Rect(x, self.NAV_Y, 110, 40)
            self._nav_buttons.append(
                Button(label, rect, self._make_nav(label, module, cls), font_size=20)
            )
            x += 116

    def _make_nav(self, label: str, module: str, cls: str):  # type: ignore[no-untyped-def]
        """Return a click handler that navigates unless already on that tab."""
        def go() -> None:
            if label != self.active_tab:
                navigate(self.app, module, cls)

        return go

    # --- hooks subclasses may override --------------------------------------

    def handle_body_event(self, event: pygame.event.Event) -> None:
        """Handle events for the scene body (below the nav bar)."""

    def draw_body(self, surface: pygame.Surface) -> None:
        """Draw the scene body (below the nav bar)."""

    # --- Scene interface -----------------------------------------------------

    def handle_event(self, event: pygame.event.Event) -> None:
        """Route to nav buttons, then the body."""
        for b in self._nav_buttons:
            b.handle_event(event)
        self.handle_body_event(event)

    def draw(self, surface: pygame.Surface) -> None:
        """Draw the gradient, nav bar, title, then the body."""
        p = self.app.palette
        cards.draw_vertical_gradient(surface, p.bg_top, p.bg_bottom)
        for b in self._nav_buttons:
            b.draw(surface, p)
        if self.title:
            text_ui.draw_text(surface, self.title, (20, self.BODY_TOP - 12), 40, p.text)
        self.draw_body(surface)
