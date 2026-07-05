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
    scene._select(scene._templates[0])
    assert scene._quantity is not None and scene._reflection is not None
    scene._reflection.text = "returned to the breath"
    scene._record()
    assert "Recorded" in scene._message
    entries = app.api.list_entries()
    assert entries and entries[0]["reflection"] == "returned to the breath"


def test_record_requires_selection(app: PygameApp) -> None:
    from merit_ledger.frontend.scenes.record_practice import RecordPracticeScene

    scene = RecordPracticeScene(app)
    scene.on_enter()
    scene._record()  # nothing selected
    assert "Pick a practice" in scene._message
    assert app.api.list_entries() == []


def test_record_dedicate_after_routes(app: PygameApp) -> None:
    from merit_ledger.frontend.scenes.dedication import DedicationScene
    from merit_ledger.frontend.scenes.record_practice import RecordPracticeScene

    scene = RecordPracticeScene(app)
    scene.on_enter()
    scene._select(scene._templates[0])
    scene._toggle_dedicate()
    assert scene._dedicate_after is True
    scene._record()
    assert isinstance(app._scene, DedicationScene)
    assert app._scene.source_entry_id is not None


def test_vows_scene_create_and_open(app: PygameApp) -> None:
    from merit_ledger.frontend.scenes.vows import VowsScene

    scene = VowsScene(app)
    scene.on_enter()
    scene._create("positive")
    assert len(app.api.list_vows()) == 1


def test_vow_detail_edits_name_and_points(app: PygameApp) -> None:
    from merit_ledger.frontend.scenes.vow_detail import VowDetailScene

    vow = app.api.create_vow({"name": "New commitment", "vow_type": "positive", "default_points": 10})
    detail = VowDetailScene(app, vow["vow_id"])
    detail.on_enter()
    # user renames it and changes the points, then saves
    assert detail._name_field is not None and detail._points_field is not None
    detail._name_field.text = "Sit zazen 20 min"
    detail._points_field.text = "7"
    detail._save_edits()
    updated = app.api.get_vow(vow["vow_id"])
    assert updated["name"] == "Sit zazen 20 min"
    assert updated["default_points"] == 7


def test_vows_create_opens_detail_for_naming(app: PygameApp) -> None:
    from merit_ledger.frontend.scenes.vow_detail import VowDetailScene
    from merit_ledger.frontend.scenes.vows import VowsScene

    scene = VowsScene(app)
    scene.on_enter()
    scene._create("positive")
    # creating a vow drops you straight into its detail scene to name it
    assert isinstance(app._scene, VowDetailScene)


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


def test_repentance_saves_with_reflection_and_intention(app: PygameApp) -> None:
    from merit_ledger.frontend.scenes.repentance import RepentanceScene

    scene = RepentanceScene(app)
    scene.on_enter()
    scene._pick("anger")
    assert scene._reflection is not None and scene._intention is not None
    scene._reflection.text = "noticed heat, breathed"
    scene._intention.text = "pause before replying"
    scene._save()
    entries = [e for e in app.api.list_entries() if e["entry_type"] == "repentance_completed"]
    assert entries and entries[0]["category"] == "anger"
    assert entries[0]["reflection"] == "noticed heat, breathed"
    assert entries[0]["repair_intention"] == "pause before replying"


def test_repentance_requires_category(app: PygameApp) -> None:
    from merit_ledger.frontend.scenes.repentance import RepentanceScene

    scene = RepentanceScene(app)
    scene.on_enter()
    scene._save()  # no category picked
    assert "category" in scene._message.lower()


def test_dedication_custom_target_and_text(app: PygameApp) -> None:
    from merit_ledger.frontend.scenes.dedication import DedicationScene

    scene = DedicationScene(app)
    scene.on_enter()
    assert scene._target_field is not None and scene._text_field is not None
    scene._target_field.text = "My grandmother"
    scene._text_field.text = "May she be at peace."
    scene._points_field.text = "5"
    scene._save()
    dedications = app.api.list_dedications()
    assert dedications
    assert dedications[0]["target_name"] == "My grandmother"
    assert dedications[0]["points_dedicated"] == 5


def test_dedication_links_to_source_entry(app: PygameApp) -> None:
    from merit_ledger.frontend.scenes.dedication import DedicationScene

    entry = app.api.create_entry({"template_id": "secular.help", "title": "Helped"})
    scene = DedicationScene(app, source_entry_id=entry["entry_id"])
    scene.on_enter()
    scene._save()
    updated = app.api.list_entries()
    linked = [e for e in updated if e["entry_id"] == entry["entry_id"]][0]
    assert linked["dedication_id"] is not None


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
