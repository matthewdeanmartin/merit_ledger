"""Practice template CRUD + merged listing (pack-suggested + custom)."""

from __future__ import annotations

from merit_ledger.backend.domain.models import PracticeTemplate
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.repository.item_keys import (
    TEMPLATE_SK_PREFIX,
    template_from_item,
    to_template_item,
    user_pk,
)
from merit_ledger.backend.services import tradition_service
from merit_ledger.local.config import DEFAULT_USER_ID


def list_custom_templates(repo: MeritRepository, user_id: str = DEFAULT_USER_ID) -> list[PracticeTemplate]:
    """Return the user's custom templates."""
    items = repo.query_pk(user_pk(user_id), TEMPLATE_SK_PREFIX)
    return [template_from_item(i) for i in items]


def list_templates(
    repo: MeritRepository, tradition_id: str, user_id: str = DEFAULT_USER_ID
) -> list[PracticeTemplate]:
    """Return pack-suggested templates for ``tradition_id`` merged with custom ones.

    Custom templates override pack templates that share a ``template_id``.
    """
    by_id: dict[str, PracticeTemplate] = {}
    for tmpl in tradition_service.suggested_templates(tradition_id):
        by_id[tmpl.template_id] = tmpl
    for tmpl in list_custom_templates(repo, user_id):
        by_id[tmpl.template_id] = tmpl
    return list(by_id.values())


def get_template(
    repo: MeritRepository, template_id: str, tradition_id: str, user_id: str = DEFAULT_USER_ID
) -> PracticeTemplate | None:
    """Return a template by id: custom first, then pack-suggested."""
    item = repo.get_item(user_pk(user_id), f"{TEMPLATE_SK_PREFIX}{template_id}")
    if item:
        return template_from_item(item)
    return tradition_service.find_template(tradition_id, template_id)


def save_template(
    repo: MeritRepository, template: PracticeTemplate, user_id: str = DEFAULT_USER_ID
) -> PracticeTemplate:
    """Persist a custom template (marks ``is_custom``)."""
    template.is_custom = True
    repo.put_item(to_template_item(template, user_id))
    return template


def delete_template(repo: MeritRepository, template_id: str, user_id: str = DEFAULT_USER_ID) -> None:
    """Delete a custom template by id."""
    repo.delete_item(user_pk(user_id), f"{TEMPLATE_SK_PREFIX}{template_id}")
