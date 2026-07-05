"""Pygame scene framework and main loop (spec §15.1)."""

from __future__ import annotations

import pygame

from merit_ledger.frontend.api_client import ApiClient
from merit_ledger.frontend.theme import Palette, palette_for_theme

WINDOW_SIZE = (960, 640)
WINDOW_TITLE = "Merit Ledger"
FPS = 60


class Scene:
    """Base class for a screen. Subclasses override the three lifecycle hooks."""

    def __init__(self, app: PygameApp) -> None:
        """Store a reference to the owning app for navigation + data access."""
        self.app = app

    def on_enter(self) -> None:
        """Called when the scene becomes active (refresh data here)."""

    def handle_event(self, event: pygame.event.Event) -> None:
        """Handle a single pygame event."""

    def update(self, dt: float) -> None:
        """Advance any time-based state by ``dt`` seconds."""

    def draw(self, surface: pygame.Surface) -> None:
        """Render the scene to ``surface``."""


class PygameApp:
    """Owns the window, the API client, the active tradition palette, and the scene stack."""

    def __init__(self, api: ApiClient) -> None:
        """Create the app around an API client (backend must already be healthy)."""
        self.api = api
        self.screen: pygame.Surface | None = None
        self.running = False
        self._scene: Scene | None = None
        self.palette: Palette = palette_for_theme("")
        self.refresh_palette()

    def refresh_palette(self) -> None:
        """Reload the palette from the active tradition's theme."""
        try:
            settings = self.api.get_settings()
            pack = self.api.get_tradition(settings.get("tradition", "secular"))
            self.palette = palette_for_theme(pack.get("theme", ""))
        except Exception:  # noqa: BLE001 - never let theming crash the UI
            self.palette = palette_for_theme("")

    def switch_to(self, scene: Scene) -> None:
        """Make ``scene`` the active scene and fire its ``on_enter``."""
        self._scene = scene
        scene.on_enter()

    def run(self, initial_scene_factory) -> None:  # type: ignore[no-untyped-def]
        """Initialize pygame, enter ``initial_scene_factory(app)``, and run the loop."""
        pygame.init()
        self.screen = pygame.display.set_mode(WINDOW_SIZE, pygame.RESIZABLE)
        pygame.display.set_caption(WINDOW_TITLE)
        clock = pygame.time.Clock()
        self.running = True
        self.switch_to(initial_scene_factory(self))

        while self.running:
            dt = clock.tick(FPS) / 1000.0
            for event in pygame.event.get():
                if event.type == pygame.QUIT:
                    self.running = False
                elif self._scene is not None:
                    self._scene.handle_event(event)
            if self._scene is not None:
                self._scene.update(dt)
                self._scene.draw(self.screen)
            pygame.display.flip()

        pygame.quit()
