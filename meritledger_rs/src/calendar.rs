//! The meritledger lunar calendars (`sprints/sprint07.md`): our own
//! computed Buddhist calendars, in three flavours, from `astro.rs` and the
//! household's time zone. They follow this code, not any official body's
//! rulings, and golden files (`tests/golden/`) pin what it produces.
//!
//! - **Theravada-inspired:** uposatha on new and full moons (quarter moons
//!   optional). Māgha Pūjā on the first full moon on or after 10 February;
//!   Vesak three full moons later, Āsāḷha two after that, Pavāraṇā three
//!   after that (the end of the rains retreat, which begins the day after
//!   Āsāḷha); the Kaṭhina season begins the day after Pavāraṇā.
//! - **Chinese-inspired:** lunisolar months starting on the household-local
//!   day of the new moon, with a leap month by the "no principal term"
//!   rule; the six (or ten) monthly fasting days; New Year, Lantern,
//!   Buddha's Birthday (4/8), Guanyin days, Ullambana (7/15), Laba (12/8).
//! - **Japanese-inspired:** fixed Gregorian dates, Higan as the equinox
//!   weeks, and Nichiren's founding day and Oeshiki.
//!
//! One observance a month is the **feast**: a festival if the month has
//! one (the one nearest mid-month), else the observance nearest the 15th
//! (for the Japanese calendar, the new or full moon nearest mid-month). With **Sunday shift** every
//! observance moves to the closest Sunday and keeps its true date.
use crate::astro::{self, Phase};
use crate::messages::{CalendarDay, CalendarView, ObservanceView};
use crate::tz::{Date, Tz};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum System {
    Theravada,
    Chinese,
    Japanese,
    None,
}

impl System {
    pub fn parse(s: &str) -> Option<System> {
        match s {
            "theravada" => Some(System::Theravada),
            "chinese" => Some(System::Chinese),
            "japanese" => Some(System::Japanese),
            "none" => Some(System::None),
            _ => None,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            System::Theravada => "theravada",
            System::Chinese => "chinese",
            System::Japanese => "japanese",
            System::None => "none",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Options {
    pub sunday_shift: bool,
    /// Theravada: uposatha on quarter moons too.
    pub quarter_moons: bool,
    /// Chinese: the ten fasting days instead of six.
    pub ten_fasting_days: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observance {
    pub date: Date,
    /// `uposatha`, `fasting`, `festival` or `moon`.
    pub kind: &'static str,
    pub name: String,
    pub feast: bool,
    /// The true date, when Sunday shift moved it.
    pub shifted_from: Option<Date>,
}

/// One lunisolar month: starts on `start`, numbered 1–12, maybe a leap month.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LunarMonth {
    pub start: Date,
    pub number: u32,
    pub leap: bool,
    pub days: u32,
}

fn day_start(d: Date) -> i64 {
    d.days() * 86_400
}

/// Household-local dates of every moon phase from `from` to `to` (inclusive).
pub fn moon_days(tz: &Tz, from: Date, to: Date) -> Vec<(Date, Phase)> {
    // A day of margin each side: local dates differ from UTC ones.
    astro::phases_between(day_start(from) - 86_400, day_start(to) + 2 * 86_400)
        .into_iter()
        .map(|(t, p)| (tz.local_date(t), p))
        .filter(|(d, _)| *d >= from && *d <= to)
        .collect()
}

/// Local date of the December solstice of `year`.
fn december_solstice(tz: &Tz, year: i32) -> Date {
    let nov = Date {
        year,
        month: 11,
        day: 1,
    };
    tz.local_date(astro::sun_reaches(270.0, day_start(nov)))
}

/// A small cache of whole years: the calendars never change for a given
/// year and settings, and the board's FPU is single precision (the f64
/// astronomy is emulated), so each year is computed once.
struct YearCache<K, V> {
    map: Mutex<HashMap<K, Arc<V>>>,
}

impl<K: Eq + Hash + Clone, V> YearCache<K, V> {
    const LIMIT: usize = 48;

    fn new() -> Self {
        YearCache {
            map: Mutex::new(HashMap::new()),
        }
    }

    fn get(&self, key: K, compute: impl FnOnce() -> V) -> Arc<V> {
        if let Some(v) = self.map.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
            return v.clone();
        }
        let v = Arc::new(compute());
        let mut map = self.map.lock().unwrap_or_else(|e| e.into_inner());
        if map.len() >= Self::LIMIT {
            map.clear();
        }
        map.insert(key, v.clone());
        v
    }
}

/// Observances of whole years, by system, options, zone and year.
type ObservanceCache = YearCache<(System, Options, Tz, i32), Vec<Observance>>;

/// The lunisolar months that start in `year` (cached).
fn months_of_year(tz: &Tz, year: i32) -> Arc<Vec<LunarMonth>> {
    static CACHE: OnceLock<YearCache<(Tz, i32), Vec<LunarMonth>>> = OnceLock::new();
    CACHE
        .get_or_init(YearCache::new)
        .get((tz.clone(), year), || {
            let (from, to) = (
                Date {
                    year,
                    month: 1,
                    day: 1,
                },
                Date {
                    year,
                    month: 12,
                    day: 31,
                },
            );
            compute_chinese_months(tz, from, to)
                .into_iter()
                .filter(|m| m.start.year == year)
                .collect()
        })
}

/// The lunisolar months overlapping `from..=to`.
pub fn chinese_months(tz: &Tz, from: Date, to: Date) -> Vec<LunarMonth> {
    let mut out: Vec<LunarMonth> = (from.year - 1..=to.year)
        .flat_map(|y| months_of_year(tz, y).as_ref().clone())
        .collect();
    out.retain(|m| m.start.add_days(i64::from(m.days) - 1) >= from && m.start <= to);
    out
}

fn compute_chinese_months(tz: &Tz, from: Date, to: Date) -> Vec<LunarMonth> {
    let first = from.year - 1;
    let last = to.year + 1;
    let span_from = december_solstice(tz, first - 1).add_days(-40);
    let span_to = december_solstice(tz, last).add_days(40);
    let new_moons: Vec<Date> = moon_days(tz, span_from, span_to)
        .into_iter()
        .filter(|(_, p)| *p == Phase::New)
        .map(|(d, _)| d)
        .collect();
    // Principal terms: the sun at multiples of 30°. Stepped by index, each
    // search starting days after the last: searching from the exact instant
    // of a term can find that same term again a year later (rounding).
    let mut terms = Vec::new();
    let mut k = (astro::sun_longitude(day_start(span_from)) / 30.0).ceil() as i64;
    let mut t = day_start(span_from);
    while t < day_start(span_to) {
        t = astro::sun_reaches(((k * 30) % 360) as f64, t + 5 * 86_400);
        terms.push(tz.local_date(t));
        k += 1;
    }
    let month11 = |solstice: Date| {
        new_moons
            .iter()
            .rposition(|d| *d <= solstice)
            .expect("a new moon before the solstice")
    };
    let mut out = Vec::new();
    for year in first - 1..last {
        let a = month11(december_solstice(tz, year));
        let b = month11(december_solstice(tz, year + 1));
        let leap_year = b - a == 13;
        let mut number = 11;
        let mut leap_used = false;
        for i in a..b {
            let (start, end) = (new_moons[i], new_moons[i + 1]);
            let has_term = terms.iter().any(|d| *d >= start && *d < end);
            let leap = leap_year && !leap_used && i > a && !has_term;
            if i > a && !leap {
                number = number % 12 + 1;
            }
            leap_used |= leap;
            out.push(LunarMonth {
                start,
                number,
                leap,
                days: (end.days() - start.days()) as u32,
            });
        }
    }
    out.retain(|m| m.start.add_days(i64::from(m.days) - 1) >= from && m.start <= to);
    out.dedup_by_key(|m| m.start);
    out
}

/// Chinese-calendar day of `date`: (month, leap, day of month).
pub fn chinese_day(months: &[LunarMonth], date: Date) -> Option<(u32, bool, u32)> {
    months
        .iter()
        .find(|m| date >= m.start && date.days() < m.start.days() + i64::from(m.days))
        .map(|m| (m.number, m.leap, (date.days() - m.start.days()) as u32 + 1))
}

fn obs(date: Date, kind: &'static str, name: impl Into<String>) -> Observance {
    Observance {
        date,
        kind,
        name: name.into(),
        feast: false,
        shifted_from: None,
    }
}

fn theravada(tz: &Tz, options: Options, from: Date, to: Date) -> Vec<Observance> {
    let mut out = Vec::new();
    let moons = moon_days(tz, from.add_days(-400), to.add_days(400));
    for (d, p) in &moons {
        let name = match p {
            Phase::New => "Uposatha (new moon)",
            Phase::Full => "Uposatha (full moon)",
            _ if options.quarter_moons => "Uposatha (quarter moon)",
            _ => continue,
        };
        out.push(obs(*d, "uposatha", name));
    }
    let fulls: Vec<Date> = moons
        .iter()
        .filter(|(_, p)| *p == Phase::Full)
        .map(|(d, _)| *d)
        .collect();
    for year in from.year - 1..=to.year + 1 {
        let start = Date {
            year,
            month: 2,
            day: 10,
        };
        let Some(magha) = fulls.iter().position(|d| *d >= start) else {
            continue;
        };
        let at = |n: usize| fulls.get(magha + n).copied();
        let named = [
            (Some(fulls[magha]), "Māgha Pūjā"),
            (at(3), "Vesak"),
            (at(5), "Āsāḷha Pūjā"),
            (at(5).map(|d| d.add_days(1)), "Vassa (rains retreat) begins"),
            (at(8), "Pavāraṇā: the rains retreat ends"),
            (at(8).map(|d| d.add_days(1)), "Kaṭhina season begins"),
        ];
        for (d, name) in named {
            if let Some(d) = d {
                out.push(obs(d, "festival", name));
            }
        }
    }
    out
}

fn chinese(tz: &Tz, options: Options, from: Date, to: Date) -> Vec<Observance> {
    let mut out = Vec::new();
    let months = chinese_months(tz, from.add_days(-60), to.add_days(60));
    for m in &months {
        let short = m.days < 30;
        let mut days: Vec<u32> = if options.ten_fasting_days {
            vec![1, 8, 14, 15, 18, 23, 24, 28, 29, 30]
        } else {
            vec![8, 14, 15, 23, 29, 30]
        };
        if short {
            // A 29-day month moves its last fasting days back by one.
            let tail = if options.ten_fasting_days { 3 } else { 2 };
            let n = days.len();
            for d in &mut days[n - tail..] {
                *d -= 1;
            }
        }
        for d in days {
            out.push(obs(
                m.start.add_days(i64::from(d) - 1),
                "fasting",
                format!("Fasting day (lunar {}/{})", m.number, d),
            ));
        }
        if m.leap {
            continue;
        }
        let festivals: &[(u32, u32, &str)] = &[
            (1, 1, "Lunar New Year"),
            (1, 15, "Lantern Festival"),
            (2, 19, "Guanyin's day (birth)"),
            (4, 8, "Buddha's Birthday"),
            (6, 19, "Guanyin's day (awakening)"),
            (7, 15, "Ullambana"),
            (9, 19, "Guanyin's day (renunciation)"),
            (12, 8, "Laba: the Buddha's awakening"),
        ];
        for (month, day, name) in festivals {
            if *month == m.number && *day <= m.days {
                out.push(obs(
                    m.start.add_days(i64::from(*day) - 1),
                    "festival",
                    *name,
                ));
            }
        }
    }
    out
}

fn japanese(tz: &Tz, from: Date, to: Date) -> Vec<Observance> {
    let mut out = Vec::new();
    for year in from.year - 1..=to.year + 1 {
        let d = |month, day| Date { year, month, day };
        let fixed = [
            (d(1, 1), "Shushō-e (New Year service)"),
            (d(2, 15), "Nehan-e (the Buddha's parinirvāṇa)"),
            (d(4, 8), "Hanamatsuri (the Buddha's birthday)"),
            (d(4, 28), "Rikkyō Kaishū (Nichiren's founding)"),
            (d(8, 13), "Obon (to the 16th)"),
            (d(10, 13), "Oeshiki (Nichiren's passing)"),
            (d(12, 8), "Rōhatsu (the Buddha's awakening)"),
            (d(12, 31), "Joya no Kane (New Year's Eve bells)"),
        ];
        for (date, name) in fixed {
            out.push(obs(date, "festival", name));
        }
        for (longitude, month, name) in [(0.0, 3, "Spring Higan"), (180.0, 9, "Autumn Higan")] {
            let equinox = tz.local_date(astro::sun_reaches(longitude, day_start(d(month, 1))));
            out.push(obs(
                equinox.add_days(-3),
                "festival",
                format!("{name} begins"),
            ));
            out.push(obs(equinox, "festival", format!("{name} (equinox)")));
        }
    }
    out
}

/// The great days of each calendar: in a month with one, it is the feast.
fn is_major(name: &str) -> bool {
    const MAJOR: [&str; 12] = [
        "Māgha Pūjā",
        "Vesak",
        "Āsāḷha Pūjā",
        "Pavāraṇā",
        "Lunar New Year",
        "Buddha's Birthday",
        "Ullambana",
        "Laba",
        "Nehan-e",
        "Hanamatsuri",
        "Obon",
        "Rōhatsu",
    ];
    MAJOR.iter().any(|m| name.starts_with(m))
}

/// Marks one feast in each Gregorian month of `list` (see the module docs).
fn choose_feasts(tz: &Tz, system: System, list: &mut Vec<Observance>, from: Date, to: Date) {
    let mut month = Date {
        year: from.year,
        month: from.month,
        day: 1,
    };
    while month <= to {
        let next = if month.month == 12 {
            Date {
                year: month.year + 1,
                month: 1,
                day: 1,
            }
        } else {
            Date {
                month: month.month + 1,
                ..month
            }
        };
        let score = |o: &Observance| {
            let mid = (o.date.day as i64 - 15).abs();
            match (o.kind, is_major(&o.name)) {
                (_, true) => mid,
                ("festival", false) => 50 + mid,
                _ => 100 + mid,
            }
        };
        let best = list
            .iter()
            .enumerate()
            .filter(|(_, o)| o.date >= month && o.date < next)
            .min_by_key(|(_, o)| (score(o), o.date))
            .map(|(i, _)| i);
        match best {
            Some(i) => list[i].feast = true,
            None if system == System::Japanese => {
                // No Japanese observance this month: the new or full moon
                // nearest mid-month is the feast.
                if let Some((d, p)) = moon_days(tz, month, next.add_days(-1))
                    .into_iter()
                    .filter(|(_, p)| matches!(p, Phase::New | Phase::Full))
                    .min_by_key(|(d, _)| (d.day as i64 - 15).abs())
                {
                    list.push(Observance {
                        feast: true,
                        ..obs(d, "moon", p.name())
                    });
                }
            }
            None => {}
        }
        month = next;
    }
}

/// The closest Sunday (0..=3 days back, else forward).
pub fn closest_sunday(d: Date) -> Date {
    let w = i64::from(d.weekday());
    if w <= 3 {
        d.add_days(-w)
    } else {
        d.add_days(7 - w)
    }
}

/// Every observance of `system` dated `from..=to`, oldest first. Whole
/// years are computed once and cached.
pub fn observances(
    system: System,
    options: Options,
    tz: &Tz,
    from: Date,
    to: Date,
) -> Vec<Observance> {
    static CACHE: OnceLock<ObservanceCache> = OnceLock::new();
    let cache = CACHE.get_or_init(YearCache::new);
    let mut out = Vec::new();
    for year in from.year..=to.year {
        let list = cache.get((system, options, tz.clone(), year), || {
            compute(
                system,
                options,
                tz,
                Date {
                    year,
                    month: 1,
                    day: 1,
                },
                Date {
                    year,
                    month: 12,
                    day: 31,
                },
            )
        });
        out.extend(
            list.iter()
                .filter(|o| o.date >= from && o.date <= to)
                .cloned(),
        );
    }
    out
}

fn compute(system: System, options: Options, tz: &Tz, from: Date, to: Date) -> Vec<Observance> {
    // Whole months a little either side, so feasts and Sunday shifts near
    // the edges come out the same whatever range is asked for.
    let wide_from = Date {
        day: 1,
        ..from.add_days(-31)
    };
    let wide_to = to.add_days(31);
    let mut list = match system {
        System::Theravada => theravada(tz, options, wide_from, wide_to),
        System::Chinese => chinese(tz, options, wide_from, wide_to),
        System::Japanese => japanese(tz, wide_from, wide_to),
        System::None => return Vec::new(),
    };
    list.retain(|o| o.date >= wide_from && o.date <= wide_to);
    choose_feasts(tz, system, &mut list, wide_from, wide_to);
    if options.sunday_shift {
        for o in &mut list {
            let sunday = closest_sunday(o.date);
            if sunday != o.date {
                o.shifted_from = Some(o.date);
                o.date = sunday;
            }
        }
    }
    list.retain(|o| o.date >= from && o.date <= to);
    list.sort_by(|a, b| {
        a.date
            .cmp(&b.date)
            .then(b.feast.cmp(&a.feast))
            .then(a.name.cmp(&b.name))
    });
    list.dedup_by(|a, b| a.date == b.date && a.name == b.name);
    list
}

/// One line per observance, for golden files and logs.
pub fn describe(list: &[Observance]) -> String {
    let mut out = String::new();
    for o in list {
        out.push_str(&format!("{} {:<9} {}", o.date.iso(), o.kind, o.name));
        if o.feast {
            out.push_str("  ✦ feast");
        }
        if let Some(d) = o.shifted_from {
            out.push_str(&format!("  (from {})", d.iso()));
        }
        out.push('\n');
    }
    out
}

/// An observance as the API shows it.
pub fn view(o: &Observance) -> ObservanceView {
    ObservanceView {
        date: o.date.iso(),
        kind: o.kind.into(),
        name: o.name.clone(),
        feast: o.feast,
        shifted_from: o.shifted_from.map(|d| d.iso()).unwrap_or_default(),
    }
}

/// The calendar page's data for `from..=to` (at most a few months).
pub fn calendar_view(
    system: System,
    options: Options,
    tz: &Tz,
    from: Date,
    to: Date,
) -> CalendarView {
    let months = chinese_months(tz, from, to);
    let phases: Vec<(Date, Phase)> = moon_days(tz, from, to);
    let mut days = Vec::new();
    let mut d = from;
    while d <= to {
        let (lunar_month, lunar_day) = match chinese_day(&months, d) {
            Some((number, leap, 1)) => (
                if leap {
                    format!("leap {number}")
                } else {
                    number.to_string()
                },
                1,
            ),
            Some((_, _, day)) => (String::new(), day),
            None => (String::new(), 0),
        };
        let phase = phases
            .iter()
            .find(|(pd, _)| *pd == d)
            .map_or("", |(_, p)| match p {
                Phase::New => "new",
                Phase::FirstQuarter => "first",
                Phase::Full => "full",
                Phase::LastQuarter => "last",
            });
        days.push(CalendarDay {
            date: d.iso(),
            lunar_day,
            lunar_month,
            phase: phase.into(),
        });
        d = d.add_days(1);
    }
    CalendarView {
        system: system.key().into(),
        from: from.iso(),
        to: to.iso(),
        days,
        observances: observances(system, options, tz, from, to)
            .iter()
            .map(view)
            .collect(),
        sunday_shift: options.sunday_shift,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ny() -> Tz {
        Tz::parse("EST5EDT,M3.2.0,M11.1.0").unwrap()
    }

    fn d(s: &str) -> Date {
        Date::parse(s).unwrap()
    }

    fn find<'a>(list: &'a [Observance], name: &str) -> Vec<&'a Observance> {
        list.iter().filter(|o| o.name.starts_with(name)).collect()
    }

    #[test]
    fn chinese_new_years() {
        // As published (China time); the household zone gives the same day here.
        for (year, cny) in [
            (2025, "2025-01-29"),
            (2026, "2026-02-17"),
            (2027, "2027-02-06"),
        ] {
            let list = observances(
                System::Chinese,
                Options::default(),
                &ny(),
                d(&format!("{year}-01-01")),
                d(&format!("{year}-12-31")),
            );
            let ny_day = find(&list, "Lunar New Year");
            assert_eq!(ny_day.len(), 1, "{year}");
            assert_eq!(ny_day[0].date.iso(), cny, "{year}");
        }
    }

    #[test]
    fn chinese_months_are_29_or_30_days_with_a_leap_now_and_then() {
        let months = chinese_months(&ny(), d("2026-01-01"), d("2030-12-31"));
        assert!(months.iter().all(|m| m.days == 29 || m.days == 30));
        let leaps: Vec<_> = months
            .iter()
            .filter(|m| m.leap)
            .map(|m| (m.start.year, m.number))
            .collect();
        // 2028 has a leap 5th month in the published calendar.
        assert!(leaps.contains(&(2028, 5)), "{leaps:?}");
        for w in months.windows(2) {
            assert_eq!(w[0].start.add_days(i64::from(w[0].days)), w[1].start);
        }
        assert_eq!(chinese_day(&months, d("2026-02-17")), Some((1, false, 1)));
    }

    #[test]
    fn fasting_days_six_or_ten() {
        let six = observances(
            System::Chinese,
            Options::default(),
            &ny(),
            d("2026-03-01"),
            d("2026-03-31"),
        );
        let ten = observances(
            System::Chinese,
            Options {
                ten_fasting_days: true,
                ..Options::default()
            },
            &ny(),
            d("2026-03-01"),
            d("2026-03-31"),
        );
        let count = |l: &[Observance]| l.iter().filter(|o| o.kind == "fasting").count();
        assert!((5..=7).contains(&count(&six)), "{}", count(&six));
        assert!(count(&ten) > count(&six));
    }

    #[test]
    fn theravada_uposatha_and_festivals() {
        let year = observances(
            System::Theravada,
            Options::default(),
            &ny(),
            d("2026-01-01"),
            d("2026-12-31"),
        );
        let uposatha = year.iter().filter(|o| o.kind == "uposatha").count();
        assert!((24..=27).contains(&uposatha), "{uposatha}");
        let date = |n: &str| find(&year, n)[0].date.iso();
        // Our rule; it happens to match Thailand's 2026 dates.
        assert_eq!(date("Māgha"), "2026-03-03");
        assert_eq!(date("Vesak"), "2026-05-31");
        assert_eq!(date("Āsāḷha"), "2026-07-29");
        assert_eq!(date("Vassa"), "2026-07-30");
        let with_quarters = observances(
            System::Theravada,
            Options {
                quarter_moons: true,
                ..Options::default()
            },
            &ny(),
            d("2026-01-01"),
            d("2026-12-31"),
        );
        assert!(
            with_quarters
                .iter()
                .filter(|o| o.kind == "uposatha")
                .count()
                >= 48
        );
    }

    #[test]
    fn japanese_fixed_days_and_higan() {
        let year = observances(
            System::Japanese,
            Options::default(),
            &ny(),
            d("2026-01-01"),
            d("2026-12-31"),
        );
        assert_eq!(find(&year, "Hanamatsuri")[0].date.iso(), "2026-04-08");
        assert_eq!(
            find(&year, "Spring Higan (equinox)")[0].date.iso(),
            "2026-03-20"
        );
        assert_eq!(
            find(&year, "Spring Higan begins")[0].date.iso(),
            "2026-03-17"
        );
    }

    #[test]
    fn one_feast_a_month_spread_out() {
        for system in [System::Theravada, System::Chinese, System::Japanese] {
            let list = observances(
                system,
                Options::default(),
                &ny(),
                d("2026-01-01"),
                d("2030-12-31"),
            );
            let feasts: Vec<Date> = list.iter().filter(|o| o.feast).map(|o| o.date).collect();
            assert_eq!(feasts.len(), 60, "{system:?}");
            for w in feasts.windows(2) {
                let gap = w[1].days() - w[0].days();
                assert!(
                    (5..=57).contains(&gap),
                    "{system:?}: {gap} days from {}",
                    w[0].iso()
                );
            }
            // Festivals win: every Vesak, New Year and Hanamatsuri is a feast.
            for o in &list {
                if [
                    "Vesak",
                    "Lunar New Year",
                    "Hanamatsuri (the Buddha's birthday)",
                ]
                .contains(&o.name.as_str())
                {
                    assert!(o.feast, "{} {}", o.date.iso(), o.name);
                }
            }
        }
    }

    #[test]
    fn sunday_shift_moves_to_the_closest_sunday() {
        let on = Options {
            sunday_shift: true,
            ..Options::default()
        };
        for system in [System::Theravada, System::Chinese, System::Japanese] {
            for o in observances(system, on, &ny(), d("2026-01-01"), d("2027-12-31")) {
                assert_eq!(o.date.weekday(), 0, "{} {}", o.date.iso(), o.name);
                if let Some(from) = o.shifted_from {
                    assert!((o.date.days() - from.days()).abs() <= 3);
                }
            }
        }
        // Wednesday goes back, Thursday forward.
        assert_eq!(closest_sunday(d("2026-10-21")).iso(), "2026-10-18");
        assert_eq!(closest_sunday(d("2026-10-22")).iso(), "2026-10-25");
    }

    #[test]
    fn a_range_gives_the_same_answer_as_a_whole_year() {
        let all = observances(
            System::Chinese,
            Options::default(),
            &ny(),
            d("2026-01-01"),
            d("2026-12-31"),
        );
        let part = observances(
            System::Chinese,
            Options::default(),
            &ny(),
            d("2026-05-10"),
            d("2026-06-20"),
        );
        let slice: Vec<_> = all
            .iter()
            .filter(|o| o.date >= d("2026-05-10") && o.date <= d("2026-06-20"))
            .cloned()
            .collect();
        assert_eq!(part, slice);
    }
}
