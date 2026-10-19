# Sprints

Each sprint is one focused chunk of work. A developer, human or AI, should be able to pick one up
cold, with no conversation history. The order and dependencies are in
[../spec/roadmap.md](../spec/roadmap.md). The v1 sprint history is in `../pygameui/sprints/`.

## Picking up a sprint

1. Read [../spec/v2_decisions.md](../spec/v2_decisions.md). It's short and settled: don't relitigate it.
2. Read the sprint file's **Read first** list.
3. Check that its **Depends on** sprints are marked done in the roadmap.
4. Ask the owner the sprint's **Questions for the owner** before building the parts they affect.
   Record the answers in the sprint file.
5. Build. Keep the sprint's checklist current as you go.

## Definition of done (every sprint)

- `make check` is green: Rust tests, `cargo fmt --check`, clippy with `-D warnings`, the UI
  typecheck, vitest, and a production `ng build` within the Angular budgets.
- New behaviour has tests. Rust logic gets unit tests in its module, the API gets tests in
  `meritledger_rs/tests/`, and pure UI logic gets `*.test.ts` files (vitest, no TestBed).
- Every exit criterion in the sprint file is checked, or explicitly deferred with a reason.
- From Sprint 1 on, a sprint that changes firmware is demonstrated on the board, and **the
  owner approves every flash** (see below).
- `spec/domain.md` is updated if the sprint changed an entity's shape.
- The sprint file's **Handoff** section is filled in: what was built, what was deferred and why,
  and surprises.
- The roadmap's State column is updated, and **the next sprint's file is checked**. If what you
  learned changes it, edit it before you stop.

## House rules

- **Shell:** Git Bash. `make` targets, POSIX scripts, no `.bat` or PowerShell helpers.
- **Python** (only in `pygameui/` and tool scripts): always use `uv run`.
- **Git:** the owner does all branching and committing. Agents don't commit, push, branch or tag
  unless the owner asks in that session.
- **Boards:** never flash, erase or reset a board without the owner's go-ahead for that session.
  Check the MAC address and flash size before writing. After the first install, use app-only updates,
  which keep the data. Read board state over HTTP (`/api/v1/sys`, `/api/v1/log`) rather than with
  long serial captures.
- **External services:** don't post to Mastodon, send email, or file upstream issues as a test
  against real accounts. Use the mock Mastodon server (`../microcontroller/gamematcher/tools/mock_mastodon.py`)
  and a mail sink. Live tests happen with the owner watching.
- **Framework changes:** if a sprint needs something in `../microcontroller/miniframework`, make
  the change there in a separate, self-contained patch with its own tests, and say so in the handoff.
  Don't copy framework code into this repo.
- **Bounds:** every collection has a fixed capacity and answers 409 `full`. Nothing evicts
  silently unless the domain says it's a ring.
- **Wire compatibility:** add `message!` fields with new numbers. Never renumber or reuse one.
- **Tone:** plain and warm. Merit and demerit are fine. No privacy or "not a judge" lecturing.

## Sprint file template

```
# Sprint N — Title
**State:** not started | in progress | done YYYY-MM-DD
**Depends on:** …   **Parallel with:** …

## Goal
## Read first
## Already decided
## Questions for the owner
## Deliverables        (checklist)
## Exit criteria       (testable)
## Out of scope
## Handoff             (filled in at the end)
```
