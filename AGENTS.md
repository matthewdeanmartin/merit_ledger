# AGENTS.md — guidance for AI contributors

Merit Ledger v2 is a Rust API (`meritledger_rs/`) and an Angular UI (`meritledger_ui/`) served from an
ESP32-S3, built on `../microcontroller/miniframework`. v1 (Python) is archived in `pygameui/`, which
has its own `AGENTS.md`.

## Before you start

1. Read `spec/v2_decisions.md`. Those decisions are settled.
2. Find your sprint in `spec/roadmap.md`, and follow `sprints/README.md`, which covers the
   definition of done, the house rules and the handoff.
3. Ask the owner the sprint's open questions before you build the parts they affect.

## Commands (Git Bash)

```bash
make test        # Rust tests + UI typecheck + vitest
make check       # full gate: + fmt, clippy -D warnings, production ng build
make run         # API on :8089
make web-dev     # UI dev server on :4200
make run-bundle  # UI + API on :8089
make firmware    # S3 image; compile only
make legacy-test # v1 tests (uses uv)
```

Python, in `pygameui/` and tool scripts only, always runs through `uv run`.

## Hard rules

- **Git:** the owner does all branching and committing. Don't commit, push, branch or tag unless
  asked in the session.
- **Boards:** never flash, erase or reset without the owner's go-ahead in this session. Check the MAC first.
- **External services:** no test posts to Mastodon, no test emails to real people, no upstream issues. Use
  mocks, and do live checks with the owner watching.
- **Wire messages:** append fields with new numbers. Never renumber or reuse one.
- **Bounds:** fixed capacity, 409 `full`, no silent eviction.
- **Framework:** improve `../microcontroller/miniframework` with self-contained patches. Don't copy its code here.

## Conventions

- Match gamematcher's style: module doc comments that explain *why*, plain `std` Rust, and
  `message!` structs in `messages.rs`.
- The UI is standalone Angular components with signals, zoneless, and no UI library. Pure logic goes
  in plain `.ts` modules with `*.test.ts` beside them.
- Mobile first. Tap targets are at least 44 px.
- Tone is plain and warm. Merit and demerit are fine. No privacy lectures.
