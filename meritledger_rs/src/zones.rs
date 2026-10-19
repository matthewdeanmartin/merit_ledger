//! Time zones by place name (the Household page's picker): the household
//! picks "New York (Eastern)", or its phone's own zone, and the board keeps
//! the POSIX rule that `tz.rs` evaluates. No one types a rule unless they
//! open "Advanced".
//!
//! Rules are today's (2026); a country that changes its clocks needs an
//! update here, like any time-zone database.

/// (IANA name, label, POSIX rule)
pub const ZONES: &[(&str, &str, &str)] = &[
    ("Pacific/Honolulu", "Honolulu (Hawaii)", "HST10"),
    (
        "America/Anchorage",
        "Anchorage (Alaska)",
        "AKST9AKDT,M3.2.0,M11.1.0",
    ),
    (
        "America/Los_Angeles",
        "Los Angeles (Pacific)",
        "PST8PDT,M3.2.0,M11.1.0",
    ),
    (
        "America/Phoenix",
        "Phoenix (Arizona, no daylight time)",
        "MST7",
    ),
    (
        "America/Denver",
        "Denver (Mountain)",
        "MST7MDT,M3.2.0,M11.1.0",
    ),
    (
        "America/Chicago",
        "Chicago (Central)",
        "CST6CDT,M3.2.0,M11.1.0",
    ),
    (
        "America/New_York",
        "New York (Eastern)",
        "EST5EDT,M3.2.0,M11.1.0",
    ),
    (
        "America/Halifax",
        "Halifax (Atlantic)",
        "AST4ADT,M3.2.0,M11.1.0",
    ),
    (
        "America/St_Johns",
        "St. John's (Newfoundland)",
        "NST3:30NDT,M3.2.0,M11.1.0",
    ),
    ("America/Mexico_City", "Mexico City", "CST6"),
    ("America/Sao_Paulo", "São Paulo", "<-03>3"),
    ("UTC", "UTC", "UTC0"),
    ("Europe/London", "London", "GMT0BST,M3.5.0/1,M10.5.0"),
    ("Europe/Dublin", "Dublin", "GMT0IST,M3.5.0/1,M10.5.0"),
    ("Europe/Lisbon", "Lisbon", "WET0WEST,M3.5.0/1,M10.5.0"),
    (
        "Europe/Paris",
        "Paris, Berlin, Rome, Madrid (Central Europe)",
        "CET-1CEST,M3.5.0,M10.5.0/3",
    ),
    (
        "Europe/Athens",
        "Athens, Helsinki (Eastern Europe)",
        "EET-2EEST,M3.5.0/3,M10.5.0/4",
    ),
    ("Europe/Moscow", "Moscow", "MSK-3"),
    ("Asia/Dubai", "Dubai", "<+04>-4"),
    ("Asia/Kolkata", "India", "IST-5:30"),
    ("Asia/Kathmandu", "Kathmandu", "<+0545>-5:45"),
    ("Asia/Colombo", "Colombo (Sri Lanka)", "<+0530>-5:30"),
    ("Asia/Yangon", "Yangon (Myanmar)", "<+0630>-6:30"),
    ("Asia/Bangkok", "Bangkok, Hanoi", "<+07>-7"),
    ("Asia/Singapore", "Singapore, Kuala Lumpur", "<+08>-8"),
    ("Asia/Shanghai", "Beijing, Shanghai", "CST-8"),
    ("Asia/Taipei", "Taipei", "CST-8"),
    ("Asia/Hong_Kong", "Hong Kong", "HKT-8"),
    ("Asia/Seoul", "Seoul", "KST-9"),
    ("Asia/Tokyo", "Tokyo", "JST-9"),
    ("Australia/Perth", "Perth", "AWST-8"),
    (
        "Australia/Adelaide",
        "Adelaide",
        "ACST-9:30ACDT,M10.1.0,M4.1.0/3",
    ),
    ("Australia/Brisbane", "Brisbane", "AEST-10"),
    (
        "Australia/Sydney",
        "Sydney, Melbourne",
        "AEST-10AEDT,M10.1.0,M4.1.0/3",
    ),
    (
        "Pacific/Auckland",
        "Auckland",
        "NZST-12NZDT,M9.5.0,M4.1.0/3",
    ),
];

/// The zone with this IANA name (a phone's `Intl` time zone).
pub fn by_name(name: &str) -> Option<(&'static str, &'static str, &'static str)> {
    ZONES.iter().find(|(iana, _, _)| *iana == name).copied()
}

/// The friendly label for a stored POSIX rule, if it is one of ours.
pub fn label_of(posix: &str) -> Option<&'static str> {
    ZONES
        .iter()
        .find(|(_, _, p)| *p == posix)
        .map(|(_, label, _)| *label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tz::{Date, Tz};

    #[test]
    fn every_rule_parses_and_names_are_unique() {
        let mut names = std::collections::HashSet::new();
        for (iana, label, posix) in ZONES {
            Tz::parse(posix).unwrap_or_else(|e| panic!("{iana}: {e}"));
            assert!(names.insert(*iana), "{iana} twice");
            assert!(!label.is_empty());
        }
    }

    #[test]
    fn rules_give_the_offsets_we_expect() {
        // Noon UTC on 2026-07-01 (northern summer).
        let july = Date {
            year: 2026,
            month: 7,
            day: 1,
        }
        .days()
            * 86_400
            + 12 * 3600;
        let hours = |name: &str| {
            Tz::parse(by_name(name).unwrap().2).unwrap().offset_at(july) as f64 / 3600.0
        };
        assert_eq!(hours("America/New_York"), -4.0);
        assert_eq!(hours("Europe/London"), 1.0);
        assert_eq!(hours("Asia/Kolkata"), 5.5);
        assert_eq!(hours("Asia/Kathmandu"), 5.75);
        assert_eq!(hours("Australia/Sydney"), 10.0, "southern winter");
        assert_eq!(hours("America/St_Johns"), -2.5);
        assert_eq!(
            label_of("EST5EDT,M3.2.0,M11.1.0"),
            Some("New York (Eastern)")
        );
    }
}
