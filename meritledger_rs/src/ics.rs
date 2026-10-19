//! iCalendar feeds of the meritledger lunar calendars (RFC 5545, by hand
//! like gamematcher's): `GET /cal/<system>.ics`, one all-day event per
//! observance. Public, like the calendar page: it holds no one's data.
use crate::calendar::Observance;
use crate::tz::Date;

/// RFC 5545 TEXT escaping.
fn escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

/// Folds a content line at 75 octets, never inside a UTF-8 character.
fn fold(line: &str, out: &mut String) {
    let mut width = 0;
    for ch in line.chars() {
        let n = ch.len_utf8();
        if width + n > 75 {
            out.push_str("\r\n ");
            width = 1;
        }
        out.push(ch);
        width += n;
    }
    out.push_str("\r\n");
}

fn compact(d: Date) -> String {
    format!("{:04}{:02}{:02}", d.year, d.month, d.day)
}

fn slug(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// A whole feed. `now` is UTC seconds (DTSTAMP).
pub fn feed(title: &str, list: &[Observance], now: u64) -> String {
    let stamp = {
        let d = Date::from_days((now / 86_400) as i64);
        let s = now % 86_400;
        format!(
            "{}T{:02}{:02}{:02}Z",
            compact(d),
            s / 3600,
            s % 3600 / 60,
            s % 60
        )
    };
    let mut out = String::new();
    for line in [
        "BEGIN:VCALENDAR",
        "VERSION:2.0",
        "PRODID:-//Merit Ledger//meritledger lunar calendars//EN",
        "CALSCALE:GREGORIAN",
        "METHOD:PUBLISH",
    ] {
        fold(line, &mut out);
    }
    fold(&format!("X-WR-CALNAME:{}", escape(title)), &mut out);
    for o in list {
        let summary = if o.feast {
            format!("{} (feast day)", o.name)
        } else {
            o.name.clone()
        };
        fold("BEGIN:VEVENT", &mut out);
        fold(
            &format!(
                "UID:{}-{}@meritledger.local",
                compact(o.date),
                slug(&o.name)
            ),
            &mut out,
        );
        fold(&format!("DTSTAMP:{stamp}"), &mut out);
        fold(&format!("DTSTART;VALUE=DATE:{}", compact(o.date)), &mut out);
        fold(
            &format!("DTEND;VALUE=DATE:{}", compact(o.date.add_days(1))),
            &mut out,
        );
        fold(&format!("SUMMARY:{}", escape(&summary)), &mut out);
        if let Some(from) = o.shifted_from {
            fold(
                &format!(
                    "DESCRIPTION:{}",
                    escape(&format!("Moved to Sunday from {}", from.iso()))
                ),
                &mut out,
            );
        }
        fold(&format!("CATEGORIES:{}", o.kind.to_uppercase()), &mut out);
        fold("TRANSP:TRANSPARENT", &mut out);
        fold("END:VEVENT", &mut out);
    }
    fold("END:VCALENDAR", &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn o(date: &str, name: &str, feast: bool) -> Observance {
        Observance {
            date: Date::parse(date).unwrap(),
            kind: "festival",
            name: name.into(),
            feast,
            shifted_from: None,
        }
    }

    #[test]
    fn a_feed_is_valid_ical() {
        let text = feed(
            "Merit Ledger, Chinese-inspired",
            &[
                o("2026-05-23", "Buddha's Birthday", true),
                o("2026-12-31", "A, b; c", false),
            ],
            1_792_281_600,
        );
        assert!(text.starts_with("BEGIN:VCALENDAR\r\n") && text.ends_with("END:VCALENDAR\r\n"));
        assert!(text.contains("DTSTART;VALUE=DATE:20260523\r\nDTEND;VALUE=DATE:20260524\r\n"));
        assert!(text.contains("SUMMARY:Buddha's Birthday (feast day)\r\n"));
        assert!(text.contains("SUMMARY:A\\, b\\; c\r\n"));
        assert!(text.contains("UID:20260523-buddha-s-birthday@meritledger.local"));
        assert!(text.contains("DTSTAMP:20261018T000000Z"));
        assert!(text.contains("X-WR-CALNAME:Merit Ledger\\, Chinese-inspired"));
        assert_eq!(text.matches("BEGIN:VEVENT").count(), 2);
    }

    #[test]
    fn long_lines_fold_without_splitting_characters() {
        let mut out = String::new();
        let line = format!("SUMMARY:{}", "Āsāḷha Pūjā ".repeat(12));
        fold(&line, &mut out);
        for piece in out.split("\r\n").filter(|p| !p.is_empty()) {
            assert!(piece.len() <= 75, "{} octets", piece.len());
        }
        let unfolded = out.replace("\r\n ", "").replace("\r\n", "");
        assert_eq!(unfolded, line);
    }
}
