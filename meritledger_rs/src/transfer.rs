//! Export, import, CSV and backup rotation (`sprints/sprint05.md`,
//! `docs/EXPORT_FORMAT.md`).
//!
//! An export is one JSON document ([`ExportDoc`]): people (names and roles
//! only: no PINs, no email), custom kinds, every entry, and the day totals
//! of entries the board no longer has in detail. Nightly backups on the SD
//! card are the same document.
//!
//! Import merges a document into this board: people are matched by name,
//! custom kinds by name and owner, and an entry already here (same person,
//! practice, day, time, amount and note) is skipped, so importing the same
//! file twice changes nothing.
use crate::catalog;
use crate::journal::Journal;
use crate::messages::{
    ExportDoc, ExportPerson, ImportReport, StoredDay, StoredEntry, StoredKind, StoredPost,
    StoredPracticeDoc, StoredText, StoredVow,
};
use crate::state::{State, MEMBER};
use crate::tz::Date;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io;

pub const FORMAT: &str = "meritledger-export";
pub const VERSION: u32 = 1;
/// Daily backups kept, then one a month for this many months.
pub const KEEP_DAILY: usize = 14;
pub const KEEP_MONTHLY: usize = 24;

/// What makes two entries the same for import.
pub type Fingerprint = (u32, String, String, u64, u32, String);

pub fn fingerprint(e: &StoredEntry) -> Fingerprint {
    (
        e.person,
        e.kind.clone(),
        e.date.clone(),
        e.created,
        e.amount,
        e.note.clone(),
    )
}

/// Every entry the board has: the journal's final versions with a card,
/// the snapshot window without one.
pub fn all_entries(state: &State, journal: Option<&Journal>) -> io::Result<Vec<StoredEntry>> {
    match journal {
        Some(j) if state.journaled => {
            let mut out = Vec::new();
            j.for_each(|e| out.push(e))?;
            out.sort_by_key(|e| e.id);
            Ok(out)
        }
        _ => Ok(state.entries.clone()),
    }
}

/// One person's entries, optionally within `from..=to`: from the journal
/// with a card (decoding only those), else from the window.
pub fn entries_of(
    state: &State,
    journal: Option<&Journal>,
    person: u32,
    range: Option<(Date, Date)>,
) -> io::Result<Vec<StoredEntry>> {
    let (lo, hi) = range.map_or((i64::MIN, i64::MAX), |(a, b)| (a.days(), b.days()));
    match journal {
        Some(j) if state.journaled => {
            let mut out = Vec::new();
            j.for_each_where(
                |h| h.person == person && i64::from(h.day) >= lo && i64::from(h.day) <= hi,
                |e| out.push(e),
            )?;
            Ok(out)
        }
        _ => Ok(state
            .entries
            .iter()
            .filter(|e| e.person == person)
            .filter(|e| Date::parse(&e.date).is_some_and(|d| d.days() >= lo && d.days() <= hi))
            .cloned()
            .collect()),
    }
}

/// Where `export` reads post and text bodies from (the card or the snapshot).
pub struct Bodies<'a> {
    pub post: &'a dyn Fn(&StoredPost) -> String,
    pub text: &'a dyn Fn(&StoredText) -> String,
}

/// Everything, with post and text bodies.
pub fn export(
    state: &State,
    journal: Option<&Journal>,
    now: u64,
    bodies: &Bodies<'_>,
) -> io::Result<ExportDoc> {
    Ok(ExportDoc {
        format: FORMAT.into(),
        version: VERSION,
        exported_at: now,
        timezone: state.timezone.clone(),
        people: state
            .people
            .iter()
            .map(|p| ExportPerson {
                id: p.id,
                display_name: p.display_name.clone(),
                role: p.role.clone(),
            })
            .collect(),
        kinds: state.kinds.clone(),
        entries: all_entries(state, journal)?,
        archived_days: state.archived_days.clone(),
        vows: state.vows.clone(),
        posts: state
            .posts
            .iter()
            .map(|p| StoredPost {
                body: (bodies.post)(p),
                on_card: false,
                ..p.clone()
            })
            .collect(),
        precepts: state.precepts.clone(),
        texts: state
            .texts
            .iter()
            .map(|t| StoredText {
                body: (bodies.text)(t),
                on_card: false,
                ..t.clone()
            })
            .collect(),
        practices: state.practices.clone(),
        readings: state.readings.clone(),
    })
}

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// One person's entries as CSV, oldest first, as `viewer` may see them.
pub fn csv(
    state: &State,
    entries: &[StoredEntry],
    viewer: crate::ledger::Viewer,
    person: u32,
) -> String {
    let mut rows: Vec<_> = entries
        .iter()
        .filter(|e| e.person == person)
        .filter_map(|e| state.entry_view(viewer, e))
        .collect();
    rows.sort_by(|a, b| {
        a.date
            .cmp(&b.date)
            .then(a.created.cmp(&b.created))
            .then(a.id.cmp(&b.id))
    });
    let mut out =
        String::from("date,practice,category,amount,unit,visibility,note,recorded_by\r\n");
    for r in rows {
        let amount = if r.unit == "once" {
            String::new()
        } else {
            r.amount.to_string()
        };
        let fields = [
            r.date.as_str(),
            &r.kind_name,
            &r.category,
            &amount,
            &r.unit,
            &r.visibility,
            &r.note,
            &r.recorded_by_name,
        ];
        let line: Vec<String> = fields.iter().map(|f| csv_field(f)).collect();
        out.push_str(&line.join(","));
        out.push_str("\r\n");
    }
    out
}

/// Backup files (`YYYY-MM-DD.json`) to delete: keep the newest
/// [`KEEP_DAILY`], and the oldest of each month for the newest
/// [`KEEP_MONTHLY`] months.
pub fn backups_to_delete(names: &[String]) -> Vec<String> {
    let mut dated: Vec<&String> = names
        .iter()
        .filter(|n| n.len() == 15 && n.ends_with(".json") && Date::parse(&n[..10]).is_some())
        .collect();
    dated.sort();
    let mut keep: HashSet<&String> = dated.iter().rev().take(KEEP_DAILY).copied().collect();
    let mut first_of_month: BTreeMap<&str, &String> = BTreeMap::new();
    for n in &dated {
        first_of_month.entry(&n[..7]).or_insert(n);
    }
    keep.extend(first_of_month.values().rev().take(KEEP_MONTHLY).copied());
    dated
        .into_iter()
        .filter(|n| !keep.contains(n))
        .cloned()
        .collect()
}

impl State {
    /// Merges `doc` into this state. Entries inside the window join
    /// `self.entries`; older ones are returned for the journal (with a
    /// card) or folded into `archived_days` (without). With `apply` false
    /// nothing changes and the report says what would happen.
    pub fn import(
        &mut self,
        doc: &ExportDoc,
        existing: &HashSet<Fingerprint>,
        apply: bool,
        now: Option<u64>,
    ) -> Result<(ImportReport, Vec<StoredEntry>), String> {
        if doc.format != FORMAT || doc.version > VERSION {
            return Err(format!(
                "Not a Merit Ledger export (format {:?}, version {})",
                doc.format, doc.version
            ));
        }
        let mut report = ImportReport {
            dry_run: !apply,
            ..Default::default()
        };
        let mut planned_people = self.people.len();
        // People, by name.
        let mut person_map: HashMap<u32, u32> = HashMap::new();
        let mut next_new = self.next_person;
        for p in &doc.people {
            let found = self
                .people
                .iter()
                .find(|q| q.display_name.to_lowercase() == p.display_name.to_lowercase())
                .map(|q| q.id);
            match found {
                Some(id) => {
                    person_map.insert(p.id, id);
                    report.people_matched += 1;
                }
                None if planned_people < self.limits.people => {
                    let id = if apply {
                        self.add_person(&p.display_name, MEMBER, "", "", now.unwrap_or(0))
                            .map_err(|e| e.message)?
                    } else {
                        next_new += 1;
                        next_new - 1
                    };
                    planned_people += 1;
                    person_map.insert(p.id, id);
                    report.people_created += 1;
                }
                None => report.notes.push(format!(
                    "{} not added: the board is full; their entries are skipped",
                    p.display_name
                )),
            }
        }
        if report.people_created > 0 {
            report
                .notes
                .push("New people have no PIN yet: give them one on the Household page".into());
        }
        // Custom kinds, by name and owner.
        let mut kind_map: HashMap<String, String> = HashMap::new();
        let mut next_kind = self.next_kind;
        for k in &doc.kinds {
            let owner = if k.owner == 0 {
                Some(0)
            } else {
                person_map.get(&k.owner).copied()
            };
            let Some(owner) = owner else { continue };
            let found = self
                .kinds
                .iter()
                .find(|q| q.owner == owner && q.name.to_lowercase() == k.name.to_lowercase())
                .map(|q| q.key.clone());
            let key = match found {
                Some(key) => key,
                None => {
                    let key = format!("c{next_kind}");
                    next_kind += 1;
                    if apply {
                        self.kinds.push(StoredKind {
                            key: key.clone(),
                            owner,
                            ..k.clone()
                        });
                        self.next_kind = next_kind;
                    }
                    report.kinds_created += 1;
                    key
                }
            };
            kind_map.insert(k.key.clone(), key);
        }
        // Entries.
        let today = self.today(now);
        let window_start = today.map_or(self.window_start, |t| {
            t.days() - crate::ledger::BACKFILL_DAYS
        });
        let mut seen = existing.clone();
        let mut older = Vec::new();
        for e in &doc.entries {
            let Some(&person) = person_map.get(&e.person) else {
                report.entries_skipped += 1;
                continue;
            };
            let kind = if catalog::builtin(&e.kind).is_some() {
                e.kind.clone()
            } else if let Some(k) = kind_map.get(&e.kind) {
                k.clone()
            } else {
                report.entries_skipped += 1;
                continue;
            };
            let mapped = StoredEntry {
                id: 0,
                person,
                kind,
                recorded_by: person_map.get(&e.recorded_by).copied().unwrap_or(0),
                rejoiced_by: e
                    .rejoiced_by
                    .iter()
                    .filter_map(|r| person_map.get(r).copied())
                    .collect(),
                deleted: false,
                ..e.clone()
            };
            if !seen.insert(fingerprint(&mapped)) {
                report.entries_skipped += 1;
                continue;
            }
            let Some(date) = Date::parse(&mapped.date) else {
                report.entries_skipped += 1;
                continue;
            };
            let in_window = date.days() >= window_start;
            if !in_window && !self.journaled {
                report.entries_summarized += 1;
                if apply {
                    let t = self.tally_of(&mapped);
                    let key = (person, date.days());
                    self.history.entry(key).or_default().add(t);
                    let total = self.history[&key];
                    self.archived_days
                        .retain(|d| (d.person, i64::from(d.day)) != key);
                    self.archived_days.push(total.to_stored(key.0, key.1));
                }
                continue;
            }
            report.entries_added += 1;
            if !apply {
                continue;
            }
            let id = self.next_entry;
            self.next_entry += 1;
            let entry = StoredEntry { id, ..mapped };
            if in_window {
                self.entries.push(entry);
            } else {
                self.fold_into_history([&entry]);
                older.push(entry);
            }
        }
        if apply && self.journaled && self.next_entry > self.entry_lease {
            self.entry_lease = self.next_entry + 1_000;
        }
        // Day totals the export only had as totals.
        for d in &doc.archived_days {
            let Some(&person) = person_map.get(&d.person) else {
                continue;
            };
            let key = (person, i64::from(d.day));
            if self
                .archived_days
                .iter()
                .any(|a| (a.person, i64::from(a.day)) == key)
            {
                continue;
            }
            if apply {
                let tally = crate::stats::Tally::from_stored(d);
                self.history.entry(key).or_default().add(tally);
                self.archived_days.push(StoredDay {
                    person,
                    ..d.clone()
                });
            }
        }
        self.import_vows(doc, &person_map, apply, &mut report);
        self.import_practices(doc, &person_map, &kind_map, apply, &mut report);
        // Reading lists: a book someone already has (same work, or title) stays.
        let mut next_reading = self.next_reading;
        for r in &doc.readings {
            let Some(&person) = person_map.get(&r.person) else {
                continue;
            };
            let have = self.readings.iter().any(|q| {
                q.person == person
                    && ((!r.work.is_empty() && q.work == r.work) || q.title == r.title)
            });
            if have || self.readings.len() + report.readings_added as usize >= self.limits.readings
            {
                continue;
            }
            report.readings_added += 1;
            if apply {
                self.readings.push(crate::messages::StoredReading {
                    id: next_reading,
                    person,
                    ..r.clone()
                });
            }
            next_reading += 1;
        }
        if apply {
            self.next_reading = next_reading;
        }
        if self.entries.len() > self.limits.entries {
            return Err(format!(
                "That would put {} entries in the recent window; the board holds {}",
                self.entries.len(),
                self.limits.entries
            ));
        }
        Ok((report, older))
    }

    /// Household texts (matched by title) and practices (by owner and
    /// title). Bodies go into the snapshot; `api.rs` moves them to the
    /// card. Steps that use an imported text or custom kind are remapped.
    fn import_practices(
        &mut self,
        doc: &ExportDoc,
        person_map: &HashMap<u32, u32>,
        kind_map: &HashMap<String, String>,
        apply: bool,
        report: &mut ImportReport,
    ) {
        let mut text_map: HashMap<u32, u32> = HashMap::new();
        let mut next_text = self.next_text;
        for t in &doc.texts {
            if let Some(found) = self.texts.iter().find(|q| q.title == t.title) {
                text_map.insert(t.id, found.id);
                continue;
            }
            if self.texts.len() + report.texts_added as usize >= self.limits.texts {
                report
                    .notes
                    .push("Some texts not added: the board holds no more".into());
                break;
            }
            report.texts_added += 1;
            text_map.insert(t.id, next_text);
            if apply {
                self.texts.push(StoredText {
                    id: next_text,
                    owner: person_map.get(&t.owner).copied().unwrap_or(0),
                    on_card: false,
                    ..t.clone()
                });
            }
            next_text += 1;
        }
        if apply {
            self.next_text = next_text;
        }
        let mut next_practice = self.next_practice;
        for p in &doc.practices {
            let Some(&owner) = person_map.get(&p.owner) else {
                continue;
            };
            if self
                .practices
                .iter()
                .any(|q| q.owner == owner && q.title == p.title)
            {
                continue;
            }
            if self.practices.len() + report.practices_added as usize >= self.limits.practices {
                report
                    .notes
                    .push("Some practices not added: the board holds no more".into());
                break;
            }
            report.practices_added += 1;
            if apply {
                let mut doc = StoredPracticeDoc {
                    id: next_practice,
                    owner,
                    ..p.clone()
                };
                for e in &mut doc.elements {
                    if let Some(old) = crate::practice_docs::household_id(&e.text) {
                        e.text = text_map
                            .get(&old)
                            .map_or_else(String::new, |new| format!("h{new}"));
                    }
                    if let Some(new) = kind_map.get(&e.logs_as) {
                        e.logs_as = new.clone();
                    }
                }
                self.practices.push(doc);
            }
            next_practice += 1;
        }
        if apply {
            self.next_practice = next_practice;
        }
    }

    /// Vows (matched by person, text and start), their posts (bodies into
    /// the snapshot; `api.rs` moves them to the card), and precepts taken
    /// (a set someone already has is left alone). Repairs keep their event
    /// but not the link to the repentance entry.
    fn import_vows(
        &mut self,
        doc: &ExportDoc,
        person_map: &HashMap<u32, u32>,
        apply: bool,
        report: &mut ImportReport,
    ) {
        let mut vow_map: HashMap<u32, u32> = HashMap::new();
        let mut next_vow = self.next_vow;
        for v in &doc.vows {
            let Some(&person) = person_map.get(&v.person) else {
                continue;
            };
            if let Some(found) = self
                .vows
                .iter()
                .find(|q| q.person == person && q.text == v.text && q.started == v.started)
            {
                vow_map.insert(v.id, found.id);
                continue;
            }
            if self.vows.len() + report.vows_added as usize >= self.limits.vows {
                report
                    .notes
                    .push("Some vows not added: the board holds no more".into());
                break;
            }
            report.vows_added += 1;
            vow_map.insert(v.id, next_vow);
            if apply {
                let mut vow = StoredVow {
                    id: next_vow,
                    person,
                    ..v.clone()
                };
                for e in &mut vow.events {
                    e.entry = 0;
                }
                self.vows.push(vow);
            }
            next_vow += 1;
        }
        if apply {
            self.next_vow = next_vow;
        }
        let mut next_post = self.next_post;
        for p in &doc.posts {
            let (Some(&vow), Some(&person)) = (vow_map.get(&p.vow), person_map.get(&p.person))
            else {
                continue;
            };
            if self
                .posts
                .iter()
                .any(|q| q.vow == vow && q.title == p.title && q.date == p.date)
            {
                continue;
            }
            if self.posts.len() + report.posts_added as usize >= self.limits.posts {
                report
                    .notes
                    .push("Some posts not added: the board holds no more".into());
                break;
            }
            report.posts_added += 1;
            if apply {
                self.posts.push(StoredPost {
                    id: next_post,
                    vow,
                    person,
                    on_card: false,
                    ..p.clone()
                });
            }
            next_post += 1;
        }
        if apply {
            self.next_post = next_post;
        }
        for p in &doc.precepts {
            let Some(&person) = person_map.get(&p.person) else {
                continue;
            };
            if crate::precepts::set(&p.set).is_none()
                || self
                    .precepts
                    .iter()
                    .any(|q| q.person == person && q.set == p.set)
            {
                continue;
            }
            report.precepts_added += 1;
            if apply {
                self.precepts.push(crate::messages::StoredPrecepts {
                    person,
                    ..p.clone()
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn rotation_keeps_two_weeks_and_a_monthly() {
        // 40 days of backups from 2026-09-10 to 2026-10-19, plus older monthlies.
        let mut all = names(&[
            "2026-01-01.json",
            "2026-02-03.json",
            "2026-02-04.json",
            "notes.txt",
        ]);
        let start = Date::parse("2026-09-10").unwrap();
        for i in 0..40 {
            all.push(format!("{}.json", start.add_days(i).iso()));
        }
        let gone = backups_to_delete(&all);
        assert!(!gone.contains(&"2026-10-19.json".to_string()));
        assert!(
            !gone.contains(&"2026-10-06.json".to_string()),
            "14th newest"
        );
        assert!(gone.contains(&"2026-10-05.json".to_string()));
        assert!(
            !gone.contains(&"2026-09-10.json".to_string()),
            "September's first"
        );
        assert!(
            !gone.contains(&"2026-10-01.json".to_string()),
            "October's first"
        );
        assert!(!gone.contains(&"2026-02-03.json".to_string()));
        assert!(gone.contains(&"2026-02-04.json".to_string()));
        assert!(
            !gone.contains(&"notes.txt".to_string()),
            "not a backup: left alone"
        );
    }

    #[test]
    fn csv_escapes() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("a, b"), "\"a, b\"");
        assert_eq!(csv_field("say \"om\""), "\"say \"\"om\"\"\"");
    }
}
