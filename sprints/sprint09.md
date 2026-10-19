# Sprint 9 — Vows and the vow blog
**State:** done 2026-10-19 (desktop and board); the owner reviews it in place
**Depends on:** 4, and the [precepts detour](detour-precepts.md) for the precept texts   **Parallel with:** 8, 12

## Goal
Track vows through their lifecycle and publish them like a blog on each person's board profile.
Show on each profile **which precepts the person has taken**, which may be only some of a set, and let a
taken precept become a vow. Sprint 11 mirrors posts to Mastodon.

## Read first
- `../spec/domain.md` § Vows
- v1's `../pygameui/merit_ledger/backend/domain/vow_state.py` and `services/vow_service.py`, plus their tests.
  Port the state machine and its tests.

## Already decided
- States: active, paused, retired, completed, and breached → repaired (repair links to a repentance entry).
- Vows are public by default, like everything else. A breach defaults to `generic` ("a vow was
  renewed"), and the person can choose otherwise.
- VowPosts are dated markdown posts attached to a vow, shown on the profile newest first.

## Owner request (2026-10-19): precepts on profiles
- **Sets** (Chinese Buddhist versions; texts and citations from the [detour](detour-precepts.md)):
  - the Five Precepts;
  - **the novice Ten Precepts** (沙彌十戒, not the ten wholesome deeds);
  - the Upāsaka precepts of the *Upāsakaśīla Sūtra* (6 major, 28 minor);
  - the Bodhisattva precepts of the *Brahmajāla Sūtra* (10 major, 48 minor).
- **People can take only some of a set** (owner: "people need to be able to take only some of them").
- **A taken precept can become a vow** (owner: "convert to vow. Good feature idea"), and then
  follows the vow lifecycle, including breach → repair. Converting is optional.
- On the profile: each set taken, with the date, an optional note (where, with whom), and which items
  were taken. Each set links to its page of precepts, with sources.

## Questions for the owner
The owner asked to review in place rather than answer up front ("I want to review things en
situ"), so these were built with defaults, each easy to change:

1. **A post is suggested, never posted.** Taking a vow (or making a precept into one) opens the vow
   page with a draft post, "Today I vowed: *…*". Completing a vow offers "I kept my vow". The
   person edits it or cancels.
2. **The markdown subset:** paragraphs, line breaks, `- ` lists, `*emphasis*`, `**strong**` and
   http(s) links. Everything else shows as typed. No images.
3. **An optional end date, and no daily check-in.** Daily keeping is already what the "Kept the precepts today"
   practice tile records; a per-vow check-in can come later if wanted.

## Deliverables
- [x] `vows.rs`: the state machine as a pure function (`transition(state, action) -> Result<state>`),
  with the v1 tests ported to Rust.
- [x] Routes: `/api/v1/vows` CRUD, `POST …/{id}/{action}`, and `/api/v1/vows/{id}/posts` CRUD.
- [x] Posts are stored on the SD card (long text), with the title and date in the snapshot. Without a card,
  there's a small NVS quota.
- [x] A safe markdown renderer for the agreed subset (escape everything else), shared by the board view and the
  Mastodon mirror.
- [x] Profile: a vows section and the blog list. A vow page shows its timeline of state changes and posts.
- [x] Breach → "repair" flow that creates a linked repentance entry.
- [x] **Precepts:**
  - Built-in sets from `catalog/precepts.json` (from the detour), at `GET /api/v1/precepts`.
  - `StoredPrecepts` per person and set: date, note, the item numbers taken (any subset), visibility.
  - Routes: `GET|PUT /api/v1/me/precepts/{set}`, plus `DELETE` for "no longer holding".
  - Profile section and precept-set pages (with citations).
  - "Make this a vow": creates a vow linked to the set and items.

## Exit criteria
- [x] Every v1 vow transition test passes in Rust, and illegal transitions → 409.
- [x] The markdown renderer escapes HTML and script (tested with hostile input).
- [x] Visibility tests for vows and posts, matching Sprint 4's rules.
- [x] Precepts: tests for partial taking (any subset, never an unknown item), a profile showing exactly
  what was taken, conversion to a vow, and visibility.

## Out of scope
Mastodon mirroring (Sprint 11), comments on posts.

## Handoff
**What was built**
- **`vows.rs`:**
  - v1's state machine, with `repair_in_progress` renamed `breached`, plus `repair`
    (breached → active), which records a `repentance` entry and links it.
  - v1's tests are ported.
  - Vows hold their own timeline (`events`, at most 64), so a vow page tells its story.
  - Others don't see a breach unless its person shows it (the default is `generic`). They see the
    repair as "Renewed this vow", and the vow as simply kept meanwhile.
- **Posts:**
  - Titles are in the snapshot, and bodies are `ml/posts/<id>.md` on the card.
  - Without a card, bodies sit in the snapshot: at most 2,000 bytes each and 16 KB in all.
  - Others see posts only on public vows; the markdown source goes only to its author.
- **`markdown.rs`:** the subset above, everything else escaped. Tested with script, `javascript:`
  and `data:` links, attribute injection and nested tags.
- **Precepts:**
  - `StoredPrecepts` per person and set: any subset, but never an unknown item.
  - Generic shows the set but not which items; private hides it.
  - "Make this a vow" needs the item to be taken, and refuses a second current vow for the same
    item.
- **Routes:**
  - `GET /api/v1/precepts` and `GET /api/v1/people/{id}/vows` are public.
  - `GET|POST /api/v1/vows`.
  - `GET|PATCH|DELETE /api/v1/vows/{id}`.
  - `POST /api/v1/vows/{id}/{pause|resume|breach|repair|complete|retire}`; the body is optional.
  - `POST /api/v1/vows/{id}/posts`.
  - `GET|PUT|DELETE /api/v1/posts/{id}`.
  - `PUT|DELETE /api/v1/me/precepts/{set}`.
- **Export and import** carry vows, posts (with bodies) and precepts, so the nightly backups do too.
  Imported bodies go to the card. See `docs/EXPORT_FORMAT.md`.
- **UI:**
  - The profile has Precepts, Vows (with "Take a vow") and Posts sections.
  - `/v/:id` is the vow page: actions with a note and who-sees-it, editing, the timeline, and the
    post composer.
  - `/precepts` and `/precepts/:set` show every item with its Chinese, our rendering and its CBETA
    links. They also hold "I've taken these" (tick any items) and "Make this a vow".
  - The styles are in `vow-styles.ts`, loaded lazily so the first load stays under its 300 kB budget.
- **Tests:**
  - `tests/vows.rs` covers lifecycle, visibility, posts on a card, editing, precepts and
    export/import.
  - The unit tests are in `vows.rs`, `precepts.rs` and `markdown.rs`.
  - On the UI side, `vows.test.ts` and `precepts.test.ts`.

**Measured on the board (2026-10-19):** `GET /api/v1/precepts` (30 KB) takes 4.1 s over the
household's weak Wi-Fi, and the UI fetches it once per visit. `GET /api/v1/people/1/vows` takes
1.1 s.

**Deferred**
- **Mastodon mirroring** is Sprint 11. `markdown::to_html` is the shared renderer.
- **Removing a person** with "delete their data" drops their posts from the snapshot but leaves
  their `ml/posts/*.md` files on the card. That's harmless, and cleanup can come with Sprint 12's
  storage page.
- **Posts written without a card** stay in the snapshot until edited with a card present (an
  import moves them right away).
- **If the precept catalog's 4 s matters,** gzip it at build time or cache it in the browser.
