# Sprint 11 — Mastodon sign-in and publishing
**State:** done 2026-10-19 (desktop, mock walk, board); live posts with the owner pending
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

## Questions for the owner (answered with defaults, review in place)
1. **As whom:** the person's own account when they've linked one (the link asks for
   `write:statuses` and keeps the token). Otherwise the household bot, prefixed with their
   name ("Kid: 📿 Daimoku · 108×").
2. **Visibility:** unlisted by default. Each person can pick public, unlisted or followers-only
   in Settings.
3. **Template:** "🧘 Meditation · 30 min" plus the note. Vow posts are "🪷 Title", the text,
   and "(On my vow: …)". The board's link is added only on a `.local` instance like mastomini,
   because it doesn't resolve elsewhere.

The original questions:

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

## What was built
- **miniframework** (`crates/miniframework/src/oauth.rs`): a self-contained, additive patch
  with a new test.
  - `Config.extra_scopes` (registered with every instance; a stored registration without them
    is redone).
  - `OAuth::start_keeping_token` (asks for the extra scopes and keeps the token) and
    `take_token`.
  - `OAuth::revoke`.
  - Plain sign-ins still ask only to read the account, and gamematcher compiles unchanged.
  - **Uncommitted in the microcontroller repo; the owner commits it.**
- **`publish.rs`:** `publishable(item, link) -> Option<String>`, the whole rule.
  - Public content kinds post in full; generic ones post their category.
  - Private entries, demerits, repentance and noise never post, and nor do posts on non-public
    vows.
  - The truth table covers every built-in kind × every visibility.
  - It also links an identity to a person (one person per account), and holds the publishing
    preferences.
- **The outbox** (`outbound.rs`) gains `status` jobs:
  - Post, edit and delete, as the person (their link's token in the Kv, `tk<person>`, apart from
    the snapshot) or the bot.
  - Published items' status ids are remembered (up to 512), so edits and deletes follow and
    re-sharing is a no-op.
  - An edit or delete that arrives while its post is in flight waits behind it (found by the
    walk).
  - Unlinking queues a revoke.
  - The OAuth steps run on the same worker.
- **Routes:**
  - `GET /api/v1/auth/providers`, `POST /api/v1/auth/oauth/start` (sign in, or `link` the
    signed-in person) and `GET /api/v1/auth/oauth/{ticket}`.
  - `GET /auth/callback` → `/signin/finishing`.
  - `DELETE /api/v1/me/mastodon`.
  - `POST /api/v1/entries/{id}/publish` and `/api/v1/posts/{id}/publish`.
  - The `/me` PATCH takes `auto_publish` and `publish_visibility`.
- **Following:** `mutate` diffs entries, posts and vows. New fresh content auto-publishes for
  people who switched it on (imports, being old, never do). Changes edit the status; deletes, or
  turning something private, delete it.
- **Sign-in:** "Sign in with Mastodon" works only for accounts already linked from Settings
  (signed in with a PIN). There are no new accounts by Mastodon.
- **Instances:** mastomini (household CA) and mastodon.social (public CAs). On the desktop,
  `MERITLEDGER_MASTODON_MOCK=<url>` adds a mock.
- **UI:**
  - Sign-in buttons per instance, and the `/signin/finishing` page.
  - A Mastodon section in Settings: link or unlink, auto-share, visibility.
  - ↗ on your own shareable entries, and "Share to Mastodon" on public vows' posts.
- **Tools:**
  - `tools/mock_mastodon.py`: gamematcher's mock plus statuses and scopes.
  - `tools/mastodon_walk.py`: links, shares, deletes and signs in over HTTP against it.

## Exit criteria
- [x] The `publishable` truth table is fully tested (`publish.rs`), plus API tests
  (`tests/publish.rs`). No private, demerit or noise item produces text.
- [x] The walk passes against the mock. Linking, sharing (status appears), deleting the entry
  (status deleted) and Mastodon sign-in all work against the desktop server, with a fresh state
  each run.
- [ ] **Live:** one post to mastomini and one to a test account on mastodon.social, with the owner
  watching. Link in Settings, then ↗ on a sit.
- [x] Deleting an entry deletes its status on the mock (the walk, step 4).

## Handoff
- **The board after this sprint:** 55 KB internal RAM free, with a low point of 46 KB.
- **Edits** use `PUT /api/v1/statuses/:id` (Mastodon 3.5+).
- **Statuses aren't removed on unlink:** unlinking leaves past statuses in place, as the UI says.
- **The board stores tokens in plain NVS:** flash isn't encrypted. That's the
  documented trade-off (the framework's OAuth spec); unlinking revokes.
- **Later:**
  - Admin-configured instances (gamematcher has them).
  - Timelines, follows, replies (out of scope).
  - Showing "shared" on entries (the board knows; the views don't carry it yet).
