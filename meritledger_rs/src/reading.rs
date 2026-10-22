//! Reading lists (`sprints/sprint12.md`, `spec/domain.md` § Books).
//!
//! - The phone searches Open Library itself (it allows any origin), so the
//!   board makes no outbound calls for books and needs no key: it stores
//!   only what someone saves.
//! - A reading session is an ordinary `reading` entry, linked to the book
//!   (`StoredEntry::reading`), so it counts in stats as minutes of reading.
//! - Public books show on the person's profile and the household's shelf.
use crate::ledger::{Actor, Viewer};
use crate::messages::{
    EntryInput, ReadingInput, ReadingLog, ReadingUpdate, ReadingView, StoredReading,
};
use crate::state::State;
use miniframework::ApiError;

fn check_status(s: &str) -> Result<String, ApiError> {
    match s {
        "" => Ok("want".into()),
        "want" | "reading" | "done" => Ok(s.into()),
        _ => Err(ApiError::bad_request("status: want, reading or done")),
    }
}

fn check_visibility(v: &str) -> Result<String, ApiError> {
    match v {
        "" | "public" => Ok("public".into()),
        "private" => Ok("private".into()),
        _ => Err(ApiError::bad_request("visibility: public or private")),
    }
}

/// An Open Library work key: `OL` digits `W` ("" for a typed-in book).
pub fn is_work(key: &str) -> bool {
    key.strip_prefix("OL")
        .and_then(|k| k.strip_suffix('W'))
        .is_some_and(|d| (1..=12).contains(&d.len()) && d.chars().all(|c| c.is_ascii_digit()))
}

fn clean(raw: &str, max: usize) -> String {
    let s = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    s.chars().take(max).collect()
}

impl State {
    fn reading_view(&self, viewer: Viewer, r: &StoredReading) -> Option<ReadingView> {
        let own = viewer.admin || (viewer.id != 0 && viewer.id == r.person);
        if r.visibility == "private" && !own {
            return None;
        }
        let minutes = self
            .entries
            .iter()
            .filter(|e| e.reading == r.id && e.person == r.person)
            .map(|e| e.amount)
            .sum();
        Some(ReadingView {
            id: r.id,
            person: r.person,
            person_name: self.name_of(r.person),
            work: r.work.clone(),
            title: r.title.clone(),
            author: r.author.clone(),
            cover: r.cover,
            status: r.status.clone(),
            page: r.page,
            pages: r.pages,
            started: r.started.clone(),
            finished: r.finished.clone(),
            visibility: r.visibility.clone(),
            rev: r.rev,
            year: r.year,
            minutes,
            can_edit: own,
        })
    }

    /// `person`'s list as `viewer` sees it: reading, then want, then done
    /// (latest first).
    pub fn readings_of(&self, viewer: Viewer, person: u32) -> Vec<ReadingView> {
        let rank = |s: &str| match s {
            "reading" => 0,
            "want" => 1,
            _ => 2,
        };
        let mut list: Vec<&StoredReading> = self
            .readings
            .iter()
            .filter(|r| r.person == person)
            .collect();
        list.sort_by(|a, b| {
            rank(&a.status)
                .cmp(&rank(&b.status))
                .then(b.updated.cmp(&a.updated))
        });
        list.into_iter()
            .filter_map(|r| self.reading_view(viewer, r))
            .collect()
    }

    /// What the household is reading now (public books only, for others).
    pub fn shelf(&self, viewer: Viewer) -> Vec<ReadingView> {
        let mut list: Vec<&StoredReading> = self
            .readings
            .iter()
            .filter(|r| r.status == "reading")
            .collect();
        list.sort_by_key(|r| std::cmp::Reverse(r.updated));
        list.into_iter()
            .filter_map(|r| self.reading_view(viewer, r))
            .collect()
    }

    fn reading_index(&self, actor: Actor, id: u32) -> Result<usize, ApiError> {
        let Actor::Person { id: me, admin, .. } = actor else {
            return Err(ApiError::new(
                403,
                "forbidden",
                "The board can't change reading lists",
            ));
        };
        let at = self
            .readings
            .iter()
            .position(|r| r.id == id)
            .ok_or_else(|| ApiError::not_found("No such book on a list"))?;
        if self.readings[at].person != me && !admin {
            return Err(ApiError::new(
                403,
                "forbidden",
                "That's someone else's reading list",
            ));
        }
        Ok(at)
    }

    pub fn add_reading(
        &mut self,
        actor: Actor,
        input: &ReadingInput,
        now: Option<u64>,
    ) -> Result<u32, ApiError> {
        let Actor::Person { id: person, .. } = actor else {
            return Err(ApiError::new(
                403,
                "forbidden",
                "The board can't change reading lists",
            ));
        };
        let title = clean(&input.title, 200);
        if title.is_empty() {
            return Err(ApiError::bad_request("A book needs a title"));
        }
        let work = input.work.trim().trim_start_matches("/works/").to_string();
        if !work.is_empty() && !is_work(&work) {
            return Err(ApiError::bad_request(
                "An Open Library work looks like OL45804W",
            ));
        }
        if !work.is_empty()
            && self
                .readings
                .iter()
                .any(|r| r.person == person && r.work == work)
        {
            return Err(ApiError::new(
                409,
                "already",
                "That book is already on your list",
            ));
        }
        if input.pages > 100_000 {
            return Err(ApiError::bad_request("Pages: up to 100,000"));
        }
        let status = check_status(&input.status)?;
        let visibility = check_visibility(&input.visibility)?;
        if self.readings.len() >= self.limits.readings {
            return Err(ApiError::new(
                409,
                "full",
                format!(
                    "The board holds {} books on reading lists",
                    self.limits.readings
                ),
            ));
        }
        let today = self.today(now).map(|d| d.iso()).unwrap_or_default();
        let id = self.next_reading;
        self.next_reading += 1;
        self.readings.push(StoredReading {
            id,
            person,
            work,
            title,
            author: clean(&input.author, 120),
            cover: input.cover,
            started: if status == "want" {
                String::new()
            } else {
                today.clone()
            },
            finished: if status == "done" {
                today
            } else {
                String::new()
            },
            status,
            page: 0,
            pages: input.pages,
            updated: now.unwrap_or(0),
            visibility,
            rev: 1,
            year: input.year,
        });
        Ok(id)
    }

    pub fn update_reading(
        &mut self,
        actor: Actor,
        id: u32,
        change: &ReadingUpdate,
        now: Option<u64>,
    ) -> Result<(), ApiError> {
        let at = self.reading_index(actor, id)?;
        if self.readings[at].rev != change.rev {
            return Err(ApiError::new(
                409,
                "stale",
                "This book changed; reload and try again",
            ));
        }
        let status = change.status.as_deref().map(check_status).transpose()?;
        let visibility = change
            .visibility
            .as_deref()
            .map(check_visibility)
            .transpose()?;
        let today = self.today(now).map(|d| d.iso()).unwrap_or_default();
        let r = &mut self.readings[at];
        if let Some(pages) = change.pages {
            if pages > 100_000 {
                return Err(ApiError::bad_request("Pages: up to 100,000"));
            }
            r.pages = pages;
        }
        if let Some(page) = change.page {
            if r.pages > 0 && page > r.pages {
                return Err(ApiError::bad_request(format!(
                    "The book has {} pages",
                    r.pages
                )));
            }
            r.page = page;
        }
        if let Some(s) = status {
            if s != "want" && r.started.is_empty() {
                r.started = today.clone();
            }
            r.finished = if s == "done" { today } else { String::new() };
            r.status = s;
        }
        if let Some(v) = visibility {
            r.visibility = v;
        }
        r.rev += 1;
        r.updated = now.unwrap_or(r.updated);
        Ok(())
    }

    pub fn delete_reading(&mut self, actor: Actor, id: u32) -> Result<(), ApiError> {
        let at = self.reading_index(actor, id)?;
        self.readings.remove(at);
        Ok(())
    }

    /// A reading session: a `reading` entry for the book's person, linked to
    /// the book, and the page they got to. Returns the entry id.
    pub fn log_reading(
        &mut self,
        actor: Actor,
        id: u32,
        log: &ReadingLog,
        now: Option<u64>,
    ) -> Result<u32, ApiError> {
        let at = self.reading_index(actor, id)?;
        if !(1..=1440).contains(&log.minutes) {
            return Err(ApiError::bad_request("Minutes: 1 to 1440"));
        }
        let r = self.readings[at].clone();
        if log.page > 0 && r.pages > 0 && log.page > r.pages {
            return Err(ApiError::bad_request(format!(
                "The book has {} pages",
                r.pages
            )));
        }
        let pages = match (r.page, log.page) {
            (from, to) if to > from && from > 0 => format!(", pp. {from}–{to}"),
            (_, to) if to > 0 => format!(", to p. {to}"),
            _ => String::new(),
        };
        let mut note = format!("{}{pages}", r.title);
        if !log.note.trim().is_empty() {
            note.push_str(" · ");
            note.push_str(log.note.trim());
        }
        let input = EntryInput {
            person: r.person,
            kind: "reading".into(),
            amount: Some(log.minutes),
            note: note.chars().take(500).collect(),
            visibility: Some(if r.visibility == "private" {
                "private".into()
            } else {
                "public".into()
            }),
            source: Some("manual".into()),
            ..Default::default()
        };
        let entry = self.record(actor, &input, now)?;
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == entry) {
            e.reading = id;
        }
        let today = self.today(now).map(|d| d.iso()).unwrap_or_default();
        let r = &mut self.readings[at];
        if log.page > 0 {
            r.page = log.page;
        }
        if r.status == "want" {
            r.status = "reading".into();
            r.started = today;
        }
        r.rev += 1;
        r.updated = now.unwrap_or(r.updated);
        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Limits, MEMBER};

    const NOW: Option<u64> = Some(1_792_000_000);

    fn state() -> (State, u32, u32) {
        let mut s = State::new(Limits {
            pin_rounds: 2,
            ..Limits::board()
        });
        let dad = s.setup("Dad", "1234", 0).unwrap();
        let kid = s.add_person("Kid", MEMBER, "1111", "", 0).unwrap();
        (s, dad, kid)
    }

    fn me(id: u32) -> Actor {
        Actor::Person {
            id,
            admin: false,
            guest: false,
        }
    }

    fn book() -> ReadingInput {
        ReadingInput {
            work: "/works/OL263645W".into(),
            title: "The Heart of Understanding".into(),
            author: "Thích Nhất Hạnh".into(),
            cover: 715880,
            pages: 54,
            year: 1987,
            ..Default::default()
        }
    }

    #[test]
    fn a_book_from_want_to_done() {
        let (mut s, dad, kid) = state();
        let id = s.add_reading(me(dad), &book(), NOW).unwrap();
        assert_eq!(s.readings[0].work, "OL263645W");
        assert_eq!(
            s.add_reading(me(dad), &book(), NOW).unwrap_err().code,
            "already"
        );
        // A session: 20 minutes, to page 18. Want becomes reading.
        let entry = s
            .log_reading(
                me(dad),
                id,
                &ReadingLog {
                    minutes: 20,
                    page: 18,
                    note: String::new(),
                },
                NOW,
            )
            .unwrap();
        let e = s.entries.iter().find(|e| e.id == entry).unwrap();
        assert_eq!((e.kind.as_str(), e.amount, e.reading), ("reading", 20, id));
        assert_eq!(e.note, "The Heart of Understanding, to p. 18");
        let again = s
            .log_reading(
                me(dad),
                id,
                &ReadingLog {
                    minutes: 15,
                    page: 40,
                    note: "on emptiness".into(),
                },
                NOW,
            )
            .unwrap();
        assert_eq!(
            s.entries.iter().find(|e| e.id == again).unwrap().note,
            "The Heart of Understanding, pp. 18–40 · on emptiness"
        );
        let v = &s.readings_of(
            Viewer {
                id: kid,
                admin: false,
                outsider: false,
            },
            dad,
        )[0];
        assert_eq!((v.status.as_str(), v.page, v.minutes), ("reading", 40, 35));
        assert_eq!(
            s.shelf(Viewer {
                id: kid,
                admin: false,
                outsider: false
            })
            .len(),
            1
        );
        // Not past the last page; done is a status.
        let past = ReadingLog {
            minutes: 5,
            page: 99,
            note: String::new(),
        };
        assert_eq!(
            s.log_reading(me(dad), id, &past, NOW).unwrap_err().status,
            400
        );
        let done = ReadingUpdate {
            rev: 3,
            status: Some("done".into()),
            ..Default::default()
        };
        s.update_reading(me(dad), id, &done, NOW).unwrap();
        assert!(!s.readings[0].finished.is_empty());
        assert!(s
            .shelf(Viewer {
                id: kid,
                admin: false,
                outsider: false
            })
            .is_empty());
    }

    #[test]
    fn lists_are_their_persons() {
        let (mut s, dad, kid) = state();
        let id = s.add_reading(me(dad), &book(), NOW).unwrap();
        let log = ReadingLog {
            minutes: 5,
            ..Default::default()
        };
        assert_eq!(
            s.log_reading(me(kid), id, &log, NOW).unwrap_err().status,
            403
        );
        assert_eq!(s.delete_reading(me(kid), id).unwrap_err().status, 403);
        let private = ReadingInput {
            work: "OL1W".into(),
            visibility: "private".into(),
            ..book()
        };
        s.add_reading(me(dad), &private, NOW).unwrap();
        assert_eq!(
            s.readings_of(
                Viewer {
                    id: kid,
                    admin: false,
                    outsider: false
                },
                dad
            )
            .len(),
            1
        );
        assert_eq!(
            s.readings_of(
                Viewer {
                    id: dad,
                    admin: false,
                    outsider: false
                },
                dad
            )
            .len(),
            2
        );
        for bad in ["OL1M", "works", "OLW"] {
            let b = ReadingInput {
                work: bad.into(),
                ..book()
            };
            assert_eq!(
                s.add_reading(me(dad), &b, NOW).unwrap_err().status,
                400,
                "{bad}"
            );
        }
        let typed = ReadingInput {
            work: String::new(),
            title: "A zine from the temple".into(),
            ..Default::default()
        };
        s.add_reading(me(dad), &typed, NOW).unwrap();
    }
}
