# Sprint 10 — Notifications, reminders and invitations
**State:** not started
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

## Questions for the owner
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

## Handoff
_(fill in)_
