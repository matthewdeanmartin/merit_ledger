# Domain sketch

This is the shared vocabulary for v2. Each sprint turns its part into `message!` structs in
`meritledger_rs/src/messages.rs` and rules in its own module. Field lists here are a starting
point, not a schema. The sprint that builds an entity owns its final shape, and records any change
here in the same change.

Patterns to copy from gamematcher (`../microcontroller/gamematcher/spec/03-domain.md`,
`05-storage-limits.md`): fixed-capacity collections that answer **409 `full`** instead of silently
evicting, stable numeric ids, `rev` counters for stale-write checks, and times as UTC seconds with
calendar dates computed in the household zone.

## People

**Person**: an account.
- Fields: `id`, `display_name`, `role`, `pin_hash`, optional `email`, optional linked `mastodon`
  handle (`@user@host`), `calendar` choice, `publish` defaults, `created`.
- Roles:
  - `admin`: the owner. The first account on a fresh board.
  - `member`: kids and housemates.
  - `guest`: a house guest. Their account can be removed with everything they recorded.
- Sign-in is "tap your name, then enter your PIN", so names are unique (ignoring case and spacing) and the
  people list is public. Email is optional.
- A person without a PIN can't sign in; others record for them.
- Recording for someone else is allowed (household trust). Each entry keeps `recorded_by`.
- Bound: 16 people.

## Practice

**PracticeKind**: something you can do or log.
- Fields: `id`, `name`, `polarity` (`merit` | `demerit`), `category`, `unit`, `default_visibility`,
  `content` flag, `owner`, `archived`.
- `category`: meditation, chanting, prostration, recitation, reading, copying, art, precepts,
  service, repentance, or custom.
- `unit`: `minutes`, `count`, or `once`.
- `content`: whether entries can be published. Noise can't be.
- `owner`: built-in, household-wide, or one person's.
- Built-in kinds ship in the firmware as a JSON catalog. Custom ones live in the snapshot.
- Bound: 256 custom kinds.

**Entry**: one logged practice.
- Fields: `id`, `person`, `kind`, `date` (household-local YYYY-MM-DD), `started`, `ended`,
  `amount` (minutes or count), `note`, `visibility`, `source`, `recorded_by`, `rev`.
- `visibility`:
  - `public`: shown on the profile and may be published.
  - `generic`: shown publicly as just the category, such as "a repentance practice".
  - `private`: only the person and admins see it.
- Demerit kinds default to `private` and repentance to `generic`. For anyone except the person and admins,
  a demerit or repentance entry never shows details, whatever its visibility: at most its generic line
  (Sprint 4, owner decision). Generic views also hide the category.
- `source`: `tap`, `timer`, `board`, `manual`, or `import`.
- **Done today** is derived: a person and kind have at least one entry on today's local date.
- Timer events (start, interrupted, ended) are **not** entries. They're noise and are dropped or
  kept only as the timer's own state.

## Timers

**TimerPreset**: a named sequence of segments.
- Fields: `id`, `name`, `segments`, `bells`, `count_target`, `logs_as` (the PracticeKind a finished
  run records), `owner`.
- `segments`: each one sits, walks or counts, with a duration in minutes or a count.
- `bells`: `open` strike count, `interval` minutes or none, `close` strike count.
- The timer runs on the phone. The board only stores presets and receives the finished entry.

## Texts and practices

**Text**: liturgy or reading material.
- Fields: `id`, `title`, `body` (markdown), `provenance`, `license`, `tradition_tags`.
- `provenance`: `llm-draft-reviewed`, `household`, or `public-domain`.
- Built-in texts ship as static files in the web bundle. Household texts live on the SD card,
  because a long text doesn't belong in an NVS snapshot.

**PracticeDoc** (build-a-practice): an ordered list of elements.
- Fields: `id`, `title`, `elements`, `owner`, `visibility`.
- Elements:
  - `meditate` (minutes, optional TimerPreset)
  - `chant` (Text, repetitions)
  - `prostrations` (count)
  - `recite` (Text)
  - `read` (Book or Text, plus a range such as a chapter or pages)
  - `copy` (Text)
  - `art` (prompt)
  - `note` (free text)
- The doc renders two ways: as a **print** view (one document) and as a **follow-along** view
  (paged, one element or chunk per page). A URL addresses each page so readers can jump.
- **As built (Sprint 8):**
  - Elements are `StoredElement` (`kind`, `title`, `text`, `minutes`, `count`, `timer`, `logs_as`,
    `note`). `logs_as` is what "I did this" records.
  - Visibility is `public` or `private`.
  - Built-in texts are `public/texts/<id>.txt`; household ones are `h<id>`.

## Vows

**Vow**: ported from v1's state machine (`pygameui/merit_ledger/backend/domain/vow_state.py`).
- Fields: `id`, `person`, `text`, `state`, `started`, `ends`, `visibility`.
- `state`: active, paused, retired, completed, or breached → repaired.
- **Precepts taken** (Sprint 9): per person and precept set (Five, novice Ten, Upāsaka, Bodhisattva), the
  date, an optional note, and which items were taken (any subset). A taken precept can become a vow.
  The sets and their sources come from `docs/precepts/` (the precepts detour).
- **VowPost**: a dated blog post about a vow, shown on the profile and optionally mirrored to
  Mastodon.
  - Fields: `id`, `vow`, `person`, `title`, `date`, `rev`.
  - The markdown body is on the SD card (`ml/posts/<id>.md`), or in the snapshot when there's no card.
  - It is visible to others only when its vow is public.
- **As built (Sprint 9):**
  - States are `active`, `paused`, `breached`, `completed` and `retired`. A `repair` returns a
    breached vow to `active` and records a repentance entry. The full table is in `vows.rs`.
  - A vow also has an `events` timeline, and its precept link (`precept_set`, `precept_kind`,
    `precept_n`) when it was made from a precept.
  - **StoredPrecepts:** `person`, `set`, `date`, `note`, `majors`, `minors` and `visibility`, one per
    person and set.

## Calendar

**CalendarSystem**: `theravada`, `chinese`, `japanese`, or `none`.
- Chosen for the household, and each person can override it.
- Options: `sunday_shift` (move observances to the closest Sunday), `quarter_moons` (Theravada),
  `fasting_days` (`six` | `ten`, Chinese).

**Observance**: computed, not stored.
- Fields: `date`, `system`, `kind` (`uposatha` | `fasting_day` | `feast` | `festival`), `name`,
  `shifted_from`.

**Event**: something scheduled that people are invited to: a feast, a group sit, or a service.
- Fields: `id`, `title`, `start`, `end`, `observance` (optional link), `invitees`, `rsvps`, `rev`.
- Invitees are household people (notified by Mastodon DM) or outside email addresses.

## Books

**ReadingItem**: a book on someone's reading list.
- Fields: `person`, `ol_work` (an Open Library work key such as `OL45804W`), `title`, `author`,
  `cover_id`, `status` (`want` | `reading` | `done`), `progress`.
- Reading sessions are ordinary Entries of a `reading` kind that point at the item.

## Notifications

**Notification**: kept in a ring per person, as in gamematcher.
- Fields: `id`, `person`, `kind`, `text`, `link`, `created`, `read`.
- **Delivery** goes to the board screen, Mastodon DM and email, as each person chooses. Browser push
  comes later, if at all.

## Storage map

Settled in Sprint 5. The details are in [../docs/SD_CARD.md](../docs/SD_CARD.md).

| What | Where | Why |
|---|---|---|
| People, sessions, custom kinds, settings, the entry id lease | Snapshot in NVS (`mlstate`) | Small, changes rarely, must survive reboots |
| **Every entry** (with a card) | SD card `ml/journal/NNNNNN.log`: append-only, CRC per record, last record per id wins | Grows forever; a tap appends about 100 bytes |
| Recent entries (the last 60 days) | RAM, loaded from the journal at boot (or the snapshot without a card) | Today, edits, profiles |
| Older days as per-day totals (`stats::Tally`) | RAM, rebuilt from the journal at boot | Streaks and heatmaps beyond the window |
| Day totals of entries that aged out **without** a card | Snapshot (`archived_days`) | Stats survive with no card |
| Backups | SD card `ml/backup/YYYY-MM-DD.json` (nightly; 14 daily, 24 monthly) | Restore via import |
| Household texts (Sprint 8) | SD card `ml/texts/` | Large |
| Built-in catalog, texts, quotes | Web bundle (firmware) | Read-only, versioned with the code |
| Secrets (session salt, OAuth registrations, Brevo key) | Framework `Kv` | Not domain state |

Without an SD card the board keeps the snapshot window plus daily totals. The Household page says so once.
It doesn't nag.
