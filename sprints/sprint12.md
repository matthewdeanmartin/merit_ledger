# Sprint 12 — Books via Open Library
**State:** done 2026-10-19 (desktop and board)
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

## Questions for the owner (answered 2026-10-19)
- **No API key.** Open Library's search and covers are open (owner: "Let me know if we need API
  keys for open Library").
- **The browser calls Open Library directly.** It answers `access-control-allow-origin: *`
  (checked), so there's no proxy and the board makes no outbound calls for books.
- **Covers are shown,** fetched by the phone from covers.openlibrary.org.
- **Progress is in pages,** with a percentage when Open Library knows the page count.

The original questions:

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

## What was built
- **`reading.rs`:** reading lists. Each `StoredReading` holds the Open Library work, title,
  author, cover, status (want, reading, done), page and pages, visibility, and start and finish
  dates.
  - One book per work per person.
  - Logging a session records a `reading` entry for its minutes, linked to the book by the new
    field `StoredEntry.reading` (15). It moves the page, and turns "want" into "reading".
  - Public books show on the profile and on the house's shelf.
- **Routes:** `GET|POST /api/v1/reading[?person=]`, `PATCH|DELETE /api/v1/reading/{id}`,
  `POST /api/v1/reading/{id}/log` and `GET /api/v1/shelf`.
- **Practices:** a `read` step can point at a book (`StoredElement.book`: `r<id>` or an Open
  Library work), checked like texts. The builder offers your list and fills in "what to read".
- **Export and import** carry reading lists.
- **UI:**
  - `/books`: search by title, author or ISBN; your list by status, with progress bars,
    "Log reading" (minutes and page), finished, private and remove. "Books" is in the nav.
  - Also a "Reading" section on profiles, and "The house is reading" on the House page.
- **`books.ts`:** the search URL (ISBN or words; only the fields used, 20 at a time), the
  parser, covers and progress. It's tested against a recorded reply
  (`fixtures/ol-search-lotus-sutra.json`), with no live calls.

## Exit criteria
- [x] Response parsing is tested against a recorded fixture.
- [x] Searches take under 2 s: 0.3–1.0 s from the house's network (to Open Library directly; the
  board isn't involved).
- [x] A reading session shows in stats as minutes of reading (`tests/reading.rs`: the profile's
  week and category).

## Handoff
- **No proxy:** `GET /api/v1/books/search` wasn't needed (CORS is open). If Open Library ever
  closes CORS, that's the fallback.
- **Typed-in books** (no Open Library work) are allowed by the API; the UI only adds from search
  for now.
- **Also this session (owner feedback):**
  - The Chinese fasting days are now named "Fasting day (lunar 9/14)": the bare "9/14" read like
    a Gregorian date. The golden files were regenerated on purpose.
  - The screen always rotates at least two cards (today and an hourly verse), and dims to 5%
    from 22:00 to 06:00.
  - A pixel shift was tried and dropped (owner: it would be annoying).
