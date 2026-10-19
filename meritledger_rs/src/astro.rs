//! Moon phases and the sun's longitude (`sprints/sprint07.md`), after Jean
//! Meeus, *Astronomical Algorithms* (2nd ed.): chapter 49 for the phases
//! (accurate to about a minute), chapter 25's low-accuracy sun for solar
//! terms (about 0.01°, so a solar term to within ~15 minutes). Plain `f64`
//! maths, no I/O; times are Unix seconds (UTC).
//!
//! These instants define the "meritledger lunar calendars": the calendars
//! follow this code, not any official body's announcements.

const DEG: f64 = std::f64::consts::PI / 180.0;
/// Mean length of a lunation, days.
pub const SYNODIC_MONTH: f64 = 29.530_588_861;

/// Which phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Phase {
    New,
    FirstQuarter,
    Full,
    LastQuarter,
}

impl Phase {
    fn offset(self) -> f64 {
        match self {
            Phase::New => 0.0,
            Phase::FirstQuarter => 0.25,
            Phase::Full => 0.5,
            Phase::LastQuarter => 0.75,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Phase::New => "New moon",
            Phase::FirstQuarter => "First quarter",
            Phase::Full => "Full moon",
            Phase::LastQuarter => "Last quarter",
        }
    }
}

fn sin(deg: f64) -> f64 {
    (deg * DEG).sin()
}

fn cos(deg: f64) -> f64 {
    (deg * DEG).cos()
}

/// Julian Ephemeris Day to Unix seconds, correcting TT to UT with ΔT
/// (NASA's polynomial for 2005–2050, fine a few decades either side).
fn jde_to_unix(jde: f64) -> i64 {
    let year = 2000.0 + (jde - 2_451_545.0) / 365.25;
    let t = year - 2000.0;
    let delta_t = 62.92 + 0.32217 * t + 0.005589 * t * t;
    ((jde - 2_440_587.5) * 86_400.0 - delta_t).round() as i64
}

fn unix_to_jd(unix: i64) -> f64 {
    unix as f64 / 86_400.0 + 2_440_587.5
}

/// The instant of `phase` in lunation `k` (k = 0 is the new moon of
/// 2000-01-06), Meeus ch. 49.
pub fn phase_time(k: i64, phase: Phase) -> i64 {
    let k = k as f64 + phase.offset();
    let t = k / 1236.85;
    let (t2, t3, t4) = (t * t, t * t * t, t * t * t * t);
    let mut jde = 2_451_550.097_66 + 29.530_588_861 * k + 0.000_154_37 * t2 - 0.000_000_150 * t3
        + 0.000_000_000_73 * t4;
    let e = 1.0 - 0.002_516 * t - 0.000_007_4 * t2;
    let m = 2.5534 + 29.105_356_70 * k - 0.000_001_4 * t2 - 0.000_000_11 * t3;
    let mp =
        201.5643 + 385.816_935_28 * k + 0.010_758_2 * t2 + 0.000_012_38 * t3 - 0.000_000_058 * t4;
    let f =
        160.7108 + 390.670_502_84 * k - 0.001_611_8 * t2 - 0.000_002_27 * t3 + 0.000_000_011 * t4;
    let om = 124.7746 - 1.563_755_88 * k + 0.002_067_2 * t2 + 0.000_002_15 * t3;
    jde += match phase {
        Phase::New | Phase::Full => {
            let (a, b, c, d) = if phase == Phase::New {
                (-0.40720, 0.17241, 0.01608, 0.01039)
            } else {
                (-0.40614, 0.17302, 0.01614, 0.01043)
            };
            let (g, h) = if phase == Phase::New {
                (0.00739, 0.00208)
            } else {
                (0.00734, 0.00209)
            };
            a * sin(mp)
                + b * e * sin(m)
                + c * sin(2.0 * mp)
                + d * sin(2.0 * f)
                + g * e * sin(mp - m)
                - 0.00514 * e * sin(mp + m)
                + h * e * e * sin(2.0 * m)
                - 0.00111 * sin(mp - 2.0 * f)
                - 0.00057 * sin(mp + 2.0 * f)
                + 0.00056 * e * sin(2.0 * mp + m)
                - 0.00042 * sin(3.0 * mp)
                + 0.00042 * e * sin(m + 2.0 * f)
                + 0.00038 * e * sin(m - 2.0 * f)
                - 0.00024 * e * sin(2.0 * mp - m)
                - 0.00017 * sin(om)
                - 0.00007 * sin(mp + 2.0 * m)
                + 0.00004 * sin(2.0 * mp - 2.0 * f)
                + 0.00004 * sin(3.0 * m)
                + 0.00003 * sin(mp + m - 2.0 * f)
                + 0.00003 * sin(2.0 * mp + 2.0 * f)
                - 0.00003 * sin(mp + m + 2.0 * f)
                + 0.00003 * sin(mp - m + 2.0 * f)
                - 0.00002 * sin(mp - m - 2.0 * f)
                - 0.00002 * sin(3.0 * mp + m)
                + 0.00002 * sin(4.0 * mp)
        }
        Phase::FirstQuarter | Phase::LastQuarter => {
            let base = -0.62801 * sin(mp) + 0.17172 * e * sin(m) - 0.01183 * e * sin(mp + m)
                + 0.00862 * sin(2.0 * mp)
                + 0.00804 * sin(2.0 * f)
                + 0.00454 * e * sin(mp - m)
                + 0.00204 * e * e * sin(2.0 * m)
                - 0.00180 * sin(mp - 2.0 * f)
                - 0.00070 * sin(mp + 2.0 * f)
                - 0.00040 * sin(3.0 * mp)
                - 0.00034 * e * sin(2.0 * mp - m)
                + 0.00032 * e * sin(m + 2.0 * f)
                + 0.00032 * e * sin(m - 2.0 * f)
                - 0.00028 * e * e * sin(mp + 2.0 * m)
                + 0.00027 * e * sin(2.0 * mp + m)
                - 0.00017 * sin(om)
                - 0.00005 * sin(mp - m - 2.0 * f)
                + 0.00004 * sin(2.0 * mp + 2.0 * f)
                - 0.00004 * sin(mp + m + 2.0 * f)
                + 0.00004 * sin(mp - 2.0 * m)
                + 0.00003 * sin(mp + m - 2.0 * f)
                + 0.00003 * sin(3.0 * m)
                + 0.00002 * sin(2.0 * mp - 2.0 * f)
                + 0.00002 * sin(mp - m + 2.0 * f)
                - 0.00002 * sin(3.0 * mp + m);
            let w = 0.00306 - 0.00038 * e * cos(m) + 0.00025 * cos(mp) - 0.00002 * cos(mp - m)
                + 0.00002 * cos(mp + m)
                + 0.00002 * cos(2.0 * f);
            base + if phase == Phase::FirstQuarter { w } else { -w }
        }
    };
    // Planetary arguments.
    let a = [
        (299.77 + 0.107_408 * k - 0.009_173 * t2, 0.000_325),
        (251.88 + 0.016_321 * k, 0.000_165),
        (251.83 + 26.651_886 * k, 0.000_164),
        (349.42 + 36.412_478 * k, 0.000_126),
        (84.66 + 18.206_239 * k, 0.000_110),
        (141.74 + 53.303_771 * k, 0.000_062),
        (207.14 + 2.453_732 * k, 0.000_060),
        (154.84 + 7.306_860 * k, 0.000_056),
        (34.52 + 27.261_239 * k, 0.000_047),
        (207.19 + 0.121_824 * k, 0.000_042),
        (291.34 + 1.844_379 * k, 0.000_040),
        (161.72 + 24.198_154 * k, 0.000_037),
        (239.56 + 25.513_099 * k, 0.000_035),
        (331.55 + 3.592_518 * k, 0.000_023),
    ];
    jde += a.iter().map(|(arg, coef)| coef * sin(*arg)).sum::<f64>();
    jde_to_unix(jde)
}

/// Every phase in `[from, to)` (Unix seconds), in time order.
pub fn phases_between(from: i64, to: i64) -> Vec<(i64, Phase)> {
    // Lunation number near `from`, one early to be safe.
    let k0 = ((unix_to_jd(from) - 2_451_550.097_66) / SYNODIC_MONTH).floor() as i64 - 1;
    let mut out = Vec::new();
    let mut k = k0;
    loop {
        let mut past_end = true;
        for phase in [
            Phase::New,
            Phase::FirstQuarter,
            Phase::Full,
            Phase::LastQuarter,
        ] {
            let t = phase_time(k, phase);
            if t < to {
                past_end = false;
                if t >= from {
                    out.push((t, phase));
                }
            }
        }
        if past_end {
            break;
        }
        k += 1;
    }
    out
}

/// The sun's apparent ecliptic longitude at `unix`, degrees in [0, 360)
/// (Meeus ch. 25, low accuracy).
pub fn sun_longitude(unix: i64) -> f64 {
    let t = (unix_to_jd(unix) - 2_451_545.0) / 36_525.0;
    let l0 = 280.466_46 + 36_000.769_83 * t + 0.000_303_2 * t * t;
    let m = 357.529_11 + 35_999.050_29 * t - 0.000_153_7 * t * t;
    let c = (1.914_602 - 0.004_817 * t - 0.000_014 * t * t) * sin(m)
        + (0.019_993 - 0.000_101 * t) * sin(2.0 * m)
        + 0.000_289 * sin(3.0 * m);
    let om = 125.04 - 1934.136 * t;
    (l0 + c - 0.005_69 - 0.004_78 * sin(om)).rem_euclid(360.0)
}

/// The instant after `after` when the sun reaches `longitude` degrees (0 =
/// March equinox, 90 = June solstice, 270 = December solstice).
pub fn sun_reaches(longitude: f64, after: i64) -> i64 {
    // ~0.9856° a day: start from the mean estimate, then Newton steps.
    let gap = (longitude - sun_longitude(after)).rem_euclid(360.0);
    let mut t = after + (gap / 360.0 * 365.2422 * 86_400.0) as i64;
    for _ in 0..6 {
        let diff = ((longitude - sun_longitude(t) + 540.0).rem_euclid(360.0)) - 180.0;
        let step = (diff / 360.0 * 365.2422 * 86_400.0) as i64;
        t += step;
        if step.abs() < 1 {
            break;
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tz::Date;

    fn utc(y: i32, m: u32, d: u32, h: i64, min: i64) -> i64 {
        Date {
            year: y,
            month: m,
            day: d,
        }
        .days()
            * 86_400
            + h * 3600
            + min * 60
    }

    fn near(got: i64, want: i64, minutes: i64) {
        assert!(
            (got - want).abs() <= minutes * 60,
            "off by {} min",
            (got - want) / 60
        );
    }

    #[test]
    fn known_moon_phases() {
        // New moon 2000-01-06 18:14 UTC (Meeus' k = 0).
        near(phase_time(0, Phase::New), utc(2000, 1, 6, 18, 14), 2);
        let found = |from: i64, phase: Phase| {
            phases_between(from, from + 35 * 86_400)
                .into_iter()
                .find(|p| p.1 == phase)
                .unwrap()
                .0
        };
        // The 2024-04-08 total solar eclipse: new moon 18:21 UTC.
        near(
            found(utc(2024, 4, 1, 0, 0), Phase::New),
            utc(2024, 4, 8, 18, 21),
            3,
        );
        // Full moons 2023-12-27 00:33 and 2025-01-13 22:27 UTC.
        near(
            found(utc(2023, 12, 20, 0, 0), Phase::Full),
            utc(2023, 12, 27, 0, 33),
            3,
        );
        near(
            found(utc(2025, 1, 5, 0, 0), Phase::Full),
            utc(2025, 1, 13, 22, 27),
            3,
        );
    }

    #[test]
    fn phases_come_in_order_about_a_week_apart() {
        let list = phases_between(utc(2026, 1, 1, 0, 0), utc(2031, 1, 1, 0, 0));
        assert!(list.len() > 4 * 12 * 5);
        for w in list.windows(2) {
            let gap = (w[1].0 - w[0].0) as f64 / 86_400.0;
            assert!(
                (6.0..8.6).contains(&gap),
                "{gap} days between {:?}",
                (w[0].1, w[1].1)
            );
            let next = match w[0].1 {
                Phase::New => Phase::FirstQuarter,
                Phase::FirstQuarter => Phase::Full,
                Phase::Full => Phase::LastQuarter,
                Phase::LastQuarter => Phase::New,
            };
            assert_eq!(w[1].1, next);
        }
    }

    #[test]
    fn equinoxes_and_solstices() {
        // 2024 March equinox 03:06 UTC; 2025 December solstice 15:03 UTC.
        near(
            sun_reaches(0.0, utc(2024, 3, 1, 0, 0)),
            utc(2024, 3, 20, 3, 6),
            30,
        );
        near(
            sun_reaches(270.0, utc(2025, 12, 1, 0, 0)),
            utc(2025, 12, 21, 15, 3),
            30,
        );
        let june = sun_reaches(90.0, utc(2026, 5, 1, 0, 0));
        assert_eq!(Date::from_days(june.div_euclid(86_400)).iso(), "2026-06-21");
    }
}
