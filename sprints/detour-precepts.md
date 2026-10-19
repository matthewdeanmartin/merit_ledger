# Detour — Precept texts: research and sources
**State:** research done 2026-10-19; renderings await the owner's review
**Depends on:** —   **Before:** Sprint 9 (precepts on profiles)   **Parallel with:** 8

## Goal
Before the board shows anyone's precepts, get the precept lists **right and cited**. For each set the
owner wants, research the source text, settle the list and its numbering, note where traditions
differ, and draft our own plain-English rendering for the owner to review. Every line traces back
to a source.

## Why a detour
- Precepts are things people actually vow to keep. A wrong or garbled list on a profile is worse than none.
- English translations are copyrighted, while the classical sources are public domain. So the board
  carries **our own renderings**, which have to be checked against the source, not against memory or
  an LLM's guess.

## The sets (owner, 2026-10-19)
All in the Chinese Buddhist versions.

1. **The Five Precepts** (五戒) for lay people.
2. **The Ten Precepts of a novice** (沙彌十戒), not the ten wholesome deeds (十善業).
3. **The Upāsaka precepts** from the *Upāsakaśīla Sūtra* (優婆塞戒經): 6 major and 28 minor.
   People may take only some. The sūtra also has a lot of practical lay advice: respect for
   monastics, paying taxes, saving part of one's earnings.
4. **The Bodhisattva precepts** of the *Brahmajāla Sūtra* (梵網經): 10 major and 48 minor.

## Sources to start from
The catalogue numbers are to be **verified** as part of this detour; they're the leads, not the conclusions.

| Set | Likely primary source | Where |
|---|---|---|
| Five | 《優婆塞五戒相經》 (Taishō T1476?), plus the Āgamas | CBETA |
| Novice ten | 《沙彌十戒法并威儀》 (T1471?); 蓮池袾宏《沙彌律儀要略》 (the late-Ming manual still used in Chinese monasteries) | CBETA (Taishō; the Zhuhong text is in the 卍續藏) |
| Upāsaka | 《優婆塞戒經》 (T1488), the chapter on receiving the precepts (受戒品) | CBETA |
| Bodhisattva | 《梵網經》 (T1484), second fascicle (卷下): 10 major, 48 minor | CBETA |

Translations to **compare against and cite, not copy:** for example, the BTTS (Buddhist Text
Translation Society) Brahmajāla, Shih Heng-ching's *Sutra on Upāsaka Precepts* (BDK English
Tripiṭaka), and Dharma Drum's lay-precept materials. Record edition, translator and page for each
one consulted.

## Deliverables
- [x] `docs/precepts/` with one page per set:
  - The list, numbered as the source numbers it.
  - For each item: the classical Chinese, our plain-English rendering, and a citation (text, Taishō
    number, fascicle, CBETA location).
  - A short "how this set is taken" note: who confers it, the partial-taking customs.
  - Variant numberings or wordings between traditions and translators, and which we follow.
  - Sources consulted: primary texts, and the translations compared (cited, not quoted at length).
- [x] `docs/precepts/README.md` explaining:
  - The method: renderings are ours, LLM-drafted from the classical text and **owner-reviewed**, each
    marked `provenance: llm-draft-reviewed` with the reviewer and date.
  - The copyright position: public-domain source texts, our own wording.
  - The naturalistic framing (`spec/v2_decisions.md`): precepts as training, not commandments with
    supernatural sanction. Where a precept's wording assumes cosmology, render the ethical content
    plainly, and note the original.
- [x] `meritledger_rs/catalog/precepts.json`: the sets as data (set key, item number, major/minor,
  short title, rendering, source citation), loaded like `practices.json`, with a test that every item
  has a citation.
- [ ] The open questions below, answered by the owner.

## Questions for the owner
1. ~~For the novice ten: Zhuhong's wording, or the older Taishō text?~~ **Moot:** they agree
   word for word, apart from 床/牀 (the same character).
2. ~~Partial taking for the Five?~~ Answered 2026-10-19: people can take only some of any set.
   T1488 describes exactly that for the Five (一分 … 滿分, p1049a18–27).
3. **Renderings:** we drafted *plain modern English*, with the classical text on the page and notes
   where we interpreted or left out cosmology. Is that right, or should they be more literal?
4. **Upāsaka minor 16 (livestock, 作淨) and 17 (robe, bowl and staff):** these are the two least
   certain renderings; see their notes in `upasaka.md`.
5. **Licence:** CBETA's text, which supplies the Chinese in the docs and `quote` fields, is
   CC BY-NC-SA 4.0 (non-commercial). The repo is MIT. That's fine for household use; record the
   exception in the repo's LICENSE or README?

## Exit criteria
- [x] Every item in every set has a citation to a primary source, verified in CBETA: 175 of 175
  (`tools/precepts_check.py`, 2026-10-19).
- [ ] The owner has reviewed every rendering, and the review is recorded.
- [ ] No sentence copied from a copyrighted translation (spot-checked against the ones cited).

## Out of scope
Monastic (bhikṣu/bhikṣuṇī) prātimokṣa, Theravada precept lists, the Eight Precepts (a later candidate:
the uposatha days in Sprint 7 make them natural).

## What was found
- **The sources are confirmed**, and the leads were right:
  - T1476 (Five)
  - T1471 (novice ten), with Zhuhong's manual quoted in X60 no. 1118
  - T1488 卷3 受戒品 (Upāsaka: 6 major at p1049a28–b24, 28 minor at p1049c01–p1050b05)
  - T1484 卷下 (10 major at p1004b16–p1005a16, 48 minor at p1005a27–p1009b19)
- **The Brahmajāla's short titles** (殺戒, 不敬師友, …) aren't in the sūtra. They're Zhiyi's, from
  菩薩戒義疏 (T1811), and every one is cited.
- **The Upāsaka sūtra's practical advice is real but sits outside the 28:**
  - the four-way division of income (p1048c21–24), the passage that says to save a quarter, comes
    before the precepts;
  - paying tax (minor 21) and keeping the law (minor 22) are among the 28.
- **The Brahmajāla is probably a 5th-century Chinese composition.** The page says so plainly; it
  doesn't change its standing.
- **Brahmajāla minor 47** forbids officials registering and controlling monastics. The page notes it
  speaks to state control of Buddhism.

## Handoff
- **Data:**
  - `meritledger_rs/catalog/precepts.json`, loaded by `meritledger_rs/src/precepts.rs`
    (`precepts::sets()`, `precepts::set(key)`; message types `PreceptSet`/`Precept` in
    `messages.rs`).
  - Set keys: `five`, `novice_ten`, `upasaka`, `bodhisattva`.
  - An item is identified by (set, kind, n). Sprint 9 stores these, so never renumber.
- **Tests:**
  - `cargo test precepts` checks the counts, the numbering, the citation format and that the docs
    match the catalog.
  - `uv run --no-project python tools/precepts_check.py` checks every citation against CBETA
    (it needs the network).
- **When the owner reviews a set:**
  - Fix its renderings in both the JSON and the page.
  - Set its `provenance` to `llm-draft-reviewed`.
  - Fill in the review log in `docs/precepts/README.md`.
- **Sprint 9 can start before the review.** The UI should show "draft" for a set whose provenance
  is still `llm-draft`.
