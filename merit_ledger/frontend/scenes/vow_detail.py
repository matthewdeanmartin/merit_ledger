"""Vow detail: status-aware actions + timeline (spec §5.3-§5.6)."""

from __future__ import annotations

import contextlib
from typing import Any

import pygame

from merit_ledger.frontend.pygame_app import PygameApp
from merit_ledger.frontend.scenes.nav import MainScene
from merit_ledger.frontend.ui import cards
from merit_ledger.frontend.ui import text as text_ui
from merit_ledger.frontend.ui.buttons import Button


def actions_for(vow: dict[str, Any]) -> list[str]:
    """Return the action labels valid for a vow, given its type + status (pure).

    Mirrors the backend state machine (Sprint 3) so the UI only offers legal moves.
    """
    status = vow.get("status")
    vtype = vow.get("vow_type")
    if status in ("completed", "retired"):
        return []
    actions: list[str] = []
    if vtype == "positive" and status in ("active", "repair_in_progress"):
        actions.append("Complete")
    if vtype == "negative" and status in ("active", "paused", "repair_in_progress"):
        actions.append("Record breach")
    if status == "repair_in_progress":
        actions.append("Complete repair")
    if status in ("active", "repair_in_progress"):
        actions.append("Pause")
    if status == "paused":
        actions.append("Resume")
    actions.append("Retire")
    return actions


class VowDetailScene(MainScene):
    """Show one vow's status/streak, its legal actions, and its timeline."""

    title = "Vow"

    def __init__(self, app: PygameApp, vow_id: str) -> None:
        """Init for a specific vow id."""
        super().__init__(app, active_tab="Vows")
        self.vow_id = vow_id
        self._vow: dict[str, Any] = {}
        self._timeline: list[dict[str, Any]] = []
        self._buttons: list[Button] = []

    def on_enter(self) -> None:
        """Load the vow + its timeline and build action buttons."""
        try:
            self._vow = self.app.api.get_vow(self.vow_id)
            self._timeline = self.app.api.list_entries(vow_id=self.vow_id)
        except Exception:
            self._vow = {}
        self._buttons = []
        x = 20
        for label in actions_for(self._vow):
            rect = pygame.Rect(x, self.BODY_TOP + 60, 180, 44)
            self._buttons.append(Button(label, rect, self._make_action(label), font_size=22))
            x += 190

    def _make_action(self, label: str):  # type: ignore[no-untyped-def]
        """Return a click handler for an action label."""
        def run() -> None:
            self._do(label)

        return run

    def _do(self, label: str) -> None:
        """Perform an action against the backend, then refresh (or route to repentance)."""
        if label == "Record breach":
            # Route to repentance prefilled with this vow's category + link (spec §5.4).
            from merit_ledger.frontend.scenes.repentance import RepentanceScene

            self.app.api.vow_action(self.vow_id, "breach", {})
            self.app.switch_to(
                RepentanceScene(
                    self.app,
                    linked_vow_id=self.vow_id,
                    category=self._vow.get("repentance_category"),
                )
            )
            return
        action_map = {
            "Complete": ("complete", {}),
            "Complete repair": ("complete", {}),
            "Pause": ("pause", {"reason": "paused from detail"}),
            "Resume": ("resume", {}),
            "Retire": ("retire", {}),
        }
        if label in action_map:
            action, body = action_map[label]
            with contextlib.suppress(Exception):
                self.app.api.vow_action(self.vow_id, action, body)
        self.on_enter()

    def handle_body_event(self, event: pygame.event.Event) -> None:
        """Forward events to action buttons."""
        for b in self._buttons:
            b.handle_event(event)

    def draw_body(self, surface: pygame.Surface) -> None:
        """Draw the vow header, actions, and timeline."""
        p = self.app.palette
        if not self._vow:
            text_ui.draw_text(surface, "Vow not found.", (20, self.BODY_TOP + 20), 26, p.text_muted)
            return
        name = self._vow.get("name", "")
        status = self._vow.get("status", "")
        label = "repair available" if status == "repair_in_progress" else status
        text_ui.draw_text(surface, name, (20, self.BODY_TOP + 8), 32, p.text)
        text_ui.draw_text(
            surface,
            f"{self._vow.get('vow_type', '')} · {label} · streak {self._vow.get('streak', 0)}",
            (20, self.BODY_TOP + 44),
            22,
            p.text_muted,
        )
        for b in self._buttons:
            b.draw(surface, p)

        y = self.BODY_TOP + 130
        text_ui.draw_text(surface, "Timeline", (20, y), 26, p.text)
        y += 40
        for e in list(reversed(self._timeline))[:6]:
            rect = pygame.Rect(20, y, surface.get_width() - 40, 40)
            cards.draw_card(surface, rect, p, radius=8)
            text_ui.draw_text(surface, str(e.get("title", e.get("entry_type", ""))), (36, y + 10), 22, p.text)
            text_ui.draw_text(surface, str(e.get("occurred_at", ""))[:10], (surface.get_width() - 140, y + 10), 20, p.text_muted)
            y += 48
