//! Golden files for the meritledger lunar calendars (`sprints/sprint07.md`):
//! each system's observances for 2026–2030, New York time, default options.
//! A change in the calendar code must change these files on purpose:
//!
//!     UPDATE_GOLDEN=1 cargo test --test calendar_golden
//!
//! then review the diff (the owner reviews the holidays in place too).
use meritledger::calendar::{describe, observances, Options, System};
use meritledger::tz::{Date, Tz};
use std::path::PathBuf;

#[test]
fn observances_match_the_golden_files() {
    let tz = Tz::parse("EST5EDT,M3.2.0,M11.1.0").unwrap();
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let update = std::env::var_os("UPDATE_GOLDEN").is_some();
    let mut stale = Vec::new();
    for system in [System::Theravada, System::Chinese, System::Japanese] {
        for year in 2026..=2030 {
            let from = Date {
                year,
                month: 1,
                day: 1,
            };
            let to = Date {
                year,
                month: 12,
                day: 31,
            };
            let text = describe(&observances(system, Options::default(), &tz, from, to));
            let path = dir.join(format!("{}-{year}.txt", system.key()));
            if update {
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(&path, &text).unwrap();
            } else if std::fs::read_to_string(&path)
                .map(|s| s.replace("\r\n", "\n"))
                .ok()
                .as_deref()
                != Some(text.as_str())
            {
                stale.push(path.display().to_string());
            }
        }
    }
    assert!(
        stale.is_empty(),
        "calendar output changed (or golden files missing): {stale:?}\nrun `UPDATE_GOLDEN=1 cargo test --test calendar_golden` and review the diff"
    );
}
