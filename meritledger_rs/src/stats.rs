//! Profiles, the household feed and Strava-like stats (`sprints/sprint04.md`).
//!
//! Stats are computed from the entries the board holds, as one viewer may
//! see them: someone else's private entries don't count, and generic ones
//! count as a day of practice but not their minutes. A streak is days in a
//! row with any merit. Until Sprint 5 moves history to the SD card, that is
//! the snapshot window; `history_from` says where the board's memory starts.
use crate::ledger::Viewer;
use crate::messages::{CategoryTotal, EntryView, FeedView, ProfileView, StoredDay, StoredEntry};
use crate::state::State;
use crate::tz::Date;
use miniframework::ApiError;
use std::collections::BTreeMap;

/// One person's day as totals, kept for days before the window (`State::history`).
/// The split lets one tally answer any viewer: the person (and admins) see
/// `own_*`; anyone else sees `open_*` plus `day_only` (generic and
/// sensitive merit counts the day, not its minutes; private merit not at all).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub own_entries: u32,
    pub own_minutes: u32,
    pub open_entries: u32,
    pub open_minutes: u32,
    pub day_only: u32,
    pub demerits: u32,
}

impl Tally {
    pub fn add(&mut self, o: Tally) {
        self.own_entries += o.own_entries;
        self.own_minutes += o.own_minutes;
        self.open_entries += o.open_entries;
        self.open_minutes += o.open_minutes;
        self.day_only += o.day_only;
        self.demerits += o.demerits;
    }

    /// The merit day as a viewer may see it, or `None` for no visible merit.
    /// Outsiders (guests, visitors) don't get the generic day counts.
    pub fn as_day(&self, own: bool, outsider: bool) -> Option<Day> {
        let day = if own {
            Day {
                entries: self.own_entries,
                minutes: self.own_minutes,
            }
        } else {
            Day {
                entries: self.open_entries + if outsider { 0 } else { self.day_only },
                minutes: self.open_minutes,
            }
        };
        (day.entries > 0).then_some(day)
    }

    pub fn from_stored(d: &StoredDay) -> Tally {
        Tally {
            own_entries: d.own_entries,
            own_minutes: d.own_minutes,
            open_entries: d.open_entries,
            open_minutes: d.open_minutes,
            day_only: d.day_only,
            demerits: d.demerits,
        }
    }

    pub fn to_stored(&self, person: u32, day: i64) -> StoredDay {
        StoredDay {
            person,
            day: day as i32,
            own_entries: self.own_entries,
            own_minutes: self.own_minutes,
            open_entries: self.open_entries,
            open_minutes: self.open_minutes,
            day_only: self.day_only,
            demerits: self.demerits,
        }
    }
}

impl State {
    /// What one entry adds to its day's tally, by the same rules as
    /// `entry_view` uses for viewers other than the person.
    pub fn tally_of(&self, e: &StoredEntry) -> Tally {
        let Some(kind) = self.kind(&e.kind) else {
            return Tally::default();
        };
        if kind.polarity != "merit" {
            return Tally {
                demerits: 1,
                ..Default::default()
            };
        }
        let minutes = if kind.unit == "minutes" { e.amount } else { 0 };
        let mut t = Tally {
            own_entries: 1,
            own_minutes: minutes,
            ..Default::default()
        };
        match e.visibility.as_str() {
            "private" => {}
            "generic" => t.day_only = 1,
            _ if crate::ledger::sensitive(&kind) => t.day_only = 1,
            _ => {
                t.open_entries = 1;
                t.open_minutes = minutes;
            }
        }
        t
    }

    /// Adds entries from before the window to `history` (boot, migration).
    pub fn fold_into_history<'a>(&mut self, entries: impl IntoIterator<Item = &'a StoredEntry>) {
        for e in entries {
            let Some(d) = Date::parse(&e.date) else {
                continue;
            };
            let t = self.tally_of(e);
            self.history.entry((e.person, d.days())).or_default().add(t);
        }
    }

    /// Moves the window to start `BACKFILL_DAYS` before `today`: older
    /// entries leave RAM for `history`. Without a journal their detail is
    /// gone, so their totals go to `archived_days` (persisted). Returns
    /// whether the snapshot changed.
    pub fn roll_window(&mut self, today: Date) -> bool {
        let start = today.days() - crate::ledger::BACKFILL_DAYS;
        if start <= self.window_start {
            return false;
        }
        self.window_start = start;
        let cut = Date::from_days(start).iso();
        let (old, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut self.entries)
            .into_iter()
            .partition(|e| e.date < cut);
        self.entries = keep;
        if old.is_empty() {
            return false;
        }
        let before: Vec<(u32, i64)> = old
            .iter()
            .filter_map(|e| Date::parse(&e.date).map(|d| (e.person, d.days())))
            .collect();
        self.fold_into_history(&old);
        if self.journaled {
            return false;
        }
        for key in before {
            let tally = self.history[&key];
            self.archived_days
                .retain(|d| (d.person, i64::from(d.day)) != key);
            self.archived_days.push(tally.to_stored(key.0, key.1));
        }
        true
    }
}

/// Days shown on a profile's heatmap: 26 weeks.
pub const HEATMAP_DAYS: i64 = 182;

/// One person's merit on one day.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Day {
    pub entries: u32,
    pub minutes: u32,
}

/// Heatmap shade, 0 (nothing) to 4.
pub fn level(day: Day) -> u32 {
    match (day.entries, day.minutes) {
        (0, _) => 0,
        (e, m) if e >= 5 || m >= 60 => 4,
        (e, m) if e >= 3 || m >= 30 => 3,
        (e, m) if e >= 2 || m >= 15 => 2,
        _ => 1,
    }
}

/// Days in a row with merit, ending today, or yesterday while today is
/// still empty (a streak isn't broken until the day is over).
pub fn current_streak(days: &BTreeMap<i64, Day>, today: i64) -> u32 {
    let start = if days.contains_key(&today) {
        today
    } else {
        today - 1
    };
    let mut n = 0;
    while days.contains_key(&(start - i64::from(n))) {
        n += 1;
    }
    n
}

pub fn longest_streak(days: &BTreeMap<i64, Day>) -> u32 {
    let (mut best, mut run, mut prev) = (0u32, 0u32, None::<i64>);
    for &d in days.keys() {
        run = if prev == Some(d - 1) { run + 1 } else { 1 };
        best = best.max(run);
        prev = Some(d);
    }
    best
}

fn totals<'a>(entries: impl Iterator<Item = &'a EntryView>) -> Vec<CategoryTotal> {
    let mut by: BTreeMap<String, CategoryTotal> = BTreeMap::new();
    for e in entries {
        let key = if e.generic {
            "other".to_string()
        } else {
            e.category.clone()
        };
        let t = by.entry(key.clone()).or_insert_with(|| CategoryTotal {
            category: key,
            ..Default::default()
        });
        t.count += 1;
        if e.unit == "minutes" {
            t.minutes += e.amount;
        }
    }
    let mut out: Vec<_> = by.into_values().collect();
    out.sort_by(|a, b| b.minutes.cmp(&a.minutes).then(b.count.cmp(&a.count)));
    out
}

impl State {
    /// Every entry of `person` that `viewer` may see, as views, with
    /// whether it is merit (known from the kind even when the view is
    /// generic).
    fn visible_entries(&self, viewer: Viewer, person: u32) -> Vec<(EntryView, bool)> {
        self.entries
            .iter()
            .filter(|e| e.person == person)
            .filter_map(|e| {
                let merit = self.kind(&e.kind).is_some_and(|k| k.polarity == "merit");
                self.entry_view(viewer, e).map(|v| (v, merit))
            })
            .collect()
    }

    pub fn profile(
        &self,
        viewer: Viewer,
        person: u32,
        now: Option<u64>,
    ) -> Result<ProfileView, ApiError> {
        let p = self
            .person(person)
            .filter(|p| !p.disabled)
            .ok_or_else(|| ApiError::not_found("No such person"))?;
        let today = self.today(now).ok_or_else(|| {
            ApiError::new(
                503,
                "clock_not_set",
                "The board's clock isn't set yet; try again in a minute",
            )
        })?;
        let own = viewer.admin || (viewer.id != 0 && viewer.id == person);
        let mut seen = self.visible_entries(viewer, person);
        seen.sort_by(|(a, _), (b, _)| {
            b.date
                .cmp(&a.date)
                .then(b.created.cmp(&a.created))
                .then(b.id.cmp(&a.id))
        });
        let merits: Vec<&EntryView> = seen.iter().filter(|(_, m)| *m).map(|(e, _)| e).collect();

        let mut days: BTreeMap<i64, Day> = BTreeMap::new();
        for e in &merits {
            let Some(d) = Date::parse(&e.date) else {
                continue;
            };
            let day = days.entry(d.days()).or_default();
            day.entries += 1;
            if e.unit == "minutes" {
                day.minutes += e.amount;
            }
        }
        // Before the window: per-day totals only (the journal at boot, or
        // what aged out without a card).
        for (&(_, d), tally) in self.history.range((person, i64::MIN)..=(person, i64::MAX)) {
            if d < self.window_start {
                if let Some(day) = tally.as_day(own, viewer.outsider) {
                    days.insert(d, day);
                }
            }
        }
        let t = today.days();
        let since = |n: i64| {
            let from = t - n + 1;
            merits.iter().copied().filter(move |e| {
                Date::parse(&e.date).is_some_and(|d| d.days() >= from && d.days() <= t)
            })
        };
        let week = totals(since(7));
        let start = today.add_days(-(HEATMAP_DAYS - 1));
        let heatmap = (0..HEATMAP_DAYS)
            .map(|i| level(days.get(&(start.days() + i)).copied().unwrap_or_default()))
            .collect();
        let oldest_entry = self
            .entries
            .iter()
            .filter(|e| e.person == person)
            .filter_map(|e| Date::parse(&e.date))
            .map(|d| d.days())
            .min();
        let oldest_tally = self
            .history
            .range((person, i64::MIN)..=(person, i64::MAX))
            .next()
            .map(|(&(_, d), _)| d);
        let history_from = match (oldest_entry, oldest_tally) {
            (Some(a), Some(b)) => Date::from_days(a.min(b)).iso(),
            (Some(d), None) | (None, Some(d)) => Date::from_days(d).iso(),
            (None, None) => today.iso(),
        };
        let demerits_30 = if own {
            let from = today.add_days(-29).iso();
            self.entries
                .iter()
                .filter(|e| e.person == person && e.date >= from)
                .filter(|e| self.kind(&e.kind).is_some_and(|k| k.polarity == "demerit"))
                .count() as u32
        } else {
            0
        };
        Ok(ProfileView {
            id: p.id,
            display_name: p.display_name.clone(),
            role: p.role.clone(),
            date: today.iso(),
            current_streak: current_streak(&days, t),
            longest_streak: longest_streak(&days),
            week_minutes: week.iter().map(|c| c.minutes).sum(),
            week,
            month: totals(since(30)),
            heatmap_start: start.iso(),
            heatmap,
            recent: seen.into_iter().map(|(e, _)| e).take(10).collect(),
            history_from,
            demerits_30,
            own,
        })
    }

    /// The household's recent entries that `viewer` may see, newest first.
    pub fn feed(&self, viewer: Viewer, offset: usize, limit: usize) -> FeedView {
        let mut all: Vec<_> = self.entries.iter().collect();
        all.sort_by(|a, b| {
            b.date
                .cmp(&a.date)
                .then(b.created.cmp(&a.created))
                .then(b.id.cmp(&a.id))
        });
        let mut visible = all
            .into_iter()
            .filter(|e| self.person(e.person).is_some_and(|p| !p.disabled))
            .filter_map(|e| self.entry_view(viewer, e))
            .skip(offset);
        let entries: Vec<_> = visible.by_ref().take(limit).collect();
        FeedView {
            more: visible.next().is_some(),
            entries,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::Actor;
    use crate::messages::EntryInput;
    use crate::state::{Limits, MEMBER};

    /// 2026-10-18 16:00 UTC: noon in New York.
    const NOON: u64 = 1_792_281_600 + 16 * 3600;

    fn days(list: &[i64]) -> BTreeMap<i64, Day> {
        list.iter()
            .map(|&d| {
                (
                    d,
                    Day {
                        entries: 1,
                        minutes: 0,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn streaks() {
        assert_eq!(current_streak(&days(&[8, 9, 10]), 10), 3);
        assert_eq!(
            current_streak(&days(&[8, 9]), 10),
            2,
            "today isn't over yet"
        );
        assert_eq!(current_streak(&days(&[7, 9]), 10), 1);
        assert_eq!(current_streak(&days(&[7]), 10), 0);
        assert_eq!(longest_streak(&days(&[1, 2, 3, 5, 6])), 3);
        assert_eq!(longest_streak(&days(&[])), 0);
    }

    #[test]
    fn levels() {
        let d = |entries, minutes| Day { entries, minutes };
        assert_eq!(
            [
                level(d(0, 0)),
                level(d(1, 5)),
                level(d(2, 0)),
                level(d(1, 30)),
                level(d(1, 60))
            ],
            [0, 1, 2, 3, 4]
        );
    }

    fn household() -> (State, u32, u32) {
        let mut s = State::new(Limits {
            pin_rounds: 2,
            ..Limits::board()
        });
        let dad = s.setup("Dad", "1234", 0).unwrap();
        let kid = s.add_person("Kid", MEMBER, "", "", 0).unwrap();
        (s, dad, kid)
    }

    fn log(s: &mut State, person: u32, kind: &str, date: &str, visibility: Option<&str>) {
        let input = EntryInput {
            person,
            kind: kind.into(),
            date: Some(date.into()),
            visibility: visibility.map(str::to_string),
            ..Default::default()
        };
        s.record(Actor::Board, &input, Some(NOON)).unwrap();
    }

    #[test]
    fn profile_counts_merit_streaks_and_minutes() {
        let (mut s, dad, kid) = household();
        for date in ["2026-10-15", "2026-10-16", "2026-10-17", "2026-10-18"] {
            log(&mut s, kid, "meditation", date, None);
        }
        log(&mut s, kid, "anger", "2026-10-18", None);
        log(&mut s, kid, "meditation", "2026-10-10", None);
        let p = s
            .profile(
                Viewer {
                    id: dad,
                    admin: false,
                    outsider: false,
                },
                kid,
                Some(NOON),
            )
            .unwrap();
        assert_eq!((p.current_streak, p.longest_streak), (4, 4));
        assert_eq!(p.week_minutes, 80);
        assert_eq!(p.week[0].category, "meditation");
        assert_eq!(p.heatmap.len(), HEATMAP_DAYS as usize);
        assert_eq!(*p.heatmap.last().unwrap(), 2, "20 minutes today");
        assert_eq!(p.history_from, "2026-10-10");
        assert!(!p.own);
        assert_eq!(p.demerits_30, 0, "not shown to others");
        assert_eq!(p.recent.len(), 5, "the private demerit is hidden");
        let mine = s
            .profile(
                Viewer {
                    id: kid,
                    admin: false,
                    outsider: false,
                },
                kid,
                Some(NOON),
            )
            .unwrap();
        assert_eq!(mine.demerits_30, 1);
        assert_eq!(
            mine.current_streak, 4,
            "demerits don't count toward streaks"
        );
    }

    #[test]
    fn others_private_merit_does_not_count() {
        let (mut s, dad, kid) = household();
        log(&mut s, kid, "meditation", "2026-10-18", Some("private"));
        let seen = s
            .profile(
                Viewer {
                    id: dad,
                    admin: false,
                    outsider: false,
                },
                kid,
                Some(NOON),
            )
            .unwrap();
        assert_eq!((seen.current_streak, seen.week_minutes), (0, 0));
        let own = s
            .profile(
                Viewer {
                    id: kid,
                    admin: false,
                    outsider: false,
                },
                kid,
                Some(NOON),
            )
            .unwrap();
        assert_eq!((own.current_streak, own.week_minutes), (1, 20));
    }

    #[test]
    fn generic_merit_counts_the_day_not_the_minutes() {
        let (mut s, _, kid) = household();
        log(&mut s, kid, "meditation", "2026-10-18", Some("generic"));
        let p = s
            .profile(
                Viewer {
                    id: 0,
                    admin: false,
                    outsider: false,
                },
                kid,
                Some(NOON),
            )
            .unwrap();
        assert_eq!((p.current_streak, p.week_minutes), (1, 0));
        assert_eq!(p.week[0].category, "other");
    }

    #[test]
    fn feed_is_newest_first_and_pages() {
        let (mut s, dad, kid) = household();
        log(&mut s, kid, "meditation", "2026-10-16", None);
        log(&mut s, dad, "chanting", "2026-10-18", None);
        log(&mut s, kid, "anger", "2026-10-18", None);
        log(&mut s, kid, "reading", "2026-10-17", None);
        let anyone = Viewer {
            id: 0,
            admin: false,
            outsider: false,
        };
        let first = s.feed(anyone, 0, 2);
        let names: Vec<_> = first.entries.iter().map(|e| e.kind_name.as_str()).collect();
        assert_eq!(
            names,
            ["Chanting", "Reading Dharma"],
            "the private demerit is skipped"
        );
        assert!(first.more);
        let rest = s.feed(anyone, 2, 2);
        assert_eq!(rest.entries.len(), 1);
        assert!(!rest.more);
    }

    /// Sprint 5 exit: stats from day totals equal stats from the entries
    /// themselves, for every kind of viewer, with or without a card.
    #[test]
    fn totals_beyond_the_window_match_the_entries() {
        let kinds = [
            "meditation",
            "chanting",
            "vegetarian",
            "repentance",
            "anger",
            "daimoku",
        ];
        let visibilities = [None, Some("public"), Some("generic"), Some("private")];
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut next = move |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n as u64) as usize
        };
        for journaled in [false, true] {
            let (mut s, dad, kid) = household();
            let today = Date::parse("2026-10-18").unwrap();
            // Entries are made with a far-back window, then the window rolls.
            s.window_start = i64::MIN;
            for _ in 0..400 {
                let input = EntryInput {
                    person: kid,
                    kind: kinds[next(kinds.len())].into(),
                    date: Some(today.add_days(-(next(120) as i64)).iso()),
                    visibility: visibilities[next(visibilities.len())].map(str::to_string),
                    amount: (next(2) == 0).then(|| 1 + next(60) as u32),
                    ..Default::default()
                };
                // Bypass the backfill limit: these are history, as an import would add.
                let id = s.next_entry;
                s.next_entry += 1;
                let kind = s.kind(&input.kind).unwrap();
                s.entries.push(crate::messages::StoredEntry {
                    id,
                    person: kid,
                    kind: kind.key.clone(),
                    date: input.date.clone().unwrap(),
                    amount: if kind.unit == "once" {
                        0
                    } else {
                        input.amount.unwrap_or(kind.default_amount)
                    },
                    visibility: input.visibility.clone().unwrap_or(kind.visibility.clone()),
                    rev: 1,
                    ..Default::default()
                });
            }
            let viewers = [
                Viewer {
                    id: kid,
                    admin: false,
                    outsider: false,
                },
                Viewer {
                    id: dad,
                    admin: false,
                    outsider: false,
                },
                Viewer {
                    id: dad,
                    admin: true,
                    outsider: false,
                },
                Viewer {
                    id: 0,
                    admin: false,
                    outsider: false,
                },
            ];
            let before: Vec<_> = viewers
                .iter()
                .map(|v| s.profile(*v, kid, Some(NOON)).unwrap())
                .collect();
            s.journaled = journaled;
            s.roll_window(today);
            assert!(s.entries.len() < 400, "some entries left the window");
            for (v, b) in viewers.iter().zip(&before) {
                let a = s.profile(*v, kid, Some(NOON)).unwrap();
                assert_eq!(
                    (a.current_streak, a.longest_streak),
                    (b.current_streak, b.longest_streak),
                    "{v:?}"
                );
                assert_eq!(a.heatmap, b.heatmap, "{v:?} journaled={journaled}");
                assert_eq!(a.week_minutes, b.week_minutes, "{v:?}");
            }
            // Without a card the totals are in the snapshot, and survive it.
            if !journaled {
                let back = State::from_stored(s.to_stored(), s.limits.clone());
                let mut back = back;
                back.window_start = s.window_start;
                for (v, b) in viewers.iter().zip(&before) {
                    assert_eq!(
                        back.profile(*v, kid, Some(NOON)).unwrap().heatmap,
                        b.heatmap,
                        "{v:?} after reload"
                    );
                }
            }
        }
    }

    #[test]
    fn rejoicing_once_in_someone_elses_merit() {
        let (mut s, dad, kid) = household();
        log(&mut s, kid, "meditation", "2026-10-18", None);
        let as_dad = Actor::Person {
            id: dad,
            admin: true,
            guest: false,
        };
        s.rejoice(as_dad, 1, Some(NOON)).unwrap();
        assert_eq!(
            s.rejoice(as_dad, 1, Some(NOON)).unwrap_err().code,
            "already"
        );
        let view = s
            .entry_view(
                Viewer {
                    id: dad,
                    admin: true,
                    outsider: false,
                },
                &s.entries[0],
            )
            .unwrap();
        assert_eq!((view.rejoices, view.rejoiced_by_me), (1, true));
        let mine = s.entries.iter().find(|e| e.person == dad).unwrap();
        assert_eq!(
            (mine.kind.as_str(), mine.note.as_str()),
            ("rejoicing", "In Kid's meditation")
        );
        // Not in your own, and not in a demerit.
        let as_kid = Actor::Person {
            id: kid,
            admin: false,
            guest: false,
        };
        assert_eq!(
            s.rejoice(as_kid, 1, Some(NOON)).unwrap_err().code,
            "cannot_rejoice"
        );
        log(&mut s, kid, "anger", "2026-10-18", Some("public"));
        let anger = s.entries.iter().find(|e| e.kind == "anger").unwrap().id;
        assert_eq!(
            s.rejoice(as_dad, anger, Some(NOON)).unwrap_err().code,
            "cannot_rejoice"
        );
    }
}
