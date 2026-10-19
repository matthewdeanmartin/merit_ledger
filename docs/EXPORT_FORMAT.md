# Export format (version 1)

Merit Ledger's export, backup and import files are all one JSON document. The board produces it
with its wire encoder, from `ExportDoc` in `meritledger_rs/src/messages.rs`.

**Where it comes from:**
- `GET /api/v1/admin/export.json` (admin), or "Download everything" on the Household page.
- The nightly backups `ml/backup/YYYY-MM-DD.json` on the SD card.

**Where it goes:** `POST /api/v1/admin/import[?dry_run=1]` (admin), or "Import a Merit Ledger file".

```json
{
  "format": "meritledger-export",
  "version": 1,
  "exported_at": 1792339200,
  "timezone": "EST5EDT,M3.2.0,M11.1.0",
  "people": [{ "id": 1, "display_name": "matt", "role": "admin" }],
  "kinds": [{ "key": "c1", "name": "Tea ceremony", "icon": "🍵", "polarity": "merit",
              "category": "ritual", "unit": "minutes", "default_amount": 15,
              "visibility": "public", "content": true, "owner": 0, "archived": false }],
  "entries": [{ "id": 1, "person": 1, "kind": "meditation", "date": "2026-10-18",
                "created": 1792339200, "amount": 20, "note": "", "visibility": "public",
                "source": "tap", "recorded_by": 1, "rev": 1, "rejoiced_by": [] }],
  "archived_days": [{ "person": 1, "day": 20700, "own_entries": 2, "own_minutes": 40,
                      "open_entries": 2, "open_minutes": 40, "day_only": 0, "demerits": 0 }]
}
```

## Fields

- **`people`** has names and roles only. **No PINs, no email addresses.** People created by an import
  need a PIN from an admin before they can sign in.
- **`kinds`** has custom practice kinds only. Built-in kinds are referred to by their stable keys
  (`meditation`, `daimoku` …, from `catalog/practices.json`).
- **`entries`** has every entry the board has, private ones included (an export is the admin's
  copy of the household's data). Fields as in `spec/domain.md` § Practice:
  - `date`: household-local `YYYY-MM-DD`.
  - `created`: UTC seconds, 0 if recorded before the clock was set.
  - `amount`: minutes or a count, 0 for `once` kinds.
  - `recorded_by`: 0 means the board's touch menu.
- **`archived_days`** holds daily totals of entries the board no longer has in detail. They aged out
  while there was no SD card. `day` is days since 1970-01-01. The split between `own_*` (what the
  person sees) and `open_*`/`day_only` (what others see) is explained on `stats::Tally`.
- **`vows`** (Sprint 9) has every vow, with its `events` timeline: `took`, `pause`, `resume`, `breach`,
  `repair`, `complete` and `retire`, each with a date, a note and a visibility. A vow made from a
  precept carries `precept_set`, `precept_kind` and `precept_n`.
- **`posts`** (Sprint 9) has the vow blog posts **with their markdown bodies**, whether the board kept
  them on the SD card or not.
- **`texts`** (Sprint 8) has the household's own texts with their markdown bodies. Built-in texts
  ship with the board's pages and aren't exported.
- **`practices`** (Sprint 8) has the built practices. A step refers to a built-in text by its id
  (`lotus-2`) or to a household text as `h<id>`.
- **`precepts`** (Sprint 9) records which precepts each person has taken: set key, date, note, and
  the `majors` and `minors` item numbers.

## Import rules

- **People** are matched by name, ignoring case. Unknown names are added as members without a PIN
  (up to the board's 16).
- **Custom kinds** are matched by name and owner, and added if missing.
- **An entry already on the board is skipped.** "Already" means same person, practice, day,
  `created`, amount and note. Importing the same file twice changes nothing.
- **Imported entries get new ids.** Who recorded them and who rejoiced in them are mapped through
  the people match.
- **Without an SD card,** imported entries older than the 60-day window become daily totals only.
  The report counts them as "kept as daily totals".
- **Vows** are matched by person, text and start date, and added if missing. **Posts** are matched
  by vow, title and date. A repair keeps its timeline event, but its link to the repentance entry
  is dropped (`entry` 0). That entry itself comes across with the other entries.
- **Precepts:** a set someone already has on the board is left as it is.
- **Texts** are matched by title and **practices** by owner and title. An imported practice's steps
  are pointed at the imported texts' new ids and custom kinds' new keys.
- **Imported post bodies** go to the SD card when there is one. Without one, they count against
  the snapshot's post quota (16 KB).
- **`?dry_run=1`** reports what would happen and changes nothing.

## Compatibility

Fields may be added in later versions (the wire format ignores unknown fields). A file from a
newer `version` than the board understands is refused. Field meanings never change within a version.
