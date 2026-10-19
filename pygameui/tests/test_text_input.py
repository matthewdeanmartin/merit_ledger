"""TextInput widget key-handling tests (headless)."""

from __future__ import annotations

import os

os.environ.setdefault("SDL_VIDEODRIVER", "dummy")

import pygame  # noqa: E402

from merit_ledger.frontend.ui.text_input import TextInput  # noqa: E402


def _key(unicode: str = "", key: int = 0) -> pygame.event.Event:
    return pygame.event.Event(pygame.KEYDOWN, {"key": key, "unicode": unicode})


def _click(pos: tuple[int, int]) -> pygame.event.Event:
    return pygame.event.Event(pygame.MOUSEBUTTONDOWN, {"button": 1, "pos": pos})


def setup_module() -> None:
    pygame.init()


def test_focus_on_click_inside_and_blur_outside() -> None:
    field = TextInput(pygame.Rect(10, 10, 100, 40))
    field.handle_event(_click((20, 20)))
    assert field.focused
    field.handle_event(_click((500, 500)))
    assert not field.focused


def test_typing_appends_only_when_focused() -> None:
    field = TextInput(pygame.Rect(10, 10, 100, 40))
    field.handle_event(_key("a"))  # not focused yet
    assert field.text == ""
    field.handle_event(_click((20, 20)))
    field.handle_event(_key("h"))
    field.handle_event(_key("i"))
    assert field.text == "hi"


def test_backspace_and_max_length() -> None:
    field = TextInput(pygame.Rect(10, 10, 100, 40), text="ab", max_length=3)
    field.handle_event(_click((20, 20)))
    field.handle_event(_key("", pygame.K_BACKSPACE))
    assert field.text == "a"
    field.handle_event(_key("x"))
    field.handle_event(_key("y"))
    field.handle_event(_key("z"))  # exceeds max_length=3
    assert field.text == "axy"


def test_enter_blurs() -> None:
    field = TextInput(pygame.Rect(10, 10, 100, 40))
    field.handle_event(_click((20, 20)))
    assert field.focused
    field.handle_event(_key("", pygame.K_RETURN))
    assert not field.focused
