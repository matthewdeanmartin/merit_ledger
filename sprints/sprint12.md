# Sprint 12 — Books via Open Library
**State:** not started
**Depends on:** 3   **Parallel with:** 9–11

## Goal
Search books on Open Library, keep a reading list with progress, log reading sessions as
practice, and point a practice's "read" step at a book.

## Read first
- `../spec/domain.md` § Books
- Open Library APIs: `search.json`, `/works/{id}.json`, covers (`covers.openlibrary.org/b/id/{id}-M.jpg`),
  and the usage guidance (set a User-Agent with contact info, be gentle, no bulk scraping).

## Already decided
- Use Open Library only, at least for now.
- Reading sessions are ordinary entries of a `reading` kind, linked to a ReadingItem.

## Questions for the owner
1. Should the **browser** call Open Library directly, or should the **board** proxy it? The proposal: the browser,
   which is simpler and keeps the board's outbound TLS free. The board only stores what you save.
   Check CORS, and proxy if blocked.
2. Should covers be shown (fetched by the phone from Open Library), or kept text-only for privacy and speed?
3. Progress as pages, percent or chapters?

## Deliverables
- [ ] Search UI: title, author or ISBN, results with cover, author and year, and "add to my list".
- [ ] ReadingItem CRUD (`/api/v1/reading`): status want, reading or done, and progress.
- [ ] "Log reading" from a book: minutes plus new progress, creating an entry.
- [ ] A Sprint 8 `read` element can reference a ReadingItem or Open Library work.
- [ ] A household shelf: what everyone is reading, on the profile and the feed.
- [ ] A proxy fallback, `GET /api/v1/books/search?q=`, if needed for CORS. It's rate-limited and cached
  briefly in PSRAM.

## Exit criteria
- [ ] Tests: Open Library response parsing against recorded fixtures (no live calls in tests).
- [ ] Searching from a phone returns results in under 2 s on household Wi-Fi.
- [ ] A reading session shows up in stats as minutes of reading.

## Out of scope
Reading on the device, SuttaCentral, CBETA, Google Books.

## Handoff
_(fill in)_
