# Sprint 10 — Notifications, reminders and invitations
**State:** done 2026-10-19 (desktop and board); the live DM/email run and the owner's review are pending
**Depends on:** 7   **Parallel with:** 12

## Goal
Reach people when the app isn't open. Add reminders for practice times and observances, feast-day
notices, a "call to service", and invitations to events: by Mastodon DM to people in the house,
and by email to people outside it.

## Read first
- `../spec/domain.md` § Calendar (Event), § Notifications
- `../microcontroller/gamematcher/api/src/notify.rs`, `outbound.rs`, `mail.rs`, `announce.rs`, and
  `spec/08-mastodon.md`. Port the outbox, Brevo mail, the household bot and coalescing.

## Already decided
- Channels are the board screen, Mastodon DM and email. Each person chooses which.
- Browser push is optional and goes last, if at all. A paid push service is later and out of this sprint.
- People in the house are reached via the household Mastodon bot on mastomini. Any Mastodon instance
  must work (mastodon.social compatible). Outside people are reached by email via Brevo.
- Outbound network work happens on a worker, never on the serving task. The queue is bounded.

## Owner direction (2026-10-19)
> "Notifications (mastodon via mastomini), reminder (emails), invitations (follow pattern of intents
> as in nanacoing and gamematcher). And the board will display something."
> "yeah, intents, where you pop open a destination in a social media site with info prefilled."

- **Notifications** go to Mastodon, as a DM from the household bot on mastomini.
- **Reminders** go out by email (Brevo).
- **Invitations** are intents, as in gamematcher's `ui/src/app/invite.ts`. The host's phone opens
  email, a text message, WhatsApp, Telegram, Mastodon or Bluesky with the invitation written, and
  the guest answers on the board from a link with a token. The board sends no invitation email
  itself.
- **The board's screen shows something:** the clock, observances, calls to practice and the next
  event. This is pulled forward from Sprint 13; its touch menu stays there.

## Questions for the owner (answered with defaults, review in place)
1. **Reminders:** both kinds. Daily at a time ("sit at 07:00", optionally only some weekdays), and
   for observances ("19:00 the day before uposatha"). They're emailed and also show in the bell.
2. **Call to service:** any member (not guests). It goes to everyone in the house, or a chosen
   list through the API, and shows on the board's screen for 30 minutes.
3. **RSVP:** yes, maybe or no, plus "Bringing…".
4. **Bot and sender:** the bot is created on mastomini from Household → Messages (the admin login is
   used once and never stored). Brevo uses the household `.env` key (`BREVO_OTHER_KEY`) and its
   first verified sender.

The original questions:

1. Reminders: per person at a fixed time ("sit at 07:00"), relative to observances ("evening before
   uposatha"), or both?
2. Who may "call to service": anyone, or admins only? Does it notify everyone in the house or a chosen set?
3. Events: is RSVP yes/no enough, or should people also say what they're bringing (a feast potluck)?
4. Which account is the household bot on mastomini, and which address does Brevo send from?

## Deliverables
- [ ] A notification ring per person (bounded), with read flags and `/api/v1/me/summary` polling (60 s,
  only while visible).
- [ ] Reminder rules and a scheduler on the board tick. They only fire with a synced clock, with
  catch-up rules after a reboot (fire once if missed by less than 1 h, otherwise skip).
- [ ] Feast-day and observance notices, driven by Sprint 7's `observances()`.
- [ ] `POST /api/v1/call` (call to service): notifies the household, with an optional message and time.
- [ ] Events: CRUD, invitees (people or email addresses), RSVP, and an `.ics` attachment or link.
- [ ] Outbox: Mastodon DM via the bot (OAuth app on the instance, with the token in `Kv`) and Brevo email,
  with retry and backoff and a bounded queue. Use the mock Mastodon server and a mail sink in tests.
- [ ] UI: notification bell, preferences per channel, the events page, and the RSVP page (it works for
  email invitees without an account, via a token link).

## Exit criteria
- [ ] Tests:
  - Reminder catch-up after a simulated reboot.
  - Coalescing (ten quick changes → one DM).
  - A full queue → 409.
  - Email and DM payloads against the mock and the sink.
- [ ] A live run with the owner watching: one DM through mastomini and one email to the owner's address.
- [ ] An idle tab makes 10 requests or fewer in 10 minutes.

## Out of scope
Browser push (Sprint 14, optional), SMS, a paid push service.

## What was built
- **`notify.rs`:** the bell, plus reminders, observance notices and calls to service.
  - **The bell** is a ring of 30 per person. Unread news of the same kind about the same thing
    coalesces into one line, and only a new line is sent on, so ten quick answers make one DM.
  - **Channels:** Mastodon DMs for everything except reminders, which are emailed. Each person sets
    their handle and can turn DMs and email off.
  - **Reminders** fire on the minute tick with a synced clock. After a reboot, one missed by less
    than an hour still fires; older ones are skipped.
  - **Observance notices** go out once, at 07:00 household time.
  - **Rejoicing** now notifies the person rejoiced in.
- **`events.rs`:** events, invitees, RSVPs with "bringing", and a "starting soon" reminder an hour
  ahead.
  - Guests get token links and answer at `/e/:id?t=…` without an account.
  - Each event has an `.ics` file at `/cal/event/:id.ics`, for its people or a guest's token.
- **`outbound.rs`:** the outbox, ported from gamematcher and slimmed to one bot and email.
  - The queue (32) is persisted in the Kv, and a full queue answers 409.
  - Idempotent retries back off from 2 minutes to an hour.
  - Brevo transactional email, at most 100 a day.
  - The bot can be created on mastomini through its admin API, or set up from any account's token.
  - The work runs on a worker thread.
- **Routes:**
  - `/api/v1/me/summary`, `/api/v1/me/notifications[/read]` and `/api/v1/me/reminders[/{id}]`.
  - `/api/v1/call` and `/api/v1/calls`.
  - `/api/v1/events[/{id}[/rsvp|/cancel|/guests[/{gid}]|/guest|/guest-rsvp]]`.
  - `/api/v1/admin/{delivery,bot,mail,delivery/test,delivery/retry/{id},screen}`.
  - `/api/v1/display/frame.bmp`.
- **UI:**
  - The bell in the header polls once a minute, only while visible.
  - Pages: `/notifications`, `/events`, `/events/:id`, `/e/:id` (guests) and `/admin/messages`
    (bot, email, tests, outbox, and the screen with a live preview).
  - The Settings page has the Mastodon handle, the DM and email switches, and reminders.
  - The House page has "Call everyone".
  - Invitations use intents: email, SMS, WhatsApp, Telegram, Mastodon, Bluesky, the share sheet,
    or "copy link".
- **The screen:**
  - `components/display/ml_display.c` is a JD9853 strip driver. Its init sequence is from
    CircuitPython's port for this board (MIT).
  - `src/screen/` holds the cards, with the canvas and text ported from gamematcher. The font is
    Atkinson Hyperlegible (SIL OFL, `fonts/OFL.txt`).
  - A card every 10 s: the clock and date, today's observances or the next one, and how many
    practised today; a fresh call to service; the next event this week.
  - Admin settings: 180° flip, colour byte order, and brightness (default 40%).
- **Bundle:** sign-in pages, admin calls and practice-page calls load on demand. The first load is
  292 kB, under the 300 kB budget.

## Measured on the board (2026-10-19)
- **Screen:** `JD9853 ready` 0.3 s after boot. The orientation and colours need the owner's eye.
  If the screen is upside down or the colours are off, it's two switches on Household → Messages,
  no reflash.
- **Internal RAM:**
  - The first build left 8.6 KB free internal RAM, with a low point of 4.3 KB. That was
    unsafe.
  - Fixes: `CONFIG_SPIRAM_MALLOC_ALWAYSINTERNAL` went from 16384 to 4096, and the display strip
    from 24 rows to 8 (5 KB).
  - Now: 53.5 KB free, low point 33.5 KB, largest block 30 KB (before this sprint: 45 KB free).
- **Brevo:** the setup reaches Brevo once Wi-Fi is up (it retried while the board was still
  joining) and found the verified sender.

## Exit criteria
- [x] Tests:
  - Reminder catch-up after a simulated reboot (`notify.rs`).
  - Coalescing: ten answers make one DM.
  - A full queue → 409 (`tests/notify.rs`).
  - DM and email payloads against a fake mastomini and Brevo.
- [ ] **A live run with the owner watching.** One DM through mastomini (set up the bot on Household
  → Messages, add your handle in Settings, then "Send me a test DM"), and one email ("Send me a
  test email").
- [x] An idle tab makes at most 10 requests in 10 minutes. The bell polls once a minute while
  visible and never while hidden (`invite.test.ts`).

## Handoff
- **Sprint 11 (Mastodon sign-in and publishing)** can reuse `outbound.rs`'s bot and queue for
  public posts. `markdown.rs` and `notify::Delivery` are the shapes to extend.
- **Sprint 13 (the screen and touch menu):**
  - The screen draws already. Next come touch (AXS5106L on I2C 41/42; the interrupt and reset
    pins are disputed, see the board skill file) and the touch menu.
  - `screen_cards()` is the place to add cards.
- **Deferred:**
  - Browser push (Sprint 14).
  - Inbound Mastodon replies as RSVPs (gamematcher has them; not needed yet).
  - A guest token is never rotated: removing the guest is how to revoke it.
