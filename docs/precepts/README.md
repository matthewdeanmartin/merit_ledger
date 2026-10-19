# Precepts

The precept sets people can record on their profile (Sprint 9), in their Chinese Buddhist versions.
Each item traces back, line by line, to the classical source.

| Set | Source | Items | Page |
|---|---|---|---|
| The Five Precepts 五戒 | 佛說優婆塞五戒相經, T1476 | 5 | [five.md](five.md) |
| The Ten Novice Precepts 沙彌十戒 | 沙彌十戒法并威儀, T1471; Zhuhong's 沙彌律儀要略 (X1118) | 10 | [novice-ten.md](novice-ten.md) |
| The Upāsaka Precepts 優婆塞戒 | 優婆塞戒經 受戒品, T1488 | 6 major, 28 minor | [upasaka.md](upasaka.md) |
| The Bodhisattva Precepts 菩薩戒 | 梵網經 卷下, T1484; titles from 菩薩戒義疏, T1811 | 10 major, 48 minor | [bodhisattva.md](bodhisattva.md) |

Anyone may take only some of a set. The *Upāsakaśīla Sūtra* describes exactly that for the
Five (see [five.md](five.md)), and the board allows it for every set.

## Where the texts come from

All texts are from **CBETA**, the Chinese Buddhist Electronic Text Association in Taipei, via
CBETA Online and its data API (cbdata.dila.edu.tw, hosted by Dharma Drum's DILA). By the owner's
direction, Taiwanese sources come first. A citation such as `T24n1484_p1004b16` means Taishō
vol. 24, text no. 1484, page 1004, column b, line 16. Every citation in these pages links to that
line on CBETA Online, and the `X` ones are in the 卍新纂續藏經.

## How the renderings were made

- **The English is ours.** Each rendering was drafted by an LLM working from the classical Chinese
  shown on the page, not from memory and not from any published translation. It is then reviewed by the
  owner.
- **Provenance** is recorded per set in `meritledger_rs/catalog/precepts.json`:
  - `llm-draft` means not yet reviewed.
  - `llm-draft-reviewed` means the owner has reviewed it, and the reviewer and date go in the
    review log below.
- **Plain, not literal.** A rendering says what the precept asks of a person today, in plain
  English. Where we've left something out or interpreted, the item's note says so, and the full
  Chinese is on the page.
- **Naturalistic.** Following `spec/v2_decisions.md`, precepts are training rules people choose
  to keep, not commandments backed by supernatural sanction. Where a precept's wording depends on
  its cosmology (gods, ghosts, rebirth, auspicious visions), we render its ethical content and say
  so in a note. We don't rewrite what the precept forbids or requires.
- **The short Chinese titles** of the Brahmajāla precepts are Zhiyi's (T1811), each cited. The other
  sets' titles are the precept's own words.
- **One rendering, two places.** The board shows `precepts.json`, and these pages show the same text
  beside the Chinese, for review. If you change a rendering, change it in both places:
  `cargo test` fails if they differ. Set keys and numbers are stored in people's records, so
  never renumber.

## How the citations are checked

```sh
uv run --no-project python tools/precepts_check.py   # needs the network
```

For every item, this fetches the cited lines from CBETA. It checks that the item's `quote` appears
at its first citation, and that its Chinese title appears at its other citations and its
title citation. Punctuation is ignored, and 床/牀 count as the same character. On 2026-10-19,
175 of 175 citations checked out. `cargo test` checks the shape offline: the counts, the
numbering, and that every item has a well-formed citation.

## Copyright

- **The classical texts** are many centuries old.
- **CBETA's digital edition** of them (its punctuation, collation and markup, including the
  Taishō and the 卍續藏) is released under **Creative Commons BY-NC-SA 4.0**, non-commercial use
  only ([CBETA 版權宣告](https://cbeta.org/copyright)). The Chinese quoted in these pages and the
  `quote` fields in `precepts.json` come from it. Text version: CBETA, 2026-08 release.
- **Our English renderings** are original. No published translation was copied. Those consulted
  for comparison are listed below, as cited, not quoted.

The repository is MIT, so the Chinese quotations are the exception: they keep CBETA's licence,
which Merit Ledger's non-commercial household use fits. Anyone who wants to sell Merit Ledger
would need to take the Chinese out, or ask CBETA.

## Translations to compare against

Spot-check the renderings against these, but don't copy them:

- *The Brahma Net Sutra*, Buddhist Text Translation Society (BTTS).
- Bhikṣuṇī Shih Heng-ching, *The Sutra on Upāsaka Precepts*, BDK English Tripiṭaka (Numata Center, 1994).
- Dharma Drum Mountain's lay-precept materials.

_(Record the edition and page for each one actually consulted.)_

## Review log

| Set | Reviewed by | Date | Notes |
|---|---|---|---|
| five | | | |
| novice_ten | | | |
| upasaka | | | |
| bodhisattva | | | |
