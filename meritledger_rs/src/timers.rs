//! Meditation timer presets (`sprints/sprint06.md`). The timer itself runs
//! in the browser (`meritledger_ui/src/app/timer.ts`); the board keeps the
//! presets and receives one entry per finished session.
//!
//! A preset is either **timed** (sitting and walking segments, with bells)
//! or a **count** (a mala: tap per bead). Built-ins ship in
//! `catalog/timers.json`; custom ones live in the snapshot with keys `t<n>`.
use crate::ledger::Actor;
use crate::messages::{StoredSegment, StoredTimer, TimerInput};
use crate::state::State;
use miniframework::wire::{self, Format};
use miniframework::ApiError;
use std::sync::OnceLock;

const JSON: &str = include_str!("../catalog/timers.json");

miniframework::message! {
    /// `catalog/timers.json`.
    pub struct TimerCatalog {
        1 timers: Vec<StoredTimer>,
    }
}

pub fn builtins() -> &'static [StoredTimer] {
    static TIMERS: OnceLock<Vec<StoredTimer>> = OnceLock::new();
    TIMERS.get_or_init(|| {
        let catalog: TimerCatalog = wire::decode(Format::Json, JSON.as_bytes())
            .expect("catalog/timers.json is valid (tested)");
        catalog
            .timers
            .into_iter()
            .map(|t| StoredTimer { builtin: true, ..t })
            .collect()
    })
}

/// A preset as stored, checked against the kinds `owner` may log.
fn checked(state: &State, input: &TimerInput, owner: u32) -> Result<StoredTimer, ApiError> {
    let bad = |m: &str| Err(ApiError::bad_request(m.to_string()));
    let name = crate::state::normalize_name(&input.name)
        .map_err(|_| ApiError::bad_request("Name: 1 to 32 characters"))?;
    let counted = input.count_target > 0;
    if counted && !input.segments.is_empty() {
        return bad("A preset is timed or counted, not both");
    }
    if !counted && !(1..=12).contains(&input.segments.len()) {
        return bad("A timed preset has 1 to 12 parts");
    }
    if input.count_target > 100_000 {
        return bad("Count: up to 100,000");
    }
    let mut total = 0;
    for seg in &input.segments {
        if !matches!(seg.kind.as_str(), "sit" | "walk") {
            return bad("Parts are sitting or walking");
        }
        if !(1..=240).contains(&seg.minutes) {
            return bad("Each part: 1 to 240 minutes");
        }
        total += seg.minutes;
    }
    if total > 600 {
        return bad("A session can be up to 10 hours");
    }
    if input.open_strikes > 9 || input.close_strikes > 9 {
        return bad("Bells: up to 9 strikes");
    }
    if input.interval_minutes > 120 {
        return bad("Interval bells: up to every 120 minutes");
    }
    if !matches!(input.timbre.as_str(), "bowl" | "bell" | "block") {
        return bad("Sound: bowl, bell or block");
    }
    let kind = state
        .kind(&input.logs_as)
        .filter(|k| k.owner == 0 || k.owner == owner)
        .filter(|k| !k.archived)
        .ok_or_else(|| ApiError::bad_request("Log it as which practice?"))?;
    match (counted, kind.unit.as_str()) {
        (true, "count") | (false, "minutes") => {}
        (true, _) => return bad("A count logs as a practice measured in counts"),
        (false, _) => return bad("A timed session logs as a practice measured in minutes"),
    }
    Ok(StoredTimer {
        key: String::new(),
        name,
        segments: input
            .segments
            .iter()
            .map(|s| StoredSegment {
                kind: s.kind.clone(),
                minutes: s.minutes,
            })
            .collect(),
        open_strikes: input.open_strikes,
        interval_minutes: input.interval_minutes,
        close_strikes: input.close_strikes,
        count_target: input.count_target,
        logs_as: kind.key,
        timbre: input.timbre.clone(),
        owner,
        archived: false,
        builtin: false,
    })
}

impl State {
    /// Built-ins, the household's presets and the viewer's own.
    pub fn timers_for(&self, viewer: u32) -> Vec<StoredTimer> {
        builtins()
            .iter()
            .cloned()
            .chain(
                self.timers
                    .iter()
                    .filter(|t| t.owner == 0 || t.owner == viewer)
                    .cloned(),
            )
            .collect()
    }

    pub fn add_timer(&mut self, actor: Actor, input: &TimerInput) -> Result<String, ApiError> {
        let Actor::Person { id, guest, .. } = actor else {
            return Err(ApiError::new(
                403,
                "forbidden",
                "The board can't add timers",
            ));
        };
        if input.household && guest {
            return Err(ApiError::new(
                403,
                "guest",
                "Guests can add timers for themselves only",
            ));
        }
        let owner = if input.household { 0 } else { id };
        let mut timer = checked(self, input, owner)?;
        if self.timers.len() >= self.limits.timers {
            return Err(ApiError::new(
                409,
                "full",
                format!("At most {} custom timers", self.limits.timers),
            ));
        }
        timer.key = format!("t{}", self.next_timer);
        self.next_timer += 1;
        let key = timer.key.clone();
        self.timers.push(timer);
        Ok(key)
    }

    /// Replaces custom preset `key` (owner, or anyone but guests for the
    /// household's). Built-ins can't be changed.
    pub fn replace_timer(
        &mut self,
        actor: Actor,
        key: &str,
        input: &TimerInput,
    ) -> Result<(), ApiError> {
        let Actor::Person { id, admin, guest } = actor else {
            return Err(ApiError::new(
                403,
                "forbidden",
                "The board can't change timers",
            ));
        };
        if builtins().iter().any(|t| t.key == key) {
            return Err(ApiError::new(
                403,
                "builtin",
                "Built-in timers can't be changed; make your own",
            ));
        }
        let at = self
            .timers
            .iter()
            .position(|t| t.key == key && (t.owner == 0 || t.owner == id))
            .ok_or_else(|| ApiError::not_found("No such timer"))?;
        let owner = self.timers[at].owner;
        let allowed = if owner == 0 {
            admin || !guest
        } else {
            owner == id
        };
        if !allowed {
            return Err(ApiError::new(
                403,
                "forbidden",
                "That timer isn't yours to change",
            ));
        }
        let mut timer = checked(self, input, owner)?;
        timer.key = key.to_string();
        timer.archived = input.archived;
        self.timers[at] = timer;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Limits, GUEST};

    fn state() -> (State, u32, u32) {
        let mut s = State::new(Limits {
            pin_rounds: 2,
            ..Limits::board()
        });
        let dad = s.setup("Dad", "1234", 0).unwrap();
        let guest = s.add_person("Guest", GUEST, "1111", "", 0).unwrap();
        (s, dad, guest)
    }

    fn me(id: u32) -> Actor {
        Actor::Person {
            id,
            admin: false,
            guest: false,
        }
    }

    fn input() -> TimerInput {
        TimerInput {
            name: "Morning".into(),
            segments: vec![StoredSegment {
                kind: "sit".into(),
                minutes: 15,
            }],
            open_strikes: 3,
            interval_minutes: 5,
            close_strikes: 1,
            count_target: 0,
            logs_as: "meditation".into(),
            timbre: "bowl".into(),
            household: false,
            archived: false,
        }
    }

    #[test]
    fn built_ins_are_valid_presets() {
        let (s, dad, _) = state();
        for t in builtins() {
            let as_input = TimerInput {
                name: t.name.clone(),
                segments: t.segments.clone(),
                open_strikes: t.open_strikes,
                interval_minutes: t.interval_minutes,
                close_strikes: t.close_strikes,
                count_target: t.count_target,
                logs_as: t.logs_as.clone(),
                timbre: t.timbre.clone(),
                household: true,
                archived: false,
            };
            checked(&s, &as_input, dad).unwrap_or_else(|e| panic!("{}: {}", t.key, e.message));
            assert!(t.builtin && !t.key.starts_with('t'), "{}", t.key);
        }
        assert!(builtins()
            .iter()
            .any(|t| t.key == "sesshin" && t.segments.len() == 3));
    }

    #[test]
    fn presets_are_checked() {
        let (mut s, dad, _) = state();
        let bad = |change: fn(&mut TimerInput)| {
            let mut i = input();
            change(&mut i);
            i
        };
        for i in [
            bad(|i| i.segments.clear()),
            bad(|i| i.segments[0].minutes = 0),
            bad(|i| i.segments[0].kind = "nap".into()),
            bad(|i| i.count_target = 108),
            bad(|i| i.timbre = "gong".into()),
            bad(|i| i.logs_as = "daimoku".into()),
            bad(|i| i.open_strikes = 10),
        ] {
            assert_eq!(s.add_timer(me(dad), &i).unwrap_err().status, 400, "{i:?}");
        }
        let mala = TimerInput {
            segments: vec![],
            count_target: 54,
            logs_as: "daimoku".into(),
            ..input()
        };
        assert_eq!(s.add_timer(me(dad), &mala).unwrap(), "t1");
    }

    #[test]
    fn own_and_household_presets() {
        let (mut s, dad, guest) = state();
        let mine = s.add_timer(me(dad), &input()).unwrap();
        let ours = s
            .add_timer(
                me(dad),
                &TimerInput {
                    household: true,
                    ..input()
                },
            )
            .unwrap();
        let as_guest = Actor::Person {
            id: guest,
            admin: false,
            guest: true,
        };
        assert_eq!(
            s.add_timer(
                as_guest,
                &TimerInput {
                    household: true,
                    ..input()
                }
            )
            .unwrap_err()
            .status,
            403
        );
        let seen: Vec<_> = s.timers_for(guest).into_iter().map(|t| t.key).collect();
        assert!(seen.contains(&ours) && !seen.contains(&mine));
        assert_eq!(
            s.replace_timer(as_guest, &ours, &input())
                .unwrap_err()
                .status,
            403
        );
        s.replace_timer(
            me(dad),
            &mine,
            &TimerInput {
                archived: true,
                name: "Evening".into(),
                ..input()
            },
        )
        .unwrap();
        let t = s.timers.iter().find(|t| t.key == mine).unwrap();
        assert!(t.archived && t.name == "Evening");
        assert_eq!(
            s.replace_timer(me(dad), "sit20", &input())
                .unwrap_err()
                .code,
            "builtin"
        );
    }
}
