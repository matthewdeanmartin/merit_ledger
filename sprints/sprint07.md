# Sprint 7 — Meritledger lunar calendars
**State:** done 2026-10-19 (desktop and board). The owner reviews the holidays in place.
**Depends on:** 0 (pure logic); 2 for the settings and UI   **Parallel with:** 3–6

## Goal
Add our own computed Buddhist calendars, in Theravada-inspired, Chinese-inspired and
Japanese-inspired versions, with an option to move observances to the closest Sunday. Add iCal
feeds and feast-day notices.

## Read first
- `../spec/domain.md` § Calendar
- `../spec/v2_decisions.md` § Calendars
- Jean Meeus, *Astronomical Algorithms*, ch. 49 (phases of the moon) and ch. 25/27 (solar longitude,
  equinoxes)
- `../microcontroller/gamematcher/api/src/ics.rs` (RFC 5545 by hand, already tested)

## Already decided
- These are "meritledger lunar calendars". **We don't track any official body's rulings.** Our
  algorithm is the definition, and golden-file tests pin it.
- Dates are computed in the household time zone (the Sprint 3 setting). Use `meritledger_rs/src/tz.rs`:
  `Tz::local_date`, plus `tz::Date` (`days`, `weekday`, `add_days`). It's already tested for DST in
  both hemispheres.
- **Sunday shift = closest Sunday.** There are no ties, because the offset is always -3..+3.
- Feast days only give notice: "Today is a feast day".
- Pure Rust, no I/O, `no_std`-friendly maths (f64 is fine on the S3's FPU).

## Answered (2026-10-18)
- **Feast days: one a month,** assigned arbitrarily and spaced out as evenly as possible. Build this as a
  per-system rule: choose one observance per lunar or Gregorian month to be "the feast", preferring
  the major festival of that month, and otherwise the observance closest to mid-month. Then check that
  consecutive feasts are about 3–5 weeks apart.
- **Holidays:** ship the proposed lists below. The owner will review them **in place**, on the
  calendar page, rather than on paper.

## Questions for the owner
The proposed observance lists (to ship, then review in place):

1. **Theravada-inspired:**
   - Uposatha on full and new moons, with quarter moons optional.
   - Festivals:
     - Magha Puja: full moon of the 3rd lunar month, approximated as the full moon in Feb–Mar.
     - Vesak: full moon of May (or the 2nd full moon if April has two).
     - Asalha Puja: full moon of July.
     - Vassa: starts the day after Asalha and ends at the October full moon (Pavarana).
     - Kathina: the month after Pavarana.
2. **Chinese-inspired:**
   - Months start at the new moon. Leap months follow the "no principal solar term" rule.
   - Fasting days: the 6-day set (8, 14, 15, 23, 29, 30; 28 and 29 when a month is short) or the 10-day set.
   - Festivals: Lunar New Year (1/1), Lantern (1/15), Buddha's Birthday (4/8), Ullambana (7/15),
     Bodhi Day / Laba (12/8).
   - Guanyin days (2/19, 6/19, 9/19) are the **open question**: keep them as feast days (cultural or
     symbolic), or drop them?
3. **Japanese-inspired** (Gregorian, fixed):
   - Shushō-e (Jan 1), Nehan-e (Feb 15), Higan (spring and autumn equinox weeks), Hanamatsuri (Apr 8)
   - Obon (Aug 13–16; or July 13–16?), Jōdō-e / Rōhatsu (Dec 8), Joya no Kane (Dec 31)
   - Nichiren-related days, if wanted: Rikkyō Kaishū (Apr 28), Oeshiki (Oct 13).
4. ~~Which observances are feast days?~~ Answered: one a month, as above.

## What was built
- **Astronomy** (`astro.rs`), after Meeus:
  - Moon phases (ch. 49, with the planetary terms) and the sun's longitude (ch. 25).
  - Tested within a few minutes of the published instants: the 2000, 2023, 2024 and 2025
    phases, the 2024 equinox and the 2025 solstice.
- **The three calendars** (`calendar.rs`). The rules are in its module docs:
  - **Chinese-inspired:** months on the household-local new-moon day, and a leap month by the
    "no principal term" rule (2028's leap 5th month comes out right).
    - Six (or ten) fasting days a month.
    - New Year, Lantern, Buddha's Birthday, Ullambana, Laba, and the three Guanyin days.
    - No Pure Land days.
  - **Theravada-inspired:** uposatha on new and full moons (quarters optional).
    - Māgha Pūjā on the first full moon on or after 10 February.
    - Vesak 3 full moons later, Āsāḷha 2 after that, Pavāraṇā 3 after that.
    - Vassa and Kaṭhina begin the day after Āsāḷha and Pavāraṇā respectively.
    - 2025 and 2026 come out the same as Thailand's dates, without promising to.
  - **Japanese-inspired:** Shushō-e, Nehan-e, the Higan weeks (equinox ± 3), Hanamatsuri,
    Rikkyō Kaishū, Obon (August), Oeshiki, Rōhatsu, and Joya no Kane.
- **Feasts:** one per Gregorian month.
  - A **major** festival wins (Vesak, New Year, Buddha's Birthday, Hanamatsuri, Ullambana and their
    kin), then other festivals, then the observance nearest the 15th.
  - Japanese months with nothing use the new or full moon nearest mid-month.
  - Tested: 60 feasts in 5 years for each system, 5–57 days apart, and the major ones always win.
- **Sunday shift:** to the closest Sunday (Wednesday goes back, Thursday forward), keeping
  `shifted_from`.
- **Golden files:** `tests/golden/{system}-{year}.txt` for 2026–2030 (New York, default options). A
  change must be regenerated on purpose: `UPDATE_GOLDEN=1 cargo test --test calendar_golden`.
- **Settings:**
  - The household calendar defaults to Chinese-inspired. Sunday shift, quarter moons and ten fasting
    days are options.
  - Each person can choose their own calendar.
  - The **time zone is picked by place** ("New York (Eastern)", or "Use this phone's"), with the
    POSIX rule only under "Advanced" (owner feedback: the typed rule was unusable). The places
    are in `zones.rs`, and a test checks every rule parses and gives the right offset.
- **API and feeds:**
  - `GET /api/v1/calendar?from&to[&system]` (public, up to 120 days).
  - `GET /api/v1/timezones`.
  - Today's observances in `TodayView`.
  - `GET /cal/{chinese|theravada|japanese}.ics`: public iCalendar, two months back to 400 days
    ahead, with the household's options.
- **UI:**
  - Calendar page: a month grid with the moon phase, the lunar day, "4/1" at a lunar month's
    start, and observances, feasts tinted.
  - Below the grid: the month's list and a Subscribe link.
  - A "Today: …" banner on Today.
  - Calendar settings on the Household page, "Your calendar" on settings.

## Exit criteria
- [x] Astronomy within 5 minutes of published phase times (known instants from 2000 to 2025).
- [x] Golden files stable; the owner reviews them, and the running calendar, in place.
- [x] Sunday shift: every shifted date is a Sunday within 3 days (all systems, 2026–2027), plus an
  API test.
- [ ] **A full year of all three systems in under 50 ms on the S3: not met cold, met warm.**
  - The first computation takes **0.60 s**, because the S3's FPU is single-precision and the f64
    astronomy is emulated.
  - Years are cached (**1.5 ms** for a cached year), and housekeeping warms last, this and next year
    for every calendar in use once the clock is set (0.29 s, a minute after boot).
  - Feeds load in 0.13–0.19 s; they took 1.7–8.9 s before the cache and the shorter range.

## Questions for the owner (answered 2026-10-19)
1. **Household-local dates stay.** The Chinese-inspired calendar may be a day off from China's
   published dates (Buddha's Birthday 2026: 23 May here, 24 May in China); that's accepted.
2. **Guanyin days stay**, as cultural and symbolic observances.
3. **The disclaimer is written down** in [docs/CALENDARS.md](../docs/CALENDARS.md): no attempt to match official
   bodies, the international date line or traditional calculation methods.

## Handoff
- The calendars are pure functions of (system, options, time zone, date range), cached by year.
  Sprint 10's reminders and Sprint 13's screen should call `calendar::observances` and share the cache.
- The months of the lunisolar calendar (`calendar::chinese_months`) are cached too. `calendar_view`
  uses them for every system, to show the lunar day.
- If calendar speed matters again (another board, or cold boots), precompute moon phases and solar
  terms at build time (`build.rs` can run `astro.rs` on the PC) into a table for 2020–2060.
