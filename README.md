# Merit Ledger

A household Buddhist practice ledger that lives on a small touchscreen board in the house.

You tap a practice on your phone and it's recorded for the day, Strava-style. Merit Ledger also
covers:

- sitting with bells on your phone
- following the household's lunar calendar with its uposatha and feast days
- printing a practice, or following along with it on a phone
- keeping vows as a blog
- tracking what you're reading via Open Library
- inviting people by Mastodon DM or email

It's built for one household's Chinese-descended, vegetarian, naturalistic practice first, with
features for other people second.

> **v2 is in progress.** It's a rewrite that runs on an ESP32-S3. v1, the Python desktop app, is
> archived in [`pygameui/`](pygameui/README.md) and still runs.

## Layout

```
meritledger_rs/   Rust: API, storage, firmware (on miniframework)
meritledger_ui/   Angular UI, served from the board
spec/             v2 decisions, domain sketch, roadmap, measurements
sprints/          one self-contained file per sprint, so work can be handed off
tools/            firmware build (and, from Sprint 1, deploy) scripts
pygameui/         v1, archived: Python + FastAPI + SQLite + Pygame
```

Start with [spec/v2_decisions.md](spec/v2_decisions.md) and [spec/roadmap.md](spec/roadmap.md).
About the calendar dates (and what they don't try to be): [docs/CALENDARS.md](docs/CALENDARS.md).
The precept sets, with their sources: [docs/precepts/](docs/precepts/README.md).

## Requirements

- The sibling repo **`C:\github\microcontroller`** (miniframework), checked out next to this one.
  The Rust crate and the UI both use it by relative path.
- Rust (stable) for desktop. For firmware, the `esp` toolchain (`espup`) and ESP-IDF 5.5.3 at
  `C:\Espressif`.
- Node 24 and npm.
- Git Bash and `make`.

## Quick start (desktop)

```sh
cd meritledger_ui && npm install && cd ..
make test          # Rust tests + UI type check + vitest
make run-bundle    # build the UI, serve UI + API on http://127.0.0.1:8089
```

For UI work, run `make run` in one shell and `make web-dev` in another. The second serves
http://localhost:4200 with hot reload, and proxies `/api` to :8089.

`make check` is the full gate: tests, fmt, clippy `-D warnings`, and a production UI build.

## On the board

The board is a Waveshare ESP32-S3-Touch-LCD-1.47. Read [DEPLOY.md](DEPLOY.md) first.

```sh
make certs         # once: meritledger.local from the household CA
make firmware      # build only; never touches a board
make board-check PORT=COM5   # read-only identity check
make update PORT=COM5        # app-only update; keeps everyone's data
```

## License

MIT. See [LICENSE](LICENSE).
