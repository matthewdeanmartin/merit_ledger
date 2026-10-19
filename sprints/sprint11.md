# Sprint 11 — Mastodon sign-in and publishing
**State:** not started
**Depends on:** 4, 10   **Parallel with:** 12

## Goal
Sign in with Mastodon, link a Mastodon account to a person, and publish **content**, such as "sat
30 minutes" or a vow post, to Mastodon. Publish nothing that isn't visible on the board, and
publish no noise.

## Read first
- `../spec/v2_decisions.md` § Content vs noise, § People and privacy (Mastodon mirrors the board)
- `../microcontroller/miniframework/spec/OAUTH.md`, plus gamematcher's `identity.rs` and Sprint 6 notes
- The Mastodon API: `POST /api/v1/statuses`, OAuth scopes (`read:accounts`, `write:statuses`)
- Sprint 9's `meritledger_rs/src/markdown.rs` (the post renderer to share) and `vows.rs`: vow posts and their visibility, where only public vows' posts are seen by others

## Already decided
- The expected instance is mastomini.local, but **mastodon.social must work too** (public CA, standard API).
- Only `public` items of `content` kinds and vow posts can be published. `generic` items publish as their
  category text. `private` and noise items never publish.
- Publishing is per item (a "share" button), and each person can switch on auto-publish for content.

## Questions for the owner
1. Does publishing post **as the person** (their own account, which needs a stored `write:statuses` token)
   or **as the household bot**, mentioning them? The proposal: own account when linked, otherwise the bot.
2. Post visibility on Mastodon: public, unlisted or followers-only by default?
3. Post template, for example: "🪷 Sat 30 minutes · meritledger.local". Should it include a link to the profile
   (only resolvable on the LAN)?

## Deliverables
- [ ] Framework `oauth` feature enabled: "Sign in with Mastodon" plus linking an existing person.
- [ ] Token storage for `write:statuses` (encrypted at rest is out of scope; it's in a `Kv` namespace
  apart from the snapshot), with revoke and unlink.
- [ ] Publisher: queued through the Sprint 10 outbox. It's idempotent per item (stores the status id) and
  deletes or edits on Mastodon when the item is deleted or edited (best effort).
- [ ] Rules as a pure function `publishable(item, viewer=public) -> Option<Text>`, with tests for every
  visibility, polarity and noise combination.
- [ ] UI: a share button on entries and vow posts, the auto-publish toggle, and the linked account on the profile.

## Exit criteria
- [ ] The `publishable` truth table is fully tested. No private, demerit or noise item can produce text.
- [ ] Walk the sign-in and publish flow against `mock_mastodon.py`. Then do one live post to mastomini and one to a
  test account on mastodon.social, with the owner watching.
- [ ] Deleting an entry deletes its status on the mock.

## Out of scope
Reading timelines, following, replies.

## Handoff
_(fill in)_
