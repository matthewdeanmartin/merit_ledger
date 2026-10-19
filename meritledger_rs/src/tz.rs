//! The household time zone as a POSIX TZ string (`EST5EDT,M3.2.0,M11.1.0`),
//! evaluated in plain Rust so desktop, board and tests agree without a
//! time-zone database. "Done today", calendar days and reminders all use
//! [`Tz::local_date`].
//!
//! Supported: `std offset [dst [offset] [,start[/time],end[/time]]]` with
//! `<...>` quoted names and `Mm.w.d` rules (what every US/EU/AU zone uses).
//! Julian-day rules (`Jn`, `n`) are refused rather than guessed.

/// A civil date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Date {
    /// `YYYY-MM-DD`.
    pub fn iso(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    pub fn parse(s: &str) -> Option<Date> {
        let mut parts = s.split('-');
        let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() || y.len() != 4 || m.len() != 2 || d.len() != 2 {
            return None;
        }
        let date = Date {
            year: y.parse().ok()?,
            month: m.parse().ok()?,
            day: d.parse().ok()?,
        };
        (date.month >= 1
            && date.month <= 12
            && date.day >= 1
            && date.day <= days_in_month(date.year, date.month))
        .then_some(date)
    }

    /// Days since 1970-01-01.
    pub fn days(&self) -> i64 {
        days_from_civil(self.year, self.month, self.day)
    }

    pub fn from_days(days: i64) -> Date {
        civil_from_days(days)
    }

    pub fn add_days(&self, n: i64) -> Date {
        Date::from_days(self.days() + n)
    }

    /// 0 = Sunday .. 6 = Saturday.
    pub fn weekday(&self) -> u32 {
        (self.days() + 4).rem_euclid(7) as u32
    }
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

pub fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if is_leap(y) => 29,
        _ => 28,
    }
}

/// Howard Hinnant's `days_from_civil`.
fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = i64::from(y) - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(m);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Howard Hinnant's `civil_from_days`.
fn civil_from_days(z: i64) -> Date {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
    Date { year, month, day }
}

/// `Mm.w.d/time`: the d-th weekday (0 = Sunday) of week w (5 = last) of
/// month m, at `time` seconds after local midnight.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Rule {
    month: u32,
    week: u32,
    weekday: u32,
    time: i64,
}

impl Rule {
    fn date(&self, year: i32) -> Date {
        let first = Date {
            year,
            month: self.month,
            day: 1,
        };
        let offset = (self.weekday + 7 - first.weekday()) % 7;
        let mut day = 1 + offset + 7 * (self.week - 1);
        while day > days_in_month(year, self.month) {
            day -= 7;
        }
        Date {
            year,
            month: self.month,
            day,
        }
    }
}

/// A parsed POSIX TZ.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Tz {
    /// Seconds to add to UTC for local standard time (New York: -18000).
    std_offset: i64,
    /// Daylight time: offset to add to UTC, start and end rules.
    dst: Option<(i64, Rule, Rule)>,
}

struct Cursor<'a> {
    s: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.at).copied()
    }

    fn eat(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn name(&mut self) -> Result<(), String> {
        if self.eat(b'<') {
            while let Some(c) = self.peek() {
                self.at += 1;
                if c == b'>' {
                    return Ok(());
                }
            }
            return Err("unclosed <name>".into());
        }
        let start = self.at;
        while self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
            self.at += 1;
        }
        if self.at - start < 3 {
            return Err("zone names have at least 3 letters".into());
        }
        Ok(())
    }

    fn number(&mut self) -> Result<i64, String> {
        let start = self.at;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.at += 1;
        }
        std::str::from_utf8(&self.s[start..self.at])
            .unwrap_or("")
            .parse()
            .map_err(|_| "expected a number".to_string())
    }

    /// `[+-]hh[:mm[:ss]]` in seconds.
    fn time(&mut self) -> Result<i64, String> {
        let sign = if self.eat(b'-') {
            -1
        } else {
            self.eat(b'+');
            1
        };
        let mut secs = self.number()? * 3600;
        if self.eat(b':') {
            secs += self.number()? * 60;
            if self.eat(b':') {
                secs += self.number()?;
            }
        }
        Ok(sign * secs)
    }

    fn rule(&mut self) -> Result<Rule, String> {
        if !self.eat(b'M') {
            return Err("only Mm.w.d rules are supported".into());
        }
        let month = self.number()? as u32;
        let week = if self.eat(b'.') {
            self.number()? as u32
        } else {
            0
        };
        let weekday = if self.eat(b'.') {
            self.number()? as u32
        } else {
            7
        };
        if !(1..=12).contains(&month) || !(1..=5).contains(&week) || weekday > 6 {
            return Err("bad Mm.w.d rule".into());
        }
        let time = if self.eat(b'/') { self.time()? } else { 7200 };
        Ok(Rule {
            month,
            week,
            weekday,
            time,
        })
    }
}

impl Tz {
    pub fn utc() -> Tz {
        Tz {
            std_offset: 0,
            dst: None,
        }
    }

    pub fn parse(spec: &str) -> Result<Tz, String> {
        let mut c = Cursor {
            s: spec.trim().as_bytes(),
            at: 0,
        };
        c.name()?;
        // POSIX offsets are west-positive; ours are east-positive.
        let std_offset = -c.time()?;
        if c.peek().is_none() {
            return Ok(Tz {
                std_offset,
                dst: None,
            });
        }
        c.name()?;
        let dst_offset = match c.peek() {
            Some(b',') | None => std_offset + 3600,
            _ => -c.time()?,
        };
        if !c.eat(b',') {
            return Err("daylight time needs start and end rules".into());
        }
        let start = c.rule()?;
        if !c.eat(b',') {
            return Err("daylight time needs an end rule".into());
        }
        let end = c.rule()?;
        if c.peek().is_some() {
            return Err("unexpected text at the end".into());
        }
        Ok(Tz {
            std_offset,
            dst: Some((dst_offset, start, end)),
        })
    }

    /// Seconds to add to UTC for local time at `utc` (Unix seconds).
    pub fn offset_at(&self, utc: i64) -> i64 {
        let Some((dst_offset, start, end)) = self.dst else {
            return self.std_offset;
        };
        let year = Date::from_days((utc + self.std_offset).div_euclid(86_400)).year;
        // Transitions as UTC instants: the start rule is in standard time,
        // the end rule in daylight time.
        let at = |rule: Rule, offset: i64| rule.date(year).days() * 86_400 + rule.time - offset;
        let (on, off) = (at(start, self.std_offset), at(end, dst_offset));
        let in_dst = if on < off {
            utc >= on && utc < off
        } else {
            // Southern hemisphere: daylight time spans New Year.
            utc >= on || utc < off
        };
        if in_dst {
            dst_offset
        } else {
            self.std_offset
        }
    }

    /// The household-local calendar date at `utc`.
    pub fn local_date(&self, utc: i64) -> Date {
        Date::from_days((utc + self.offset_at(utc)).div_euclid(86_400))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unix seconds for a UTC civil time.
    fn utc(y: i32, m: u32, d: u32, h: i64, min: i64) -> i64 {
        days_from_civil(y, m, d) * 86_400 + h * 3600 + min * 60
    }

    #[test]
    fn civil_round_trip_and_weekday() {
        for days in [-1_000_000i64, -1, 0, 1, 19_000, 20_744, 2_000_000] {
            assert_eq!(Date::from_days(days).days(), days);
        }
        let d = Date::parse("2026-10-18").unwrap();
        assert_eq!(d.weekday(), 0, "a Sunday");
        assert_eq!(d.add_days(14).iso(), "2026-11-01");
        assert!(Date::parse("2026-02-29").is_none());
        assert!(Date::parse("2028-02-29").is_some());
        assert!(Date::parse("2026-1-1").is_none());
    }

    #[test]
    fn new_york() {
        let tz = Tz::parse("EST5EDT,M3.2.0,M11.1.0").unwrap();
        // 2026: daylight time from Mar 8 07:00 UTC to Nov 1 06:00 UTC.
        assert_eq!(tz.offset_at(utc(2026, 3, 8, 6, 59)), -5 * 3600);
        assert_eq!(tz.offset_at(utc(2026, 3, 8, 7, 0)), -4 * 3600);
        assert_eq!(tz.offset_at(utc(2026, 11, 1, 5, 59)), -4 * 3600);
        assert_eq!(tz.offset_at(utc(2026, 11, 1, 6, 0)), -5 * 3600);
        // 03:30 UTC on Oct 19 is still the evening of Oct 18 in New York.
        assert_eq!(tz.local_date(utc(2026, 10, 19, 3, 30)).iso(), "2026-10-18");
        assert_eq!(tz.local_date(utc(2026, 10, 19, 4, 0)).iso(), "2026-10-19");
    }

    #[test]
    fn sydney_spans_new_year() {
        let tz = Tz::parse("AEST-10AEDT,M10.1.0,M4.1.0/3").unwrap();
        // 2026: daylight time ends Apr 5 (16:00 UTC Apr 4), starts Oct 4 (16:00 UTC Oct 3).
        assert_eq!(tz.offset_at(utc(2026, 1, 15, 0, 0)), 11 * 3600);
        assert_eq!(tz.offset_at(utc(2026, 4, 4, 15, 59)), 11 * 3600);
        assert_eq!(tz.offset_at(utc(2026, 4, 4, 16, 0)), 10 * 3600);
        assert_eq!(tz.offset_at(utc(2026, 10, 3, 15, 59)), 10 * 3600);
        assert_eq!(tz.offset_at(utc(2026, 10, 3, 16, 0)), 11 * 3600);
    }

    #[test]
    fn fixed_and_quoted_zones() {
        assert_eq!(Tz::parse("UTC0").unwrap(), Tz::utc());
        assert_eq!(
            Tz::parse("<+0530>-5:30").unwrap().offset_at(0),
            5 * 3600 + 1800
        );
        assert_eq!(
            Tz::parse("JST-9")
                .unwrap()
                .local_date(utc(2026, 12, 31, 15, 0))
                .iso(),
            "2027-01-01"
        );
    }

    #[test]
    fn bad_strings_are_refused() {
        for bad in [
            "",
            "E5",
            "EST",
            "EST5EDT",
            "EST5EDT,J60,J300",
            "EST5EDT,M13.1.0,M11.1.0",
            "EST5EDT,M3.2.0",
        ] {
            assert!(Tz::parse(bad).is_err(), "{bad}");
        }
    }
}
