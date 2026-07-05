"""In-memory repository for tests and demos."""

from __future__ import annotations

from merit_ledger.backend.repository.base import MeritItem, MeritRepository


def _prefix_ok(value: str | None, prefix: str | None) -> bool:
    """Return True if ``value`` satisfies the optional ``prefix`` filter."""
    if prefix is None:
        return True
    return value is not None and value.startswith(prefix)


class InMemoryMeritRepository(MeritRepository):
    """Dict-backed repository. Not persistent; useful for unit tests."""

    def __init__(self) -> None:
        """Create an empty store keyed by (pk, sk)."""
        self._store: dict[tuple[str, str], MeritItem] = {}

    def put_item(self, item: MeritItem) -> None:
        """Insert or replace ``item`` at its (pk, sk)."""
        self._store[(item.pk, item.sk)] = item.model_copy(deep=True)

    def get_item(self, pk: str, sk: str) -> MeritItem | None:
        """Return a deep copy of the item at (pk, sk) or None."""
        found = self._store.get((pk, sk))
        return found.model_copy(deep=True) if found else None

    def query_pk(self, pk: str, sk_begins_with: str | None = None) -> list[MeritItem]:
        """Return items in partition ``pk`` filtered by sk prefix, sorted by sk."""
        results = [
            i.model_copy(deep=True)
            for i in self._store.values()
            if i.pk == pk and _prefix_ok(i.sk, sk_begins_with)
        ]
        return sorted(results, key=lambda i: i.sk)

    def query_gsi1(self, gsi1pk: str, gsi1sk_begins_with: str | None = None) -> list[MeritItem]:
        """Query GSI1 by partition and optional sort-key prefix, sorted by gsi1sk."""
        results = [
            i.model_copy(deep=True)
            for i in self._store.values()
            if i.gsi1pk == gsi1pk and _prefix_ok(i.gsi1sk, gsi1sk_begins_with)
        ]
        return sorted(results, key=lambda i: i.gsi1sk or "")

    def query_gsi2(self, gsi2pk: str, gsi2sk_begins_with: str | None = None) -> list[MeritItem]:
        """Query GSI2 by partition and optional sort-key prefix, sorted by gsi2sk."""
        results = [
            i.model_copy(deep=True)
            for i in self._store.values()
            if i.gsi2pk == gsi2pk and _prefix_ok(i.gsi2sk, gsi2sk_begins_with)
        ]
        return sorted(results, key=lambda i: i.gsi2sk or "")

    def delete_item(self, pk: str, sk: str) -> None:
        """Delete the item at (pk, sk) if present."""
        self._store.pop((pk, sk), None)

    def scan_all(self) -> list[MeritItem]:
        """Return deep copies of all items, sorted by (pk, sk)."""
        return [
            self._store[key].model_copy(deep=True)
            for key in sorted(self._store.keys())
        ]

    def clear(self) -> int:
        """Delete every item and return the number removed."""
        removed = len(self._store)
        self._store.clear()
        return removed
