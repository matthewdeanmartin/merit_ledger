"""Practice template endpoints (spec §14.3)."""

from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException

from merit_ledger.backend.api.deps import get_repo
from merit_ledger.backend.domain.models import PracticeTemplate
from merit_ledger.backend.repository.base import MeritRepository
from merit_ledger.backend.services import profile_service, template_service

router = APIRouter()


def _active_tradition(repo: MeritRepository) -> str:
    """Return the user's currently selected tradition id."""
    return profile_service.get_settings(repo).tradition


@router.get("/templates")
def list_templates(repo: MeritRepository = Depends(get_repo)) -> list[PracticeTemplate]:
    """List pack-suggested + custom templates for the active tradition."""
    return template_service.list_templates(repo, _active_tradition(repo))


@router.get("/templates/{template_id}")
def get_template(template_id: str, repo: MeritRepository = Depends(get_repo)) -> PracticeTemplate:
    """Return a single template by id."""
    tmpl = template_service.get_template(repo, template_id, _active_tradition(repo))
    if tmpl is None:
        raise HTTPException(status_code=404, detail="Unknown template")
    return tmpl


@router.post("/templates")
def create_template(
    template: PracticeTemplate, repo: MeritRepository = Depends(get_repo)
) -> PracticeTemplate:
    """Create a custom template."""
    return template_service.save_template(repo, template)


@router.put("/templates/{template_id}")
def update_template(
    template_id: str, template: PracticeTemplate, repo: MeritRepository = Depends(get_repo)
) -> PracticeTemplate:
    """Update a custom template (path id wins)."""
    template.template_id = template_id
    return template_service.save_template(repo, template)


@router.delete("/templates/{template_id}")
def delete_template(template_id: str, repo: MeritRepository = Depends(get_repo)) -> dict[str, str]:
    """Delete a custom template."""
    template_service.delete_template(repo, template_id)
    return {"status": "deleted"}
