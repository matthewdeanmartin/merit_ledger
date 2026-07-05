"""Record a positive practice (spec §5.2).

Left column: pick a practice template. Right column: optional quantity + reflection, then
Record. A "Dedicate after" toggle routes to the dedication scene for the new entry.
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


class RecordPracticeScene(MainScene):
    """Pick a practice template, optionally set quantity/reflection, and record it."""

    title = "Record"

    def __init__(self, app: PygameApp) -> None:
        """Init with no templates loaded and nothing selected."""
        super().__init__(app, active_tab="Record")
        self._templates: list[dict[str, Any]] = []
        self._selected: dict[str, Any] | None = None
        self._template_buttons: list[Button] = []
        self._quantity: TextInput | None = None
        self._reflection: TextInput | None = None
        self._action_buttons: list[Button] = []
        self._dedicate_after = False
        self._message = ""

    def on_enter(self) -> None:
        """Load templates and build the picker + the right-hand form."""
        try:
            self._templates = self.app.api.list_templates()
        except Exception:
            self._templates = []

        self._template_buttons = []
        y = self.BODY_TOP + 20
        for tmpl in self._templates[:8]:
            rect = pygame.Rect(20, y, 440, 42)
            self._template_buttons.append(
                Button(
                    f"{tmpl['name']}  (+{tmpl.get('default_points', 0)})",
                    rect,
                    partial(self._select, tmpl),
                    font_size=22,
                )
            )
            y += 48

        self._build_form()

    def _build_form(self) -> None:
        """(Re)build the right-hand quantity/reflection fields + action buttons."""
        default_qty = self._selected.get("default_quantity") if self._selected else None
        self._quantity = TextInput(
            pygame.Rect(500, self.BODY_TOP + 60, 120, 40),
            text=str(default_qty) if default_qty else "",
            placeholder="qty",
            max_length=6,
            font_size=24,
        )
        self._reflection = TextInput(
            pygame.Rect(500, self.BODY_TOP + 120, 420, 40),
            placeholder="optional reflection (keep it general)",
            max_length=120,
            font_size=22,
        )
        self._action_buttons = [
            Button("Dedicate after: off", pygame.Rect(500, self.BODY_TOP + 180, 260, 40),
                   self._toggle_dedicate, font_size=20),
            Button("Record", pygame.Rect(500, self.BODY_TOP + 232, 160, 46),
                   self._record, font_size=26),
        ]

    def _select(self, tmpl: dict[str, Any]) -> None:
        """Select a template and refresh the form defaults."""
        self._selected = tmpl
        self._message = ""
        self._build_form()

    def _toggle_dedicate(self) -> None:
        """Flip the 'dedicate after recording' toggle."""
        self._dedicate_after = not self._dedicate_after
        state = "on" if self._dedicate_after else "off"
        self._action_buttons[0].label = f"Dedicate after: {state}"

    def _record(self) -> None:
        """Create the entry from the selected template + entered fields."""
        if self._selected is None:
            self._message = "Pick a practice on the left first."
            return
        qty_text = self._quantity.text if self._quantity else ""
        reflection = self._reflection.text.strip() if self._reflection else ""
        entry: dict[str, Any] = {
            "entry_type": "practice_completed",
            "template_id": self._selected["template_id"],
            "title": self._selected["name"],
            "quantity": int(qty_text) if qty_text.isdigit() else None,
            "quantity_unit": self._selected.get("quantity_unit"),
            "reflection": reflection or None,
        }
        try:
            saved = self.app.api.create_entry(entry)
        except Exception:
            self._message = "Could not record — is the backend running?"
            return

        if self._dedicate_after:
            from merit_ledger.frontend.scenes.dedication import DedicationScene

            self.app.switch_to(DedicationScene(self.app, source_entry_id=saved.get("entry_id")))
            return
        self._message = f"Recorded: {self._selected['name']} (+{saved.get('points', 0)})"

    def handle_body_event(self, event: pygame.event.Event) -> None:
        """Forward events to template buttons, form fields, and action buttons."""
        for b in self._template_buttons:
            b.handle_event(event)
        if self._quantity is not None:
            self._quantity.handle_event(event)
        if self._reflection is not None:
            self._reflection.handle_event(event)
        for b in self._action_buttons:
            b.handle_event(event)

    def update(self, dt: float) -> None:
        """Blink the field carets."""
        if self._quantity is not None:
            self._quantity.update(dt)
        if self._reflection is not None:
            self._reflection.update(dt)

    def draw_body(self, surface: pygame.Surface) -> None:
        """Draw the template picker, the form, and any confirmation message."""
        p = self.app.palette
        if not self._templates:
            text_ui.draw_text(surface, "No practices available.", (20, self.BODY_TOP + 20), 24, p.text_muted)
        for b in self._template_buttons:
            b.draw(surface, p)

        # Right-hand form.
        if self._selected is not None:
            text_ui.draw_text(surface, self._selected["name"], (500, self.BODY_TOP + 20), 24, p.text)
            unit = self._selected.get("quantity_unit") or "quantity"
            text_ui.draw_text(surface, unit, (500, self.BODY_TOP + 48), 18, p.text_muted)
        else:
            text_ui.draw_text(surface, "Select a practice →", (500, self.BODY_TOP + 20), 22, p.text_muted)

        if self._quantity is not None:
            self._quantity.draw(surface, p)
        if self._reflection is not None:
            self._reflection.draw(surface, p)
        for b in self._action_buttons:
            b.draw(surface, p)

        if self._message:
            rect = pygame.Rect(20, surface.get_height() - 66, surface.get_width() - 40, 44)
            cards.draw_card(surface, rect, p, radius=10)
            text_ui.draw_text(surface, self._message, (36, surface.get_height() - 56), 24, p.accent)
