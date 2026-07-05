"""Frontend scene state/routing tests (spec §23.4). No rendering assertions.

Scenes are constructed with an ApiClient wired to an in-memory backend, and their handler
methods are called directly. We stub the app's screen with a dummy headless surface.
"""

from __future__ import annotations

import os

import pytest
from fastapi.testclient import TestClient

os.environ.setdefault("SDL_VIDEODRIVER", "dummy")
os.environ.setdefault("SDL_AUDIODRIVER", "dummy")

import pygame  # noqa: E402

from merit_ledger.backend.main import create_app  # noqa: E402
from merit_ledger.backend.repository.memory_repo import InMemoryMeritRepository  # noqa: E402
from merit_ledger.frontend.api_client import ApiClient  # noqa: E402
from merit_ledger.frontend.pygame_app import PygameApp  # noqa: E402
from merit_ledger.frontend.scenes.vow_detail import actions_for  # noqa: E402


@pytest.fixture
def app() -> PygameApp:
    """A PygameApp over an in-memory backend, with a headless display initialized."""
    pygame.init()
    pygame.display.set_mode((960, 640))
    client = ApiClient(client=TestClient(create_app(repo=InMemoryMeritRepository())))
    a = PygameApp(client)
    a.screen = pygame.display.get_surface()
    a.api.set_tradition("secular")
    return a


# --- vow_detail action table (pure) -----------------------------------------


def test_actions_positive_active() -> None:
    acts = actions_for({"vow_type": "positive", "status": "active"})
    assert "Complete" in acts and "Pause" in acts and "Retire" in acts
    assert "Record breach" not in acts


def test_actions_negative_active_offers_breach() -> None:
    acts = actions_for({"vow_type": "negative", "status": "active"})
    assert "Record breach" in acts


def test_actions_repair_offers_complete_repair() -> None:
    acts = actions_for({"vow_type": "negative", "status": "repair_in_progress"})
    assert "Complete repair" in acts


def test_actions_retired_has_none() -> None:
    assert actions_for({"vow_type": "positive", "status": "retired"}) == []


# --- scene behavior ----------------------------------------------------------


def test_record_scene_creates_entry(app: PygameApp) -> None:
    from merit_ledger.frontend.scenes.record_practice import RecordPracticeScene

    scene = RecordPracticeScene(app)
    scene.on_enter()
    assert scene._templates  # secular templates loaded
    scene._record(scene._templates[0])
    assert "Recorded" in scene._message
    assert app.api.list_entries()  # persisted


def test_vows_scene_create_and_open(app: PygameApp) -> None:
    from merit_ledger.frontend.scenes.vows import VowsScene

    scene = VowsScene(app)
    scene.on_enter()
    scene._create("positive")
    assert len(app.api.list_vows()) == 1


def test_breach_routes_to_repentance_with_prefill(app: PygameApp) -> None:
    from merit_ledger.frontend.scenes.repentance import RepentanceScene
    from merit_ledger.frontend.scenes.vow_detail import VowDetailScene

    vow = app.api.create_vow({"name": "No harsh speech", "vow_type": "negative",
                              "repentance_category": "speech"})
    detail = VowDetailScene(app, vow["vow_id"])
    detail.on_enter()
    detail._do("Record breach")
    # app switched to a repentance scene carrying the link + category
    scene = app._scene
    assert isinstance(scene, RepentanceScene)
    assert scene.linked_vow_id == vow["vow_id"]
    assert scene.category == "speech"


def test_mudita_rejoice_creates_entry(app: PygameApp) -> None:
    from merit_ledger.frontend.scenes.mudita_garden import MuditaGardenScene

    scene = MuditaGardenScene(app)
    scene.on_enter()
    first = scene._feed[0]
    scene._rejoice(first)
    assert first["sample_id"] in scene._flowers
    assert any(e["entry_type"] == "mudita_rejoiced" for e in app.api.list_entries())


def test_settings_tradition_switch_reskins(app: PygameApp) -> None:
    from merit_ledger.frontend.scenes.settings import SettingsScene

    scene = SettingsScene(app)
    scene.on_enter()
    scene._set_tradition("pure_land")
    assert app.api.get_settings()["tradition"] == "pure_land"


def test_settings_clear_user_data_keeps_settings(app: PygameApp) -> None:
    from merit_ledger.frontend.scenes.settings import SettingsScene

    app.api.set_tradition("zen")
    app.api.create_entry({"template_id": "zen.zazen", "title": "Zazen"})
    scene = SettingsScene(app)
    scene.on_enter()
    scene._ask_clear("user_data")
    scene._do_clear()
    assert app.api.list_entries() == []
    assert app.api.get_settings()["tradition"] == "zen"  # kept
