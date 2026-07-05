"""Dedicate merit / intention (spec §5.8).

Pick a preset target (fills the fields) or type a custom one, edit the dedication text and
points, then Save. When opened from Record with a ``source_entry_id``, the dedication links
back to that entry.
"""

from __future__ import annotations

from functools import partial
from typing import Any

import pygame

from merit_ledger.frontend.pygame_app import PygameApp
from merit_ledger.frontend.scenes.nav import MainScene
from merit_ledger.frontend.ui import cards
from merit_ledger.frontend.ui import text as text_ui
from merit_ledger.frontend.ui.buttons import Button
from merit_ledger.frontend.ui.text_input import TextInput


class DedicationScene(MainScene):
    """Choose or type a target, edit text + points, and dedicate merit."""

    title = "Dedicate"

    def __init__(self, app: PygameApp, source_entry_id: str | None = None) -> None:
        """Init, optionally linking the dedication to a source entry."""
        super().__init__(app, active_tab="Dedicate")
        self.source_entry_id = source_entry_id
        self._targets: list[dict[str, Any]] = []
        self._preset_buttons: list[Button] = []
        self._target_type = "generic_group"
        self._target_field: TextInput | None = None
        self._text_field: TextInput | None = None
        self._points_field: TextInput | None = None
        self._save_button: Button | None = None
        self._message = ""

    def on_enter(self) -> None:
        """Load presets, build preset buttons (left) and editable fields (right)."""
        default_text = ""
        try:
            presets = self.app.api.dedication_presets()
            self._targets = presets.get("targets", [])
            default_text = presets.get("default_text", "")
        except Exception:
            self._targets = []

        self._preset_buttons = []
        y = self.BODY_TOP + 30
        for target in self._targets[:8]:
            rect = pygame.Rect(20, y, 360, 40)
            self._preset_buttons.append(Button(target["target_name"], rect, partial(self._pick, target), font_size=20))
            y += 46

        self._target_field = TextInput(
            pygame.Rect(420, self.BODY_TOP + 60, 460, 40),
            text="All sentient beings", placeholder="dedicate to…", max_length=60, font_size=24,
        )
        self._text_field = TextInput(
            pygame.Rect(420, self.BODY_TOP + 120, 460, 40),
            text=default_text, placeholder="dedication words", max_length=140, font_size=22,
        )
        self._points_field = TextInput(
            pygame.Rect(420, self.BODY_TOP + 180, 120, 40),
            text="0", max_length=5, font_size=24,
        )
        self._save_button = Button(
            "Dedicate", pygame.Rect(560, self.BODY_TOP + 180, 160, 44), self._save, font_size=24
        )

    def _pick(self, target: dict[str, Any]) -> None:
        """Fill the target fields from a preset."""
        self._target_type = target.get("target_type", "generic_group")
        if self._target_field is not None:
            self._target_field.text = target["target_name"]

    def _save(self) -> None:
        """Create the dedication from the current field values."""
        if self._target_field is None or self._text_field is None or self._points_field is None:
            return
        body: dict[str, Any] = {
            "target_type": self._target_type,
            "target_name": self._target_field.text.strip() or "All sentient beings",
            "dedication_text": self._text_field.text.strip(),
            "points_dedicated": int(self._points_field.text) if self._points_field.text.isdigit() else 0,
        }
        if self.source_entry_id:
            body["source_entry_id"] = self.source_entry_id
        try:
            self.app.api.create_dedication(body)
            self._message = f"Dedicated to {body['target_name']}."
        except Exception:
            self._message = "Could not save."

    def handle_body_event(self, event: pygame.event.Event) -> None:
        """Forward events to preset buttons, fields, and the save button."""
        for b in self._preset_buttons:
            b.handle_event(event)
        for field in (self._target_field, self._text_field, self._points_field):
            if field is not None:
                field.handle_event(event)
        if self._save_button is not None:
            self._save_button.handle_event(event)

    def update(self, dt: float) -> None:
        """Blink the field carets."""
        for field in (self._target_field, self._text_field, self._points_field):
            if field is not None:
                field.update(dt)

    def draw_body(self, surface: pygame.Surface) -> None:
        """Draw the preset picker, the editable fields, and confirmation."""
        p = self.app.palette
        text_ui.draw_text(surface, "Presets", (20, self.BODY_TOP), 22, p.text_muted)
        for b in self._preset_buttons:
            b.draw(surface, p)

        text_ui.draw_text(surface, "Target", (420, self.BODY_TOP + 36), 18, p.text_muted)
        text_ui.draw_text(surface, "Words", (420, self.BODY_TOP + 96), 18, p.text_muted)
        text_ui.draw_text(surface, "Points", (420, self.BODY_TOP + 156), 18, p.text_muted)
        for field in (self._target_field, self._text_field, self._points_field):
            if field is not None:
                field.draw(surface, p)
        if self._save_button is not None:
            self._save_button.draw(surface, p)
        if self.source_entry_id:
            text_ui.draw_text(surface, "linked to your last practice", (420, self.BODY_TOP + 230), 18, p.text_muted)

        if self._message:
            rect = pygame.Rect(20, surface.get_height() - 66, surface.get_width() - 40, 44)
            cards.draw_card(surface, rect, p, radius=10)
            text_ui.draw_text(surface, self._message, (36, surface.get_height() - 56), 24, p.accent)
