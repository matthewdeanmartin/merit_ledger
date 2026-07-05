# Merit Ledger

A private-first Buddhist practice journal for recording merit, vows, repentance,
dedication, and rejoicing. It supports Zen, Chinese Mahayana, Nichiren, Pure Land, and
secular ethical-practice modes.

Merit Ledger is a small, calm desktop app. Everything stays **on your own machine** — no
account, no cloud, no telemetry, no feed, no leaderboard. It is a practice tool, not a judge.

> "A quiet place to record wholesome practice."

## Requirements

- Python **3.10+**
- [`uv`](https://docs.astral.sh/uv/) (recommended) — handles the virtualenv and dependencies for you

## Quick start (from a checkout)

```bash
uv sync          # install dependencies into a local .venv
make run         # start the backend + open the app window
```

That's it. `make run` launches the local FastAPI backend, waits for it to report healthy,
then opens the Pygame window. On first launch you'll be walked through onboarding
(choose a tradition, a point style, and a name); after that it goes straight to your dashboard.

No `make`? Run the console script directly:

```bash
uv run merit_ledger
```

Or, once installed as a package (see below):

```bash
merit_ledger
```

## Handy commands

| Command | What it does |
| --- | --- |
| `make run` | Launch the desktop app (backend + window). **Start here.** |
| `make backend` | Start the backend only, wait for `/health`, then exit (smoke check). |
| `make backend-dev` | Run the API with autoreload at `http://127.0.0.1:8765` (Ctrl-C to stop). |
| `make where-data` | Print where your ledger is stored on disk. |
| `make print-url` | Print the local backend URL. |
| `make help` | List every available target. |

### Exploring the API

The backend is a normal FastAPI app. With `make backend-dev` running, open
`http://127.0.0.1:8765/docs` for interactive Swagger docs, or hit `GET /health` to confirm
it's up. The frontend talks to this same local API over HTTP — nothing leaves your machine.

## Where your data lives

Your ledger is a single local SQLite file, created on first run. Run `make where-data` to
see the exact path. By platform:

| OS | Location |
| --- | --- |
| Windows | `%LOCALAPPDATA%\MeritLedger\merit_ledger.sqlite3` |
| macOS | `~/Library/Application Support/MeritLedger/merit_ledger.sqlite3` |
| Linux | `~/.local/share/MeritLedger/merit_ledger.sqlite3` |

You can export everything to JSON or Markdown (and re-import JSON) from **Settings → Data**.
Settings also offers two resets: **Clear user data** (wipes the practice ledger but keeps
your profile and settings — handy for testing) and **Clear ALL data** (a full factory reset
that re-runs onboarding).

> ⚠️ Exported files may contain private reflections. Review before sharing.

## Installation (as a package)

Not published to PyPI yet. From a checkout you can build and install the wheel:

```bash
uv build
pipx install dist/merit_ledger-*.whl   # or: pip install dist/merit_ledger-*.whl
merit_ledger
```

## How it's built

- **Frontend:** Pygame (`pygame-ce`) — scene-based UI, one calm theme per tradition.
- **Backend:** a local FastAPI server the app starts on `127.0.0.1:8765`.
- **Storage:** SQLite modeled as a DynamoDB-style single table, so the domain logic is
  storage-agnostic and cloud migration stays open later.
- **Architecture:** business logic lives in the backend (domain + services); the frontend
  is a thin view layer that only calls the HTTP API.

See [`spec/spec.md`](spec/spec.md) for the full product spec and [`sprints/`](sprints/) for
the incremental build plan.

## Development

```bash
make check      # full local quality gate (format, lint, security, tests, types)
make test       # pytest with coverage
make typecheck  # mypy strict
```

Tests run headless; the Pygame-touching tests set `SDL_VIDEODRIVER=dummy` automatically.

## Contributing

See [CONTRIBUTING.md](docs/extending/CONTRIBUTING.md).

## License

MIT — see [LICENSE](LICENSE).

## Changelog

See [CHANGELOG.md](CHANGELOG.md).
