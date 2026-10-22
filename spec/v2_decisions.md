# Merit Ledger v2 — decisions

What the owner decided across two rounds of questions (2026-10-18). These are settled. Don't
reopen them in a sprint. If one turns out to be wrong in practice, raise it with the owner and
record the change here.

For the plan see [roadmap.md](roadmap.md), for the data model [domain.md](domain.md), and for
the work [../sprints/](../sprints/README.md).

## Shape

- **The website runs on the board.** It's a household homelab app, like
  `../microcontroller/gamematcher`, built on `../microcontroller/miniframework` (a Rust API plus
  an Angular UI using `miniframework-ng`). Laptops and desktops are not always on, so the board is
  the hub. "Works better than you'd think" on S3, C6 and P4 boards; S2 is too small.
- **Target board: Waveshare ESP32-S3-Touch-LCD-1.47.** It has a 172×320 touchscreen, 8 MiB of
  octal PSRAM, 16 MiB of flash and an SD slot. See
  `../microcontroller/BOARD_SKILL_ESP32_S3_TOUCH_LCD_1_47.md`. The larger 2-inch board is not the target.
- **Repo layout:**
  - `meritledger_rs/` holds the API, storage and firmware.
  - `meritledger_ui/` holds the Angular UI.
  - `pygameui/` holds v1, archived: the Python desktop app, which still runs.
- **Storage:** no SQLite and no Turso. Live state is a versioned CBOR snapshot in two
  CRC-checked NVS slots (gamematcher's proven scheme), held in PSRAM. **Long history and backups go
  to a 32 GB SD card.** Without a card, old detail is ephemeral, and people are expected to understand that.
- **Backups** go to the SD card automatically. The UI can also export to a documented, standard
  format: JSON for everything, CSV for practice entries, and iCalendar for calendars.
- **HTTPS is required for the meditation timer** because Wake Lock needs a secure context. The
  framework's household CA makes HTTPS workable. Plain HTTP still serves everything else, so a
  guest without the CA can use the site.
- **Clients:** phones and laptops. Bells and timer sounds play on the phone or laptop, never on the board.
- **Network:** inbound is household LAN only. Outbound internet is used for Mastodon, email
  (Brevo) and Open Library.

## People and privacy

- The household is the owner and their kids. **House guests can join**, but it's hypothetical.
- **Three ways in** (owner, 2026-10-21):
  - **PIN account:** anyone may make one while sign-up is open. They're a member, and an admin
    removes any that should have been guests.
  - **Guest:** a required name and no PIN. The account lives in that browser's cookie, so two
    guests get two accounts. There are at most 8 at a time. A guest unused for 90 days is deleted
    with everything they logged, and "End visit" deletes it at once.
  - **No account:** the dharma side reads without signing in (practices, texts, precepts, the
    calendar, the timer, book search, the house). Anything that writes needs an account.
- **Guests and visitors see only public things:** public entries and vows, never "generic"
  household detail. Members see generic items as before.
- The house trusts itself. Anyone can record a practice for anyone from the board's touch menu
  or the website. Auth friction on the board isn't wanted.
- **Data is public by default.** Each person has a profile page on the board.
- **Demerits and repentance default to private**, or to public with generic text such as
  "a repentance practice". Give people the tools and sensible defaults. **Don't lecture them about privacy**:
  no nag screens and no "have you thought about…" copy.
- **Mastodon only mirrors the board.** Nothing reaches Mastodon that isn't visible on the board,
  except notifications.

## Worldview and tone

- The app serves **this household's Buddhism first**, with features for other people second.
- The lineage descends from Chinese Buddhism and is **vegetarian**. Traditions that argue
  against vegetarianism don't appear as defaults.
- **Naturalistic reading:** no literal bodhisattvas, celestial buddhas, hells, Yama or
  intercession, and no efficacy from belief alone. The Dharmakaya and the Dharma exist, but they are not people.
  No-self is literal: nothing is reborn. Continuity runs through cause and effect, other people and a
  collective consciousness. Karma bookkeeping is for **self-improvement**.
- **Practice keeps you remembering you're a Buddhist.** Meditation, chanting and prostrations
  are meditative and philosophical activities: thinking through hard problems, or symbolic
  placeholders when the alternative would be doing nothing.
- **Practices come from anywhere:** sutra copying, painting thangka, and reading fiction and
  illustrating it are all practices. The philosophy is what has efficacy.
- **Nichiren is in.** It puts the Lotus Sutra (the Dharma) first: read the stories, act on them,
  and chant the title. **Pure Land is out** of this project entirely.
- **Tone:** merit and demerit are fine, and so is scoring. The v1 "not a judge" language came from a
  previous bot and is dropped.
- **Liturgy and texts:** use original, LLM-drafted renderings that the owner reviews, rather than
  copying modern translations, which are copyrighted even when the source is public domain.
- **Out of scope:** temple operations (buildings, parking, staff), dana and money, and begging.

## Features

### Effortless merit tracking, Strava-like
- One tap starts a practice, and that **records it as done for the day**.
- There are canned merit and demerit options, and people can add their own.
- **Content vs noise:** "I meditated 30 min" is content and can be published. Start, interrupt
  and end events, and "didn't get angry today", are noise and are never auto-published.

### Bells, timers, meditation (on the phone)
- Opening, interval and closing bells, including three strikes.
- Sitting and walking periods, sesshin-style.
- A counting or mala mode for chanting and prostrations.
- **No metronome.**

### Build-a-practice
- Compose a practice from elements: meditate, chant, prostrations, recite a long text, read X,
  copy a text, or make art.
- The result is **printable**.
- It can also be **followed along** on phones: 3 or 4 people open the same document and each
  presses "next page" themselves while the leader calls the pages out loud. There is no device sync.

### Calendars — "meritledger lunar calendars"
- These are our own computed calendars, deliberately not tracking any official body's rulings.
- **Three defaults:** Theravada-inspired (uposatha from moon phases), Chinese-inspired
  (lunisolar) and Japanese-inspired (fixed Gregorian dates).
- **The weekend option moves an observance to the closest Sunday.**
- **Feast days only give notice** ("today is a feast day"). Holidays are vegetarian feasts.
- **Invitations** go by Mastodon to people in the house (mastomini.local; any Mastodon, such as
  mastodon.social, must work) and by email to people outside it (Brevo, as in gamematcher).
- iCalendar feeds, as in gamematcher.

### Vows
- People track vows and publish them like a blog on their board profile, mirrored to Mastodon
  if they choose.

### Notifications
- Delivered by **Mastodon DM, email and the board's screen**.
- Browser push is optional and low priority ("useful 1x in a kalpa").
- A paid notification service may come later. It's not planned yet.

### Books
- **Open Library**: search, reading list and progress. A "read this" step in a practice can
  point at a book.

### Board screen (touch)
- Shows notifications, the lunar day and next observance, and Buddhist messages.
- Its **touch menu** offers *call to service* (notify the household) and *record practice*,
  which asks who it's for and adds an entry to that person's account.
