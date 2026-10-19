It is probably going to be a **big ugly conversion if you let the ORM shape the app**.

The least-ugly path is:

```text
FastAPI routes
  -> service layer
    -> repository interface
      -> SQLite/Turso implementation now
      -> DynamoDB implementation later
```

Do **not** try to find one magical ORM that makes SQLite/Turso and DynamoDB interchangeable. SQL databases and DynamoDB want different data models.

For Python, the closest “real” DynamoDB ORM-ish thing is **PynamoDB**. It is a Pythonic interface for DynamoDB and supports things like query/scan filters, DynamoDB Local, transactions, serialization/deserialization, and the DynamoDB API surface. ([PynamoDB][1]) But PynamoDB is not SQLAlchemy-for-everything. It helps once you are already in DynamoDB land.

So I would **not** do this:

```text
SQLAlchemy models now
hope to swap dialect later
DynamoDB someday
```

That is how you get sadness.

I would do this:

```python
from typing import Protocol
from datetime import datetime
from pydantic import BaseModel


class MeritEvent(BaseModel):
    id: str
    user_id: str
    tradition: str
    action_key: str
    merit_points: int
    note: str | None = None
    created_at: datetime


class MeritEventRepository(Protocol):
    async def put(self, event: MeritEvent) -> None: ...

    async def get(self, user_id: str, event_id: str) -> MeritEvent | None: ...

    async def list_for_user(
        self,
        user_id: str,
        *,
        limit: int = 100,
        after: str | None = None,
    ) -> list[MeritEvent]: ...
```

Then local version:

```python
class SqliteMeritEventRepository:
    def __init__(self, db):
        self.db = db

    async def put(self, event: MeritEvent) -> None:
        await self.db.execute(
            """
            INSERT INTO merit_events (
                id, user_id, tradition, action_key, merit_points, note, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            """,
            (
                event.id,
                event.user_id,
                event.tradition,
                event.action_key,
                event.merit_points,
                event.note,
                event.created_at.isoformat(),
            ),
        )
        await self.db.commit()
```

Cloud version later:

```python
class DynamoMeritEventRepository:
    async def put(self, event: MeritEvent) -> None:
        # boto3 or PynamoDB later
        ...
```

The trick is to make your **domain model** boring Pydantic/dataclasses, and make persistence dumb. The repository methods should be based on **access patterns**, not table-shaped CRUD.

For your app, that means thinking like this from day one:

```text
Need:
- add merit event
- list recent merit events for user
- list events by vow
- list events by tradition/category
- get current active vows
- pause/resume vow
- repentance log for event/category
- merit transfer history
```

Not this:

```text
I have tables:
- users
- vows
- merit_events
- repentance_events
- transfers
therefore later DynamoDB can just mirror them
```

DynamoDB wants you to design around access patterns. AWS’s own DynamoDB material talks about combining multiple SQL-style tables into one table using patterns like adjacency lists and item-type prefixes. ([Amazon Web Services, Inc.][2]) That is the real mismatch.

For the freemium MVP, I’d use **SQLModel or SQLAlchemy only if you want local dev speed**, but I would keep it trapped inside `repositories/sqlite_*.py`. SQLAlchemy is explicitly a SQL toolkit/ORM, not a general persistence abstraction. ([SQLAlchemy][3])

My recommendation for this project:

```text
Use Pydantic models for app/domain objects.
Use pyturso/sqlite directly or SQLModel for local storage.
Use Repository interfaces for all data access.
Later use PynamoDB or boto3 for DynamoDB.
Do not share ORM models between local and cloud.
```

For the MVP, this is enough structure:

```text
app/
  main.py
  api/
    merit_events.py
    vows.py
    repentance.py
    transfers.py
  domain/
    models.py
  repositories/
    interfaces.py
    sqlite_merit_events.py
    sqlite_vows.py
    sqlite_transfers.py
    dynamo_merit_events.py  # later
  services/
    merit_service.py
    vow_service.py
``
The conversion is only ugly if your FastAPI routes are full of SQLAlchemy query objects. Keep routes/service code speaking in terms of:

```python
await repo.list_recent_events(user_id)
await repo.add_vow(user_id, vow)
await repo.record_repentance(user_id, repentance)
```

Then the future DynamoDB switch is annoying but sane.

The honest answer: **no ORM will pivot nicely from SQLite/Turso to DynamoDB. A repository boundary will.**

[1]: https://pynamodb.readthedocs.io/?utm_source=chatgpt.com "Welcome to PynamoDB's documentation! — PynamoDB 6.0.1 ..."
[2]: https://aws.amazon.com/blogs/compute/creating-a-single-table-design-with-amazon-dynamodb/?utm_source=chatgpt.com "Creating a single-table design with Amazon DynamoDB"
[3]: https://www.sqlalchemy.org/?utm_source=chatgpt.com "SQLAlchemy - The Database Toolkit for Python"
