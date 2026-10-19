# About the dates

Merit Ledger's calendars are **"meritledger lunar calendars"**: our own, computed by the board from
astronomy and the household's time zone. They are meant to be good enough for a household's practice,
not authoritative.

## What we don't attempt

Buddhist calendars are genuinely complicated, and the authorities disagree with each other. We make
**no attempt** to:

- **Match any official body.** National sanghas, temple associations and governments set dates by their
  own rules and committees, and sometimes change them by announcement. Thailand's Mahanikaya and
  Dhammayut orders don't always agree; Myanmar, Sri Lanka and Cambodia each have their own
  reckoning; Chinese temples follow the Chinese government's calendar. We don't follow any of
  them, and we won't chase their corrections.
- **Get the international date line "right".** A lunar month starts on the day of the new moon
  *where you are*. The same new moon can fall on different calendar days in New York and in Beijing,
  so our Chinese-inspired dates are sometimes a day earlier (or later) than the ones Chinese temples
  publish. For example, Buddha's Birthday 2026 is 23 May here and 24 May in China. That is expected.
- **Reproduce traditional calculation methods.** Real Chinese calendars use their own astronomical
  conventions (Beijing time, specific ephemerides). Theravada calendars use various traditional
  arithmetic (some Thai, some Burmese, some Sri Lankan) rather than modern astronomy.
- **Follow regional customs**, such as Obon in July versus August, local festivals, or a temple's own
  anniversaries.

## What we do

- **Moon phases** come from modern astronomy (Jean Meeus' algorithms), accurate to within a minute
  or two.
- **The calendar day** of an event is the day it happens in the **household's time zone** (set on the
  Household page).
- **The rules for each calendar** are written down in `meritledger_rs/src/calendar.rs`, and
  **golden files** (`meritledger_rs/tests/golden/`) pin exactly what they produce for 2026–2030, so
  the dates only change when someone changes the rules on purpose.
- **Guanyin days** are included in the Chinese-inspired calendar as cultural and symbolic observances:
  Guanyin as compassion, not as a being who intercedes.

## If a date looks wrong

Check it against the rule in `calendar.rs` before anything else. If our rule gives a date you don't
want, change the rule (and regenerate the golden files):

```sh
UPDATE_GOLDEN=1 cargo test --test calendar_golden
```

If you want a specific temple's or country's date, that's a different calendar, and out of scope.
Mark it as a household event instead (Sprint 10).
