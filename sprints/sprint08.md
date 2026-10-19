# Sprint 8 — Texts and build-a-practice
**State:** done 2026-10-19 (desktop and board); the owner reviews it in place
**Depends on:** 3, 6   **Parallel with:** 9

## Goal
Compose a practice from elements (meditate, chant, prostrations, recite, read, copy, art) and
render it two ways: a **printable** document, and a **follow-along** reader that 3–4 people page
through on their own phones while the leader calls the pages.

## Read first
- `../spec/domain.md` § Texts and practices
- `../spec/v2_decisions.md` § Build-a-practice, § Worldview and tone (liturgy policy)

## Already decided
- Follow-along means each reader presses "next" themselves. There is **no device sync**. Every page
  has its own URL (`/practice/{id}/{page}`) so people can catch up.
- Texts are **original LLM-drafted renderings, reviewed by the owner**, and each records its
  provenance. Don't paste modern copyrighted translations. Public-domain translations are fine
  if they're cited.
- The naturalistic framing applies: texts can keep traditional names (the Lotus Sutra, Guanyin as a
  symbol), but there's no literal intercession or celestial-saviour language in household-authored framing.
- A meditate element can launch the Sprint 6 timer with its preset.

## Owner request (2026-10-19)
> "build a practice should include the Nichiren/SGI format because it is just so popular and easy
> to do. It is basically mahayana in its bones."

Built as the **Gongyo (Nichiren / SGI style)** template:
- three daimoku;
- Lotus Sutra ch. 2, the recited portion, with the ten factors three times;
- ch. 16, the verses (Jigage);
- chanting Nam-myoho-renge-kyo (108, with the mala timer);
- silent prayers, in our own words;
- three daimoku to close.

The recitation is the romanized Japanese reading, as in SGI books, with the Chinese and a short
English summary after it.

## Questions for the owner
Built with these defaults for review in place (the owner's preference), each easy to change:
- **Starter set:**
  - Gongyo: ch. 2, the ch. 16 verses, daimoku and silent prayers.
  - Shared Mahāyāna texts: the Heart Sutra (Chinese and our English), the verse of repentance
    (T293), the Three Refuges, the Five Precepts (from the precepts catalog), a dedication, a
    loving-kindness reflection and an evening reflection.
  - A retelling of the ch. 16 physician's parable.
  - **The Eight Precepts are deferred** until they get a sourced detour like the others.
- **Review:** front matter has `provenance`, `reviewed_by` and `reviewed_on`. Every built-in text
  starts as `llm-draft`.
- **Paper:** the print view doesn't fix a page size, so Letter and A4 both work. A booklet layout
  is deferred.
- **Romanization:** romaji for the Nichiren recitations, which is how they're chanted. The Chinese is
  shown for the Lotus Sutra, Heart Sutra and repentance verse. Pinyin and Pāli are deferred.

The original questions:

1. **The starter text set.** The proposal:
   - Three Refuges
   - Five Precepts (and the Eight Precepts for uposatha)
   - Heart Sutra (our rendering)
   - Dedication of merit
   - Metta reflection
   - The daimoku and a Lotus Sutra chapter summary (ch. 2 or 16)
   - Verse of atonement
   - A short evening reflection
2. Who reviews LLM drafts, and where is approval recorded? The proposal: `provenance: llm-draft-reviewed`
   with a reviewer and date.
3. Paper size for printing: Letter, A4 or both? Should there be a booklet layout (folded A5 or half-letter)?
4. Should built-in texts include pinyin, romaji or Pali alongside English?

## Deliverables
- [x] Text library:
  - Built-ins as markdown in the web bundle (`meritledger_ui/public/texts/*.md`, with front matter
    for provenance and license).
  - Household texts on the SD card via `/api/v1/texts`, falling back to a small NVS quota when there's
    no card.
- [x] PracticeDoc CRUD (`/api/v1/practices`), with the elements from domain.md and a bound.
- [x] Builder UI: add, reorder and remove elements, choose texts, books and timer presets, and preview.
- [x] **Print view:** print CSS, page breaks between sections, readable at arm's length (≥ 14 pt), no
  navigation chrome. Test in Chrome's and Safari's print preview.
- [x] **Follow-along view:**
  - Large type, one element or chunk per screen.
  - Next and previous buttons, swipe, and the page number in the URL.
  - A Wake Lock while open (HTTPS).
- [x] Doing a practice: "I did this" at the end logs entries for its elements (for example, 20 min meditation and
  108 daimoku).
- [x] A text pager: split long texts into screen-sized pages by paragraph (a pure function, tested).

## Exit criteria
- [x] Vitest: the pager splits on paragraph boundaries and never produces an empty page, and URL ↔
  page round-trips work.
- [ ] Three phones open the same practice and each pages independently. A late joiner jumps to page N
  by URL. **Owner to try in place.** It's built that way: each page is `/practice/{id}/{n}`, and
  there's no shared state.
- [ ] A printed practice (photo or PDF in `spec/measurements/`) is legible and has no orphaned headings.
  **Owner to print one.** The print CSS uses 14 pt type, keeps headings with what follows, and
  hides the app's chrome.
- [x] Every built-in text has provenance and license front matter, enforced by a test.

## Out of scope
Leader-controlled sync, audio recordings of chants.

## Handoff
**What was built**
- **Built-in texts:** `meritledger_ui/public/texts/*.txt` plus `index.json`.
  - They're markdown with front matter. The `.txt` extension is because the board's asset bundler
    serves no `.md` type.
  - A vitest test checks every file's provenance, license and source, and that the index matches
    the files.
- **The gongyo texts are verified against CBETA by `tools/gongyo_check.py`:**
  - Ch. 2 (T262 p5b25–c13) and the ch. 16 verses (p43b12–44a04) are kept there as (Chinese
    phrase, romaji) pairs, one reading per character.
  - The check confirms the Chinese is exactly CBETA's, every character has one reading, and the
    text files say every phrase.
  - Whether each reading matches the owner's gongyo book is the review.
- **Household texts:**
  - Routes: `GET|POST /api/v1/texts`, `GET|PUT|DELETE /api/v1/texts/h<id>`.
  - Bodies are `ml/texts/<id>.md` on the card. Without a card they go in the snapshot, at most
    4 KB each and 16 KB in all.
  - Guests can't add them.
- **Practices** (`practice_docs.rs`):
  - Routes: `GET|POST /api/v1/practices`, `GET|PUT|DELETE /api/v1/practices/{id}`.
  - Up to 40 steps, each checked: its kind, a real text, a timer, and a practice kind it logs as.
  - Public ones can be followed without signing in.
- **"I did this":** `POST /api/v1/practices/{id}/done` records one entry per step that logs, with
  the practice's name as the note.
- **Export and import** carry texts (with bodies) and practices. A step's household text and
  custom kind are remapped on import.
- **UI:**
  - `/build` lists the practices and the templates: gongyo, evening practice, Lotus Sutra study
    and sutra copying.
  - `/build/:id` is the builder.
  - `/practice/:id/:page` is follow-along: big type, Back/Next, swipe, arrow keys and a wake lock.
    "Open the timer" goes to `/timer?from=<preset>`.
  - `/practice/:id/print` is the print view, with a Sources section (CBETA's licence asks for
    attribution).
  - `/texts` is the library and reader, and where household texts are added.
  - "Practice" is in the nav.
- **Bundle:** the vow, precept and practice routes moved from `Api` to `ContentApi`, which only
  lazy pages import, so the first load stays under 300 kB (298.9 kB).

**Deferred**
- Eight Precepts, pinyin and Pāli, and the booklet layout (see above).
- Books in `read` steps (Sprint 12; a step's note says what to read for now).

**Measured on the board (2026-10-19):** `/texts/lotus-2.txt` 1.3 s, `/texts/index.json` 1.8 s,
`/api/v1/practices` 1.0 s, on the household's weak Wi-Fi.
