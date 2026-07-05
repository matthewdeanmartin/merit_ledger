"""Splash routing decision (pure, no pygame display needed)."""

from __future__ import annotations

from merit_ledger.frontend.scenes.splash import next_scene_name


def test_routes_to_onboarding_when_not_onboarded() -> None:
    assert next_scene_name({"onboarded": False}) == "onboarding"
    assert next_scene_name({}) == "onboarding"


def test_routes_to_dashboard_when_onboarded() -> None:
    assert next_scene_name({"onboarded": True}) == "dashboard"
