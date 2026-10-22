//! The precept sets people can record taking (`catalog/precepts.json`):
//! the Five, the novice Ten, the Upāsaka and the Brahmajāla bodhisattva
//! precepts, in their Chinese Buddhist versions. Every item cites the
//! classical source by CBETA line, so a list on a profile can be traced
//! back and checked (`tools/precepts_check.py`; method and sources in
//! `docs/precepts/`). Set keys and item numbers are stored in people's
//! records, so never renumber; fix a rendering instead.
use crate::ledger::{check_visibility, Actor, Viewer};
use crate::messages::{Precept, PreceptSet, PreceptsInput, PreceptsView, StoredPrecepts};
use crate::state::State;
use crate::tz::Date;
use miniframework::wire::{self, Format};
use miniframework::ApiError;
use std::sync::OnceLock;

const NOTE_MAX: usize = 200;

const JSON: &str = include_str!("../catalog/precepts.json");

pub fn sets() -> &'static [PreceptSet] {
    static SETS: OnceLock<Vec<PreceptSet>> = OnceLock::new();
    SETS.get_or_init(|| {
        let catalog: crate::messages::PreceptCatalog = wire::decode(Format::Json, JSON.as_bytes())
            .expect("catalog/precepts.json is valid (tested)");
        catalog.sets
    })
}

pub fn set(key: &str) -> Option<&'static PreceptSet> {
    sets().iter().find(|s| s.key == key)
}

/// One precept: set key, `major`/`minor`, number.
pub fn item(set_key: &str, kind: &str, n: u32) -> Option<&'static Precept> {
    set(set_key)?
        .items
        .iter()
        .find(|i| i.kind == kind && i.n == n)
}

fn count(set: &PreceptSet, kind: &str) -> u32 {
    set.items.iter().filter(|i| i.kind == kind).count() as u32
}

/// Item numbers, sorted and deduplicated, all in `1..=max`.
fn checked_numbers(list: &[u32], max: u32, what: &str) -> Result<Vec<u32>, ApiError> {
    let mut out = list.to_vec();
    out.sort_unstable();
    out.dedup();
    if out.iter().any(|&n| n == 0 || n > max) {
        return Err(ApiError::bad_request(if max == 0 {
            format!("This set has no {} precepts", what.to_lowercase())
        } else {
            format!("{what} precepts are numbered 1 to {max}")
        }));
    }
    Ok(out)
}

impl State {
    pub fn has_taken(&self, person: u32, set: &str, kind: &str, n: u32) -> bool {
        self.precepts.iter().any(|p| {
            p.person == person
                && p.set == set
                && match kind {
                    "major" => p.majors.contains(&n),
                    "minor" => p.minors.contains(&n),
                    _ => false,
                }
        })
    }

    /// Records which precepts of `set_key` the actor has taken: any subset
    /// but never an unknown item, replacing what was there.
    pub fn set_precepts(
        &mut self,
        actor: Actor,
        set_key: &str,
        input: &PreceptsInput,
        now: Option<u64>,
    ) -> Result<(), ApiError> {
        let Actor::Person { id: person, .. } = actor else {
            return Err(ApiError::new(
                403,
                "forbidden",
                "The board can't record precepts",
            ));
        };
        let set = set(set_key).ok_or_else(|| ApiError::not_found("No such precept set"))?;
        let majors = checked_numbers(&input.majors, count(set, "major"), "Major")?;
        let minors = checked_numbers(&input.minors, count(set, "minor"), "Minor")?;
        if majors.is_empty() && minors.is_empty() {
            return Err(ApiError::bad_request(
                "Choose at least one precept (or remove the set)",
            ));
        }
        let date = if input.date.is_empty() {
            String::new()
        } else {
            let d = Date::parse(&input.date)
                .ok_or_else(|| ApiError::bad_request("Dates look like 2026-10-18"))?;
            if self.today(now).is_some_and(|t| d > t) {
                return Err(ApiError::bad_request("That date hasn't happened yet"));
            }
            d.iso()
        };
        let note = input.note.trim().to_string();
        if note.chars().count() > NOTE_MAX {
            return Err(ApiError::bad_request(format!(
                "Notes: at most {NOTE_MAX} characters"
            )));
        }
        let visibility = if input.visibility.is_empty() {
            "public".to_string()
        } else {
            check_visibility(&input.visibility)?
        };
        let record = StoredPrecepts {
            person,
            set: set.key.clone(),
            date,
            note,
            majors,
            minors,
            visibility,
        };
        match self
            .precepts
            .iter_mut()
            .find(|p| p.person == person && p.set == set_key)
        {
            Some(p) => *p = record,
            None => self.precepts.push(record),
        }
        Ok(())
    }

    /// "No longer holding": removes the record (vows made from it stay).
    pub fn remove_precepts(&mut self, actor: Actor, set_key: &str) -> Result<(), ApiError> {
        let Actor::Person { id: person, .. } = actor else {
            return Err(ApiError::new(
                403,
                "forbidden",
                "The board can't record precepts",
            ));
        };
        let before = self.precepts.len();
        self.precepts
            .retain(|p| !(p.person == person && p.set == set_key));
        if self.precepts.len() == before {
            return Err(ApiError::not_found("You haven't recorded that set"));
        }
        Ok(())
    }

    /// `person`'s precepts as `viewer` may see them, in catalog order.
    pub fn precepts_of(&self, viewer: Viewer, person: u32) -> Vec<PreceptsView> {
        let own = viewer.admin || (viewer.id != 0 && viewer.id == person);
        sets()
            .iter()
            .filter_map(|set| {
                let p = self
                    .precepts
                    .iter()
                    .find(|p| p.person == person && p.set == set.key)?;
                if p.visibility == "private" && !own {
                    return None;
                }
                let generic = !own && p.visibility == "generic";
                Some(PreceptsView {
                    person,
                    set: set.key.clone(),
                    title: set.title.clone(),
                    date: if generic {
                        String::new()
                    } else {
                        p.date.clone()
                    },
                    note: if generic {
                        String::new()
                    } else {
                        p.note.clone()
                    },
                    majors: if generic {
                        Vec::new()
                    } else {
                        p.majors.clone()
                    },
                    minors: if generic {
                        Vec::new()
                    } else {
                        p.minors.clone()
                    },
                    visibility: p.visibility.clone(),
                    total_majors: count(set, "major"),
                    total_minors: count(set, "minor"),
                    draft: set.provenance != "llm-draft-reviewed",
                    generic,
                })
            })
            .collect()
    }
}

/// A CBETA line id: canon letter(s), volume, `n`, work, `_p`, page, column, line.
#[cfg(test)]
fn is_line_id(s: &str) -> bool {
    let Some((work, line)) = s.split_once("_p") else {
        return false;
    };
    let Some((vol, num)) = work.split_once('n') else {
        return false;
    };
    let canon = vol.trim_end_matches(|c: char| c.is_ascii_digit());
    let b = line.as_bytes();
    !canon.is_empty()
        && canon.chars().all(|c| c.is_ascii_uppercase())
        && vol.len() > canon.len()
        && !num.is_empty()
        && num.chars().all(|c| c.is_ascii_alphanumeric())
        && b.len() == 7
        && b[..4].iter().all(u8::is_ascii_digit)
        && matches!(b[4], b'a'..=b'c')
        && b[5..].iter().all(u8::is_ascii_digit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sets_are_numbered_as_the_sources_number_them() {
        let counts = |key: &str| {
            let s = set(key).unwrap();
            let n = |kind: &str| s.items.iter().filter(|i| i.kind == kind).count();
            (n("major"), n("minor"))
        };
        assert_eq!(counts("five"), (5, 0));
        assert_eq!(counts("novice_ten"), (10, 0));
        assert_eq!(counts("upasaka"), (6, 28));
        assert_eq!(counts("bodhisattva"), (10, 48));
        assert_eq!(sets().len(), 4);
        // Their short Chinese titles are Zhiyi's (T1811), cited one by one.
        let b = set("bodhisattva").unwrap();
        assert!(b.items.iter().all(|i| i.label_cite.is_some()));
        for s in sets() {
            for kind in ["major", "minor"] {
                let ns: Vec<u32> = s
                    .items
                    .iter()
                    .filter(|i| i.kind == kind)
                    .map(|i| i.n)
                    .collect();
                assert_eq!(
                    ns,
                    (1..=ns.len() as u32).collect::<Vec<_>>(),
                    "{} {kind}",
                    s.key
                );
            }
        }
    }

    #[test]
    fn every_item_cites_its_source() {
        for s in sets() {
            assert!(s.key.chars().all(|c| c.is_ascii_lowercase() || c == '_'));
            assert!(!s.source.work.is_empty() && s.source.juan > 0, "{}", s.key);
            assert!(
                matches!(s.provenance.as_str(), "llm-draft" | "llm-draft-reviewed"),
                "{}",
                s.key
            );
            for i in &s.items {
                let at = format!("{} {} {}", s.key, i.kind, i.n);
                assert!(matches!(i.kind.as_str(), "major" | "minor"), "{at}");
                assert!(!i.title.is_empty() && !i.text.is_empty(), "{at}");
                assert!(!i.label_zh.is_empty() && !i.quote.is_empty(), "{at}");
                assert!(!i.cite.is_empty(), "{at}: no citation");
                for c in i.cite.iter().chain(&i.label_cite) {
                    assert!(is_line_id(c), "{at}: {c:?} is not a CBETA line id");
                }
                // The first citation is in the set's own source.
                assert!(
                    i.cite[0].contains(&format!("n{}_", &s.source.work[1..])),
                    "{at}"
                );
            }
        }
    }

    /// The owner reviews renderings on the doc pages, so they must say
    /// exactly what the board shows.
    #[test]
    fn docs_match_the_catalog() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/precepts");
        for s in sets() {
            let page = dir.join(format!("{}.md", s.key.replace('_', "-")));
            let md = std::fs::read_to_string(&page)
                .unwrap()
                .replace("\r\n", "\n");
            for i in &s.items {
                let at = format!("{} {} {}", s.key, i.kind, i.n);
                assert!(md.contains(&format!("\n{}\n", i.text)), "{at}: rendering");
                assert!(
                    md.contains(&format!("{}. {} · ", i.n, i.title)),
                    "{at}: title"
                );
                assert!(md.contains(&format!("[{}]", i.cite[0])), "{at}: citation");
            }
        }
    }

    fn people() -> (State, u32, u32) {
        let mut s = State::new(crate::state::Limits {
            pin_rounds: 2,
            ..crate::state::Limits::board()
        });
        let dad = s.setup("Dad", "1234", 0).unwrap();
        let kid = s
            .add_person("Kid", crate::state::MEMBER, "1111", "", 0)
            .unwrap();
        (s, dad, kid)
    }

    fn me(id: u32) -> Actor {
        Actor::Person {
            id,
            admin: false,
            guest: false,
        }
    }

    fn taking(majors: &[u32], minors: &[u32], visibility: &str) -> PreceptsInput {
        PreceptsInput {
            date: "2025-04-08".into(),
            note: "At the temple, with Master Hui".into(),
            majors: majors.to_vec(),
            minors: minors.to_vec(),
            visibility: visibility.into(),
        }
    }

    const NOW: Option<u64> = Some(1_792_000_000);

    #[test]
    fn any_subset_but_no_unknown_items() {
        let (mut s, dad, _) = people();
        for (set, majors, minors) in [
            ("five", vec![6], vec![]),
            ("five", vec![0], vec![]),
            ("five", vec![1], vec![1]),
            ("upasaka", vec![], vec![29]),
            ("bodhisattva", vec![11], vec![]),
            ("five", vec![], vec![]),
        ] {
            let err = s
                .set_precepts(me(dad), set, &taking(&majors, &minors, ""), NOW)
                .unwrap_err();
            assert_eq!(err.status, 400, "{set} {majors:?} {minors:?}");
        }
        let eight = s.set_precepts(me(dad), "eight", &taking(&[1], &[], ""), NOW);
        assert_eq!(eight.unwrap_err().status, 404);
        let late = PreceptsInput {
            date: "2027-01-01".into(),
            ..taking(&[1], &[], "")
        };
        assert_eq!(
            s.set_precepts(me(dad), "five", &late, NOW)
                .unwrap_err()
                .status,
            400
        );
        // Two of the five (少分), given out of order and twice.
        s.set_precepts(me(dad), "five", &taking(&[4, 1, 4], &[], ""), NOW)
            .unwrap();
        let upasaka = taking(&[1, 2, 3, 4, 5, 6], &[21, 22], "");
        s.set_precepts(me(dad), "upasaka", &upasaka, NOW).unwrap();
        let mine = s.precepts_of(
            Viewer {
                id: dad,
                admin: false,
                outsider: false,
            },
            dad,
        );
        assert_eq!(mine.len(), 2);
        assert_eq!(
            (mine[0].set.as_str(), mine[0].majors.clone()),
            ("five", vec![1, 4])
        );
        assert_eq!(mine[1].minors, vec![21, 22]);
        assert_eq!((mine[1].total_majors, mine[1].total_minors), (6, 28));
        assert!(mine[0].draft);
        assert!(s.has_taken(dad, "upasaka", "minor", 21));
        assert!(!s.has_taken(dad, "upasaka", "minor", 20));
        // Taking more replaces the record.
        s.set_precepts(me(dad), "five", &taking(&[1, 2, 3, 4, 5], &[], ""), NOW)
            .unwrap();
        assert_eq!(s.precepts.len(), 2);
        s.remove_precepts(me(dad), "five").unwrap();
        assert_eq!(s.remove_precepts(me(dad), "five").unwrap_err().status, 404);
        let board = s.set_precepts(Actor::Board, "five", &taking(&[1], &[], ""), NOW);
        assert_eq!(board.unwrap_err().status, 403);
    }

    #[test]
    fn others_see_what_the_person_shows() {
        let (mut s, dad, kid) = people();
        s.set_precepts(me(dad), "five", &taking(&[1, 2], &[], "public"), NOW)
            .unwrap();
        s.set_precepts(me(dad), "novice_ten", &taking(&[1], &[], "generic"), NOW)
            .unwrap();
        s.set_precepts(me(dad), "bodhisattva", &taking(&[1], &[3], "private"), NOW)
            .unwrap();
        let theirs = s.precepts_of(
            Viewer {
                id: kid,
                admin: false,
                outsider: false,
            },
            dad,
        );
        assert_eq!(theirs.len(), 2);
        assert_eq!(theirs[0].majors, vec![1, 2]);
        assert_eq!(theirs[0].note, "At the temple, with Master Hui");
        assert!(theirs[1].generic && theirs[1].majors.is_empty() && theirs[1].note.is_empty());
        assert_eq!(
            s.precepts_of(
                Viewer {
                    id: 0,
                    admin: false,
                    outsider: false
                },
                dad
            )
            .len(),
            2
        );
        assert_eq!(
            s.precepts_of(
                Viewer {
                    id: kid,
                    admin: true,
                    outsider: false
                },
                dad
            )
            .len(),
            3
        );
    }

    #[test]
    fn line_ids() {
        assert!(is_line_id("T24n1484_p1004b16"));
        assert!(is_line_id("X60n1118_p0228a07"));
        assert!(!is_line_id("T24n1484_p1004d16"));
        assert!(!is_line_id("T1484"));
        assert!(!is_line_id("24n1484_p1004b16"));
    }
}
