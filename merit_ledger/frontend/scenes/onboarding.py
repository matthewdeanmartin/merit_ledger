"""Onboarding flow: mode → point style → privacy → name (spec §5.1)."""

from __future__ import annotations

import pygame

from merit_ledger.frontend.pygame_app import PygameApp, Scene
from merit_ledger.frontend.ui import cards
from merit_ledger.frontend.ui import text as text_ui
from merit_ledger.frontend.ui.buttons import Button

TRADITIONS = [
    ("zen", "Zen"),
    ("chinese_mahayana", "Chinese Mahayana"),
    ("nichiren", "Nichiren"),
    ("pure_land", "Pure Land"),
    ("secular", "Secular"),
    ("custom", "Custom / Mixed"),
]

POINT_STYLES = [
    ("points", "Points enabled"),
    ("count_only", "Count only"),
    ("reflection_only", "Reflection only"),
]

WELCOME = (
    "Merit Ledger helps you record wholesome actions, vows, dedications, "
    "repentance, and rejoicing. It is a practice tool, not a judge."
)
PRIVACY = "Local-only. Please do not record secrets, names, or identifying details."


class OnboardingScene(Scene):
    """Collects tradition, point style, and a profile name, then persists them."""

    def __init__(self, app: PygameApp) -> None:
        """Start at the first step with no selections."""
        super().__init__(app)
        self.step = 0  # 0=mode, 1=points, 2=name/privacy
        self.tradition = "secular"
        self.points_mode = "points"
        self.name = "Practitioner"
        self._buttons: list[Button] = []

    def on_enter(self) -> None:
        """Build buttons for the current step."""
        self._rebuild()

    # --- step wiring ---------------------------------------------------------

    def _rebuild(self) -> None:
        """Recreate the option buttons for the active step."""
        self._buttons = []
        y = 220
        if self.step == 0:
            for tid, label in TRADITIONS:
                self._buttons.append(self._option(label, y, lambda t=tid: self._choose_tradition(t)))
                y += 56
        elif self.step == 1:
            for mode, label in POINT_STYLES:
                self._buttons.append(self._option(label, y, lambda m=mode: self._choose_points(m)))
                y += 56
        else:
            self._buttons.append(self._option("Begin practice", y + 40, self._finish))

    def _option(self, label: str, y: int, cb) -> Button:  # type: ignore[no-untyped-def]
        """Create a centered option button at vertical position ``y``."""
        width = 420
        rect = pygame.Rect((960 - width) // 2, y, width, 46)
        return Button(label, rect, cb, font_size=26)

    def _choose_tradition(self, tradition: str) -> None:
        self.tradition = tradition
        self.step = 1
        self._rebuild()

    def _choose_points(self, mode: str) -> None:
        self.points_mode = mode
        self.step = 2
        self._rebuild()

    def _finish(self) -> None:
        """Persist choices to the backend and go to the dashboard."""
        self.app.api.set_tradition(self.tradition)
        settings = self.app.api.get_settings()
        settings["tradition"] = self.tradition
        settings["points_mode"] = self.points_mode
        settings["onboarded"] = True
        self.app.api.put_settings(settings)
        profile = self.app.api.get_profile()
        profile["name"] = self.name
        profile["tradition"] = self.tradition
        self.app.api.put_profile(profile)
        self.app.refresh_palette()

        from merit_ledger.frontend.scenes.dashboard import DashboardScene

        self.app.switch_to(DashboardScene(self.app))

    # --- pygame hooks --------------------------------------------------------

    def handle_event(self, event: pygame.event.Event) -> None:
        """Route input to option buttons; allow typing the name on the last step."""
        for b in self._buttons:
            b.handle_event(event)
        if self.step == 2 and event.type == pygame.KEYDOWN:
            if event.key == pygame.K_BACKSPACE:
                self.name = self.name[:-1]
            elif event.unicode and event.unicode.isprintable() and len(self.name) < 24:
                self.name += event.unicode

    def draw(self, surface: pygame.Surface) -> None:
        """Draw the step's prompt, options, and (on the last step) the name field."""
        p = self.app.palette
        cards.draw_vertical_gradient(surface, p.bg_top, p.bg_bottom)
        cx = surface.get_width() // 2
        prompts = ["Choose your mode", "Choose your point style", "Name your practice"]
        text_ui.draw_text_center(surface, prompts[self.step], (cx, 90), 44, p.text)
        if self.step == 0:
            text_ui.draw_text_center(surface, WELCOME, (cx, 150), 22, p.text_muted)
        elif self.step == 2:
            text_ui.draw_text_center(surface, PRIVACY, (cx, 150), 22, p.text_muted)
            text_ui.draw_text_center(surface, f"Name: {self.name}", (cx, 200), 30, p.accent)
        for b in self._buttons:
            b.draw(surface, p)
