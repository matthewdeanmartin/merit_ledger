"""Repository interface and item shape (spec §13).

The domain never learns whether storage is SQLite, DynamoDB, or in-memory. Every
repository speaks in terms of :class:`MeritItem`, whose fields map 1:1 to the SQLite
single-table columns and to a future DynamoDB table.
"""

from __future__ import annotations

import abc
from typing import Any

from pydantic import BaseModel, Field


class MeritItem(BaseModel):
    """A single row in the single-table design.

    Fields mirror the SQLite columns exactly (spec §13.2). ``item`` is the decoded
    JSON payload; it is serialized to the ``item_json`` column by storage backends.
    """

    pk: str
    sk: str
    entity_type: str
    gsi1pk: str | None = None
    gsi1sk: str | None = None
    gsi2pk: str | None = None
    gsi2sk: str | None = None
    item: dict[str, Any] = Field(default_factory=dict)
    created_at: str
    updated_at: str


class MeritRepository(abc.ABC):
    """Abstract single-table repository mirroring DynamoDB access patterns."""

    @abc.abstractmethod
    def put_item(self, item: MeritItem) -> None:
        """Insert or replace an item keyed by (pk, sk)."""

    @abc.abstractmethod
    def get_item(self, pk: str, sk: str) -> MeritItem | None:
        """Return the item at (pk, sk) or ``None`` if absent."""

    @abc.abstractmethod
    def query_pk(self, pk: str, sk_begins_with: str | None = None) -> list[MeritItem]:
        """Return items in a partition, optionally filtered by sk prefix, sorted by sk."""

    @abc.abstractmethod
    def query_gsi1(self, gsi1pk: str, gsi1sk_begins_with: str | None = None) -> list[MeritItem]:
        """Query GSI1 by partition, optionally filtered by sort-key prefix."""

    @abc.abstractmethod
    def query_gsi2(self, gsi2pk: str, gsi2sk_begins_with: str | None = None) -> list[MeritItem]:
        """Query GSI2 by partition, optionally filtered by sort-key prefix."""

    @abc.abstractmethod
    def delete_item(self, pk: str, sk: str) -> None:
        """Delete the item at (pk, sk); no error if absent."""

    @abc.abstractmethod
    def scan_all(self) -> list[MeritItem]:
        """Return every item (used for export). Not a Dynamo access pattern, but cheap locally."""
