"""SQLite single-table repository (spec §13.2).

Models a DynamoDB single-table design without requiring DynamoDB Local: one table,
composite (pk, sk) primary key, GSI-style columns, and a JSON item payload.
"""

from __future__ import annotations

import json
import sqlite3
import threading
from pathlib import Path

from merit_ledger.backend.repository.base import MeritItem, MeritRepository

_CREATE_TABLE = """
CREATE TABLE IF NOT EXISTS merit_items (
    pk TEXT NOT NULL,
    sk TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    gsi1pk TEXT,
    gsi1sk TEXT,
    gsi2pk TEXT,
    gsi2sk TEXT,
    item_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (pk, sk)
);
"""

_CREATE_GSI1 = "CREATE INDEX IF NOT EXISTS idx_gsi1 ON merit_items (gsi1pk, gsi1sk);"
_CREATE_GSI2 = "CREATE INDEX IF NOT EXISTS idx_gsi2 ON merit_items (gsi2pk, gsi2sk);"

_COLUMNS = "pk, sk, entity_type, gsi1pk, gsi1sk, gsi2pk, gsi2sk, item_json, created_at, updated_at"


class SqliteMeritRepository(MeritRepository):
    """Repository backed by a SQLite database file (or ``:memory:``)."""

    def __init__(self, db_path: str | Path = ":memory:") -> None:
        """Open (or create) the database and ensure the schema exists.

        Args:
            db_path: Filesystem path, or ``":memory:"`` for an ephemeral db.
        """
        # FastAPI runs sync endpoints in a threadpool, so the connection is used from
        # multiple threads. Allow cross-thread use and serialize access with a lock.
        self._conn = sqlite3.connect(str(db_path), check_same_thread=False)
        self._conn.row_factory = sqlite3.Row
        self._lock = threading.Lock()
        with self._lock:
            self._conn.execute(_CREATE_TABLE)
            self._conn.execute(_CREATE_GSI1)
            self._conn.execute(_CREATE_GSI2)
            self._conn.commit()

    def close(self) -> None:
        """Close the underlying connection."""
        with self._lock:
            self._conn.close()

    @staticmethod
    def _row_to_item(row: sqlite3.Row) -> MeritItem:
        """Convert a DB row to a :class:`MeritItem`."""
        return MeritItem(
            pk=row["pk"],
            sk=row["sk"],
            entity_type=row["entity_type"],
            gsi1pk=row["gsi1pk"],
            gsi1sk=row["gsi1sk"],
            gsi2pk=row["gsi2pk"],
            gsi2sk=row["gsi2sk"],
            item=json.loads(row["item_json"]),
            created_at=row["created_at"],
            updated_at=row["updated_at"],
        )

    def put_item(self, item: MeritItem) -> None:
        """Upsert ``item`` keyed by (pk, sk)."""
        with self._lock:
            self._conn.execute(
                f"INSERT OR REPLACE INTO merit_items ({_COLUMNS}) VALUES (?,?,?,?,?,?,?,?,?,?)",
                (
                    item.pk,
                    item.sk,
                    item.entity_type,
                    item.gsi1pk,
                    item.gsi1sk,
                    item.gsi2pk,
                    item.gsi2sk,
                    json.dumps(item.item),
                    item.created_at,
                    item.updated_at,
                ),
            )
            self._conn.commit()

    def get_item(self, pk: str, sk: str) -> MeritItem | None:
        """Return the item at (pk, sk) or None."""
        with self._lock:
            cur = self._conn.execute(
                f"SELECT {_COLUMNS} FROM merit_items WHERE pk = ? AND sk = ?",
                (pk, sk),
            )
            row = cur.fetchone()
        return self._row_to_item(row) if row else None

    def query_pk(self, pk: str, sk_begins_with: str | None = None) -> list[MeritItem]:
        """Return items in partition ``pk`` filtered by sk prefix, sorted by sk."""
        with self._lock:
            if sk_begins_with is None:
                cur = self._conn.execute(
                    f"SELECT {_COLUMNS} FROM merit_items WHERE pk = ? ORDER BY sk",
                    (pk,),
                )
            else:
                cur = self._conn.execute(
                    f"SELECT {_COLUMNS} FROM merit_items WHERE pk = ? AND sk LIKE ? ORDER BY sk",
                    (pk, f"{sk_begins_with}%"),
                )
            rows = cur.fetchall()
        return [self._row_to_item(r) for r in rows]

    def _query_gsi(
        self, pk_col: str, sk_col: str, pk_value: str, sk_prefix: str | None
    ) -> list[MeritItem]:
        """Shared GSI query for both indexes."""
        with self._lock:
            if sk_prefix is None:
                cur = self._conn.execute(
                    f"SELECT {_COLUMNS} FROM merit_items WHERE {pk_col} = ? ORDER BY {sk_col}",
                    (pk_value,),
                )
            else:
                cur = self._conn.execute(
                    f"SELECT {_COLUMNS} FROM merit_items "
                    f"WHERE {pk_col} = ? AND {sk_col} LIKE ? ORDER BY {sk_col}",
                    (pk_value, f"{sk_prefix}%"),
                )
            rows = cur.fetchall()
        return [self._row_to_item(r) for r in rows]

    def query_gsi1(self, gsi1pk: str, gsi1sk_begins_with: str | None = None) -> list[MeritItem]:
        """Query GSI1 by partition and optional sort-key prefix."""
        return self._query_gsi("gsi1pk", "gsi1sk", gsi1pk, gsi1sk_begins_with)

    def query_gsi2(self, gsi2pk: str, gsi2sk_begins_with: str | None = None) -> list[MeritItem]:
        """Query GSI2 by partition and optional sort-key prefix."""
        return self._query_gsi("gsi2pk", "gsi2sk", gsi2pk, gsi2sk_begins_with)

    def delete_item(self, pk: str, sk: str) -> None:
        """Delete the item at (pk, sk) if present."""
        with self._lock:
            self._conn.execute("DELETE FROM merit_items WHERE pk = ? AND sk = ?", (pk, sk))
            self._conn.commit()

    def scan_all(self) -> list[MeritItem]:
        """Return every item, sorted by (pk, sk)."""
        with self._lock:
            cur = self._conn.execute(f"SELECT {_COLUMNS} FROM merit_items ORDER BY pk, sk")
            rows = cur.fetchall()
        return [self._row_to_item(r) for r in rows]

    def clear(self) -> int:
        """Delete every row and return the number removed."""
        with self._lock:
            cur = self._conn.execute("DELETE FROM merit_items")
            self._conn.commit()
            return int(cur.rowcount)
