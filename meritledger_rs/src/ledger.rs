//! Practice kinds and entries (`sprints/sprint03.md`, `spec/domain.md`
//! § Practice): the rules, on [`State`], free of HTTP.
//!
//! - One tap is one entry, with the kind's default amount, dated today in
//!   the household zone. Doing a kind twice in a day is two entries.
//! - Anyone in the household may record for anyone (guests only for
//!   themselves); the entry keeps who recorded it (0 = the board's menu).
//! - `private` entries are seen by their person and admins; someone else's
//!   `generic` entry shows only its category.
use crate::catalog::{self, DEFAULT_PINS};
use crate::messages::{
    EntryInput, EntryUpdate, EntryView, KindInput, KindUpdate, KindView, StoredEntry, StoredKind,
    Tile, TodayView,
};
use crate::state::State;
use crate::tz::Date;
use miniframework::ApiError;

/// How far back an entry may be dated: the snapshot window. Older history
/// arrives with Sprint 5's archive.
pub const BACKFILL_DAYS: i64 = 59;
const NOTE_MAX: usize = 500;

/// Who is acting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Actor {
    /// A signed-in person.
    Person { id: u32, admin: bool, guest: bool },
    /// The board's own touch menu (Sprint 13): household trust, no session.
    Board,
}

/// Who is looking: a person, or nobody signed in (`id` 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewer {
    pub id: u32,
    pub admin: bool,
    /// A guest or someone not signed in: sees only what's public (plus a
    /// guest's own), never "generic" household detail.
    pub outsider: bool,
}

impl Viewer {
    /// Not signed in (account-less browsing of the intranet).
    pub fn anonymous() -> Viewer {
        Viewer {
            id: 0,
            admin: false,
            outsider: true,
        }
    }
}

impl Actor {
    fn id(&self) -> u32 {
        match self {
            Actor::Person { id, .. } => *id,
            Actor::Board => 0,
        }
    }
}

pub(crate) fn check_visibility(v: &str) -> Result<String, ApiError> {
    match v {
        "public" | "generic" | "private" => Ok(v.to_string()),
        _ => Err(ApiError::bad_request(
            "visibility must be public, generic or private",
        )),
    }
}

fn check_amount(unit: &str, amount: u32) -> Result<u32, ApiError> {
    match unit {
        "once" => Ok(0),
        "minutes" if (1..=1440).contains(&amount) => Ok(amount),
        "count" if (1..=1_000_000).contains(&amount) => Ok(amount),
        "minutes" => Err(ApiError::bad_request("Minutes: 1 to 1440")),
        _ => Err(ApiError::bad_request("Count: 1 to 1,000,000")),
    }
}

pub(crate) fn check_note(note: &str) -> Result<String, ApiError> {
    let note = note.trim();
    if note.chars().count() > NOTE_MAX {
        return Err(ApiError::bad_request(format!(
            "Notes: at most {NOTE_MAX} characters"
        )));
    }
    Ok(note.to_string())
}

/// Kinds whose details others never see: demerits and repentance.
pub fn sensitive(kind: &StoredKind) -> bool {
    kind.polarity == "demerit" || kind.category == "repentance"
}

pub(crate) fn clock_not_set() -> ApiError {
    ApiError::new(
        503,
        "clock_not_set",
        "The board's clock isn't set yet; try again in a minute",
    )
}

impl State {
    /// Today in the household zone, once the clock is set.
    pub fn today(&self, now: Option<u64>) -> Option<Date> {
        now.map(|t| self.tz.local_date(t as i64))
    }

    /// A built-in or custom kind by key.
    pub fn kind(&self, key: &str) -> Option<StoredKind> {
        catalog::builtin(key)
            .cloned()
            .or_else(|| self.kinds.iter().find(|k| k.key == key).cloned())
    }

    /// The household's kinds and `person`'s own.
    fn kind_usable_by(kind: &StoredKind, person: u32) -> bool {
        kind.owner == 0 || kind.owner == person
    }

    /// `person`'s Today kinds, in order.
    pub fn pins(&self, person: u32) -> Vec<String> {
        let chosen = self
            .person(person)
            .map(|p| p.pinned.clone())
            .unwrap_or_default();
        let keys: Vec<String> = if chosen.is_empty() {
            DEFAULT_PINS.iter().map(|k| k.to_string()).collect()
        } else {
            chosen
        };
        keys.into_iter()
            .filter(|k| {
                self.kind(k)
                    .is_some_and(|kind| !kind.archived && Self::kind_usable_by(&kind, person))
            })
            .collect()
    }

    pub fn set_pins(&mut self, person: u32, keys: &[String]) -> Result<(), ApiError> {
        if keys.len() > self.limits.pins {
            return Err(ApiError::new(
                409,
                "full",
                format!("At most {} on the Today page", self.limits.pins),
            ));
        }
        let mut seen = Vec::new();
        for key in keys {
            let kind = self
                .kind(key)
                .ok_or_else(|| ApiError::not_found(format!("No practice {key:?}")))?;
            if !Self::kind_usable_by(&kind, person) {
                return Err(ApiError::not_found(format!("No practice {key:?}")));
            }
            if !seen.contains(key) {
                seen.push(key.clone());
            }
        }
        self.person_mut(person)?.pinned = seen;
        Ok(())
    }

    fn kind_view(&self, kind: &StoredKind, pins: &[String]) -> KindView {
        KindView {
            key: kind.key.clone(),
            name: kind.name.clone(),
            icon: kind.icon.clone(),
            polarity: kind.polarity.clone(),
            category: kind.category.clone(),
            unit: kind.unit.clone(),
            default_amount: kind.default_amount,
            visibility: kind.visibility.clone(),
            content: kind.content,
            owner: kind.owner,
            archived: kind.archived,
            builtin: catalog::builtin(&kind.key).is_some(),
            pinned: pins.contains(&kind.key),
        }
    }

    /// Built-ins, the household's kinds and the viewer's own.
    pub fn kinds_for(&self, viewer: u32) -> Vec<KindView> {
        let pins = self.pins(viewer);
        catalog::builtins()
            .iter()
            .chain(
                self.kinds
                    .iter()
                    .filter(|k| Self::kind_usable_by(k, viewer)),
            )
            .map(|k| self.kind_view(k, &pins))
            .collect()
    }

    pub fn add_kind(&mut self, actor: Actor, input: &KindInput) -> Result<String, ApiError> {
        let Actor::Person { id, guest, .. } = actor else {
            return Err(ApiError::new(
                403,
                "forbidden",
                "The board can't add practices",
            ));
        };
        if input.household && guest {
            return Err(ApiError::new(
                403,
                "guest",
                "Guests can add practices for themselves only",
            ));
        }
        let name = crate::state::normalize_name(&input.name)
            .map_err(|_| ApiError::bad_request("Name: 1 to 32 characters"))?;
        let icon = match input.icon.trim() {
            "" => "•".to_string(),
            icon if icon.chars().count() <= 4 => icon.to_string(),
            _ => return Err(ApiError::bad_request("Icon: one emoji")),
        };
        let polarity = match input.polarity.as_str() {
            "merit" | "demerit" => input.polarity.clone(),
            _ => return Err(ApiError::bad_request("polarity must be merit or demerit")),
        };
        let category = input.category.trim().to_lowercase();
        if category.is_empty() || category.chars().count() > 24 {
            return Err(ApiError::bad_request("Category: 1 to 24 characters"));
        }
        let unit = match input.unit.as_str() {
            "minutes" | "count" | "once" => input.unit.clone(),
            _ => return Err(ApiError::bad_request("unit must be minutes, count or once")),
        };
        let default_amount = check_amount(&unit, input.default_amount)?;
        let visibility = match input.visibility.as_str() {
            "" if polarity == "demerit" => "private".to_string(),
            "" => "public".to_string(),
            v => check_visibility(v)?,
        };
        if self.kinds.len() >= self.limits.kinds {
            return Err(ApiError::new(
                409,
                "full",
                format!("At most {} custom practices", self.limits.kinds),
            ));
        }
        let key = format!("c{}", self.next_kind);
        self.next_kind += 1;
        self.kinds.push(StoredKind {
            key: key.clone(),
            name,
            icon,
            content: polarity == "merit" && unit != "once",
            polarity,
            category,
            unit,
            default_amount,
            visibility,
            owner: if input.household { 0 } else { id },
            archived: false,
        });
        Ok(key)
    }

    pub fn update_kind(
        &mut self,
        actor: Actor,
        key: &str,
        change: &KindUpdate,
    ) -> Result<(), ApiError> {
        let Actor::Person { id, admin, guest } = actor else {
            return Err(ApiError::new(
                403,
                "forbidden",
                "The board can't change practices",
            ));
        };
        if catalog::builtin(key).is_some() {
            return Err(ApiError::new(
                403,
                "builtin",
                "Built-in practices can't be changed; unpin it instead",
            ));
        }
        let name = change
            .name
            .as_deref()
            .map(crate::state::normalize_name)
            .transpose()?;
        let visibility = change
            .visibility
            .as_deref()
            .map(check_visibility)
            .transpose()?;
        let kind = self
            .kinds
            .iter_mut()
            .find(|k| k.key == key && (k.owner == 0 || k.owner == id))
            .ok_or_else(|| ApiError::not_found("No such practice"))?;
        let allowed = if kind.owner == 0 {
            admin || !guest
        } else {
            kind.owner == id
        };
        if !allowed {
            return Err(ApiError::new(
                403,
                "forbidden",
                "That practice isn't yours to change",
            ));
        }
        if let Some(amount) = change.default_amount {
            kind.default_amount = check_amount(&kind.unit, amount)?;
        }
        if let Some(name) = name {
            kind.name = name;
        }
        if let Some(icon) = &change.icon {
            kind.icon = icon.trim().chars().take(4).collect();
        }
        if let Some(v) = visibility {
            kind.visibility = v;
        }
        if let Some(archived) = change.archived {
            kind.archived = archived;
        }
        Ok(())
    }

    /// Records one entry; returns its id.
    pub fn record(
        &mut self,
        actor: Actor,
        input: &EntryInput,
        now: Option<u64>,
    ) -> Result<u32, ApiError> {
        let person = match (input.person, actor) {
            (0, Actor::Person { id, .. }) => id,
            (0, Actor::Board) => return Err(ApiError::bad_request("Record for whom?")),
            (p, _) => p,
        };
        if let Actor::Person {
            id, guest: true, ..
        } = actor
        {
            if person != id {
                return Err(ApiError::new(
                    403,
                    "guest",
                    "Guests record for themselves only",
                ));
            }
        }
        match self.person(person) {
            Some(p) if !p.disabled => {}
            _ => return Err(ApiError::not_found("No such person")),
        }
        let kind = self
            .kind(&input.kind)
            .filter(|k| Self::kind_usable_by(k, person))
            .ok_or_else(|| ApiError::not_found("No such practice"))?;
        if kind.archived {
            return Err(ApiError::new(409, "archived", "That practice is archived"));
        }
        let amount = check_amount(&kind.unit, input.amount.unwrap_or(kind.default_amount))?;
        let visibility = match &input.visibility {
            Some(v) => check_visibility(v)?,
            None => kind.visibility.clone(),
        };
        let date = self.entry_date(input.date.as_deref(), now)?;
        let note = check_note(&input.note)?;
        let source = match (actor, input.source.as_deref()) {
            (Actor::Board, _) => "board",
            (_, None | Some("tap")) => "tap",
            (_, Some("timer")) => "timer",
            (_, Some("manual")) => "manual",
            (_, Some(_)) => {
                return Err(ApiError::bad_request("source must be tap, timer or manual"))
            }
        };
        // A retried POST (the timer's offline queue) records nothing new.
        let client_id = input.client_id.trim();
        if client_id.len() > 64 {
            return Err(ApiError::bad_request("client_id: at most 64 characters"));
        }
        if !client_id.is_empty() {
            if let Some(e) = self
                .entries
                .iter()
                .find(|e| e.person == person && e.client_id == client_id)
            {
                return Ok(e.id);
            }
        }
        if self.entries.len() >= self.limits.entries {
            return Err(ApiError::new(
                409,
                "full",
                format!("The board holds {} recent entries; older ones move to the SD card from Sprint 5", self.limits.entries),
            ));
        }
        let id = self.next_entry;
        self.next_entry += 1;
        // With a journal the snapshot holds a lease on ids, renewed every
        // 1,000, so a later boot without the card never reuses one.
        if self.journaled && self.next_entry > self.entry_lease {
            self.entry_lease = self.next_entry + 1_000;
        }
        self.entries.push(StoredEntry {
            id,
            person,
            kind: kind.key,
            date: date.iso(),
            created: now.unwrap_or(0),
            amount,
            note,
            visibility,
            source: source.into(),
            recorded_by: actor.id(),
            rev: 1,
            rejoiced_by: Vec::new(),
            deleted: false,
            client_id: client_id.to_string(),
            reading: 0,
        });
        Ok(id)
    }

    /// `viewer` rejoices in entry `id` (mudita): counted on the entry, and
    /// recorded as their own "rejoicing" merit. Once per person per entry.
    pub fn rejoice(&mut self, actor: Actor, id: u32, now: Option<u64>) -> Result<u32, ApiError> {
        let Actor::Person { id: me, admin, .. } = actor else {
            return Err(ApiError::new(403, "forbidden", "Only people rejoice"));
        };
        let entry = self
            .entries
            .iter()
            .find(|e| e.id == id)
            .cloned()
            .ok_or_else(|| ApiError::not_found("No such entry"))?;
        let view = self
            .entry_view(
                Viewer {
                    id: me,
                    admin,
                    outsider: false,
                },
                &entry,
            )
            .filter(|v| v.can_rejoice)
            .ok_or_else(|| {
                ApiError::new(
                    403,
                    "cannot_rejoice",
                    "You can rejoice in someone else's public merit",
                )
            })?;
        if entry.rejoiced_by.contains(&me) {
            return Err(ApiError::new(
                409,
                "already",
                "You already rejoiced in this",
            ));
        }
        let note = format!(
            "In {}'s {}",
            view.person_name,
            view.kind_name.to_lowercase()
        );
        let input = EntryInput {
            kind: "rejoicing".into(),
            note: note.chars().take(NOTE_MAX).collect(),
            ..Default::default()
        };
        let mine = self.record(actor, &input, now)?;
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
            e.rejoiced_by.push(me);
        }
        self.notify(
            entry.person,
            crate::notify::REJOICE,
            id,
            me,
            "",
            now.unwrap_or(0),
        );
        Ok(mine)
    }

    /// A requested date (or today), within the snapshot window and not in
    /// the future.
    fn entry_date(&self, requested: Option<&str>, now: Option<u64>) -> Result<Date, ApiError> {
        let today = self.today(now);
        let date = match requested {
            Some(s) => {
                Date::parse(s).ok_or_else(|| ApiError::bad_request("Dates look like 2026-10-18"))?
            }
            None => today.ok_or_else(clock_not_set)?,
        };
        if let Some(today) = today {
            if date > today {
                return Err(ApiError::bad_request("That date hasn't happened yet"));
            }
            if date.days() < today.days() - BACKFILL_DAYS {
                return Err(ApiError::bad_request(format!(
                    "Entries can go back {} days for now",
                    BACKFILL_DAYS + 1
                )));
            }
        }
        Ok(date)
    }

    fn can_edit(viewer: Viewer, entry: &StoredEntry) -> bool {
        viewer.id != 0
            && (viewer.admin || viewer.id == entry.person || viewer.id == entry.recorded_by)
    }

    fn entry_mut(&mut self, viewer: Viewer, id: u32) -> Result<&mut StoredEntry, ApiError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or_else(|| ApiError::not_found("No such entry"))?;
        if !Self::can_edit(viewer, entry) {
            return Err(ApiError::new(
                403,
                "forbidden",
                "Only its person, who recorded it, or an admin can change it",
            ));
        }
        Ok(entry)
    }

    pub fn update_entry(
        &mut self,
        viewer: Viewer,
        id: u32,
        change: &EntryUpdate,
        now: Option<u64>,
    ) -> Result<(), ApiError> {
        let date = change
            .date
            .as_deref()
            .map(|d| self.entry_date(Some(d), now))
            .transpose()?;
        let note = change.note.as_deref().map(check_note).transpose()?;
        let visibility = change
            .visibility
            .as_deref()
            .map(check_visibility)
            .transpose()?;
        let unit = {
            let entry = self.entry_mut(viewer, id)?;
            if entry.rev != change.rev {
                return Err(ApiError::new(
                    409,
                    "stale",
                    "Someone changed this entry; reload and try again",
                ));
            }
            entry.kind.clone()
        };
        let unit = self
            .kind(&unit)
            .map(|k| k.unit)
            .unwrap_or_else(|| "once".into());
        let amount = change.amount.map(|a| check_amount(&unit, a)).transpose()?;
        let entry = self.entry_mut(viewer, id)?;
        if let Some(a) = amount {
            entry.amount = a;
        }
        if let Some(n) = note {
            entry.note = n;
        }
        if let Some(v) = visibility {
            entry.visibility = v;
        }
        if let Some(d) = date {
            entry.date = d.iso();
        }
        entry.rev += 1;
        Ok(())
    }

    pub fn delete_entry(&mut self, viewer: Viewer, id: u32) -> Result<(), ApiError> {
        self.entry_mut(viewer, id)?;
        self.entries.retain(|e| e.id != id);
        Ok(())
    }

    pub(crate) fn name_of(&self, id: u32) -> String {
        match id {
            0 => "the board".into(),
            id => self
                .person(id)
                .map_or_else(|| "a former member".into(), |p| p.display_name.clone()),
        }
    }

    /// `entry` as `viewer` may see it, or `None` when it's private to
    /// someone else.
    pub fn entry_view(&self, viewer: Viewer, entry: &StoredEntry) -> Option<EntryView> {
        let own = viewer.admin || (viewer.id != 0 && viewer.id == entry.person);
        if entry.visibility == "private" && !own {
            return None;
        }
        // Guests and visitors see what's public, not "generic" household
        // detail (owner, 2026-10-21).
        if viewer.outsider && !own && entry.visibility != "public" {
            return None;
        }
        let kind = self.kind(&entry.kind).unwrap_or_else(|| StoredKind {
            key: entry.kind.clone(),
            name: "A retired practice".into(),
            icon: "•".into(),
            polarity: "merit".into(),
            category: "practice".into(),
            unit: "once".into(),
            ..Default::default()
        });
        // Demerit and repentance details are only ever their person's (and
        // admins'): "public" for those kinds still shows just the category.
        let generic = !own && (entry.visibility == "generic" || sensitive(&kind));
        Some(EntryView {
            id: entry.id,
            person: entry.person,
            person_name: self.name_of(entry.person),
            kind: if generic {
                String::new()
            } else {
                entry.kind.clone()
            },
            kind_name: if generic {
                catalog::generic_label(&kind)
            } else {
                kind.name.clone()
            },
            icon: if generic {
                "•".into()
            } else {
                kind.icon.clone()
            },
            // A demerit's category ("speech") would say too much.
            category: if generic {
                String::new()
            } else {
                kind.category.clone()
            },
            polarity: if generic {
                String::new()
            } else {
                kind.polarity.clone()
            },
            unit: if generic {
                "once".into()
            } else {
                kind.unit.clone()
            },
            date: entry.date.clone(),
            created: entry.created,
            amount: if generic { 0 } else { entry.amount },
            note: if generic {
                String::new()
            } else {
                entry.note.clone()
            },
            visibility: entry.visibility.clone(),
            source: entry.source.clone(),
            recorded_by_name: if generic {
                String::new()
            } else {
                self.name_of(entry.recorded_by)
            },
            rev: entry.rev,
            can_edit: Self::can_edit(viewer, entry),
            generic,
            rejoices: entry.rejoiced_by.len() as u32,
            rejoiced_by_me: viewer.id != 0 && entry.rejoiced_by.contains(&viewer.id),
            can_rejoice: viewer.id != 0
                && viewer.id != entry.person
                && !generic
                && kind.polarity == "merit"
                && entry.visibility == "public",
        })
    }

    /// `person`'s entries from `from` to `to` (inclusive), newest first.
    pub fn entries_for(
        &self,
        viewer: Viewer,
        person: u32,
        from: Date,
        to: Date,
        limit: usize,
    ) -> Vec<EntryView> {
        self.views_of(&self.entries, viewer, person, from, to, limit)
    }

    /// Like [`State::entries_for`], over `list` (older entries read back
    /// from the journal, say).
    pub fn views_of(
        &self,
        list: &[StoredEntry],
        viewer: Viewer,
        person: u32,
        from: Date,
        to: Date,
        limit: usize,
    ) -> Vec<EntryView> {
        let (from, to) = (from.iso(), to.iso());
        let mut found: Vec<&StoredEntry> = list
            .iter()
            .filter(|e| e.person == person && e.date >= from && e.date <= to)
            .collect();
        found.sort_by(|a, b| {
            b.date
                .cmp(&a.date)
                .then(b.created.cmp(&a.created))
                .then(b.id.cmp(&a.id))
        });
        found
            .into_iter()
            .filter_map(|e| self.entry_view(viewer, e))
            .take(limit)
            .collect()
    }

    /// The Today page: `person`'s pinned kinds with today's counts, and
    /// today's entries.
    pub fn today_view(
        &self,
        viewer: Viewer,
        person: u32,
        now: Option<u64>,
    ) -> Result<TodayView, ApiError> {
        let target = self
            .person(person)
            .filter(|p| !p.disabled)
            .ok_or_else(|| ApiError::not_found("No such person"))?;
        let today = self.today(now).ok_or_else(clock_not_set)?;
        let date = today.iso();
        let todays: Vec<&StoredEntry> = self
            .entries
            .iter()
            .filter(|e| e.person == person && e.date == date)
            .collect();
        let tiles = self
            .pins(person)
            .iter()
            .filter_map(|key| self.kind(key))
            .map(|k| {
                let mine: Vec<_> = todays.iter().filter(|e| e.kind == k.key).collect();
                Tile {
                    count: mine.len() as u32,
                    total: mine.iter().map(|e| e.amount).sum(),
                    key: k.key,
                    name: k.name,
                    icon: k.icon,
                    unit: k.unit,
                    default_amount: k.default_amount,
                    polarity: k.polarity,
                }
            })
            .collect();
        let observances = crate::calendar::observances(
            self.calendar_of(person),
            self.calendar_options(),
            &self.tz,
            today,
            today,
        )
        .iter()
        .map(crate::calendar::view)
        .collect();
        Ok(TodayView {
            observances,
            date,
            person,
            person_name: target.display_name.clone(),
            tiles,
            entries: self.entries_for(viewer, person, today, today, 100),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Limits, GUEST, MEMBER};

    /// 2026-10-18 16:00 UTC: noon in New York, a Sunday.
    const NOON: u64 = 1_792_281_600 + 16 * 3600;

    fn household() -> (State, u32, u32, u32) {
        let mut s = State::new(Limits {
            pin_rounds: 2,
            ..Limits::board()
        });
        let dad = s.setup("Dad", "1234", 0).unwrap();
        let kid = s.add_person("Kid", MEMBER, "1111", "", 0).unwrap();
        let guest = s.add_person("Guest", GUEST, "2222", "", 0).unwrap();
        (s, dad, kid, guest)
    }

    fn person(id: u32, admin: bool) -> Actor {
        Actor::Person {
            id,
            admin,
            guest: false,
        }
    }

    fn tap(kind: &str) -> EntryInput {
        EntryInput {
            kind: kind.into(),
            ..Default::default()
        }
    }

    #[test]
    fn noon_constant_is_what_it_says() {
        let s = household().0;
        assert_eq!(s.today(Some(NOON)).unwrap().iso(), "2026-10-18");
    }

    #[test]
    fn a_tap_records_the_default_for_today() {
        let (mut s, dad, ..) = household();
        let id = s
            .record(person(dad, true), &tap("meditation"), Some(NOON))
            .unwrap();
        let e = s.entries.iter().find(|e| e.id == id).unwrap();
        assert_eq!(
            (
                e.person,
                e.amount,
                e.date.as_str(),
                e.source.as_str(),
                e.recorded_by
            ),
            (dad, 20, "2026-10-18", "tap", dad)
        );
        assert_eq!(e.visibility, "public");
    }

    #[test]
    fn dates_follow_the_household_zone() {
        let (mut s, dad, ..) = household();
        // 23:59 and 00:01 New York time on either side of midnight.
        let late = NOON + 11 * 3600 + 59 * 60;
        let early = NOON + 12 * 3600 + 60;
        s.record(person(dad, true), &tap("vegetarian"), Some(late))
            .unwrap();
        s.record(person(dad, true), &tap("vegetarian"), Some(early))
            .unwrap();
        let dates: Vec<_> = s.entries.iter().map(|e| e.date.clone()).collect();
        assert_eq!(dates, ["2026-10-18", "2026-10-19"]);
    }

    #[test]
    fn twice_a_day_is_two_entries_and_today_counts_them() {
        let (mut s, dad, ..) = household();
        s.record(person(dad, true), &tap("meditation"), Some(NOON))
            .unwrap();
        let second = EntryInput {
            amount: Some(10),
            ..tap("meditation")
        };
        s.record(person(dad, true), &second, Some(NOON)).unwrap();
        let today = s
            .today_view(
                Viewer {
                    id: dad,
                    admin: true,
                    outsider: false,
                },
                dad,
                Some(NOON),
            )
            .unwrap();
        let tile = today.tiles.iter().find(|t| t.key == "meditation").unwrap();
        assert_eq!((tile.count, tile.total), (2, 30));
        assert_eq!(today.entries.len(), 2);
    }

    #[test]
    fn demerits_default_private_and_repentance_generic() {
        let (mut s, dad, kid, _) = household();
        s.record(person(kid, false), &tap("anger"), Some(NOON))
            .unwrap();
        s.record(person(kid, false), &tap("repentance"), Some(NOON))
            .unwrap();
        let from_dad = Viewer {
            id: dad,
            admin: false,
            outsider: false,
        };
        let seen = s.entries_for(
            from_dad,
            kid,
            Date::parse("2026-10-18").unwrap(),
            Date::parse("2026-10-18").unwrap(),
            10,
        );
        assert_eq!(seen.len(), 1, "the private one is hidden");
        assert!(seen[0].generic);
        assert_eq!(seen[0].kind_name, "A repentance practice");
        assert!(seen[0].note.is_empty() && seen[0].kind.is_empty());
        // The kid sees both, in full.
        let own = s.entries_for(
            Viewer {
                id: kid,
                admin: false,
                outsider: false,
            },
            kid,
            Date::parse("2026-10-18").unwrap(),
            Date::parse("2026-10-18").unwrap(),
            10,
        );
        assert_eq!(own.len(), 2);
        assert!(own.iter().all(|e| !e.generic));
    }

    #[test]
    fn recording_for_someone_else_keeps_who_did_it() {
        let (mut s, dad, kid, guest) = household();
        let input = EntryInput {
            person: kid,
            ..tap("prostrations")
        };
        let id = s.record(person(dad, false), &input, Some(NOON)).unwrap();
        let e = s.entries.iter().find(|e| e.id == id).unwrap();
        assert_eq!((e.person, e.recorded_by), (kid, dad));
        // The board's menu records as 0, "the board".
        let id = s.record(Actor::Board, &input, Some(NOON)).unwrap();
        let view = s
            .entry_view(
                Viewer {
                    id: kid,
                    admin: false,
                    outsider: false,
                },
                s.entries.iter().find(|e| e.id == id).unwrap(),
            )
            .unwrap();
        assert_eq!(
            (view.source.as_str(), view.recorded_by_name.as_str()),
            ("board", "the board")
        );
        // Guests record only for themselves.
        let as_guest = Actor::Person {
            id: guest,
            admin: false,
            guest: true,
        };
        assert_eq!(
            s.record(as_guest, &input, Some(NOON)).unwrap_err().status,
            403
        );
        assert!(s.record(as_guest, &tap("meditation"), Some(NOON)).is_ok());
    }

    #[test]
    fn only_the_person_recorder_or_admin_edits() {
        let (mut s, dad, kid, guest) = household();
        let id = s
            .record(person(kid, false), &tap("chanting"), Some(NOON))
            .unwrap();
        let change = EntryUpdate {
            rev: 1,
            amount: Some(45),
            ..Default::default()
        };
        assert_eq!(
            s.update_entry(
                Viewer {
                    id: guest,
                    admin: false,
                    outsider: false
                },
                id,
                &change,
                Some(NOON)
            )
            .unwrap_err()
            .status,
            403
        );
        s.update_entry(
            Viewer {
                id: kid,
                admin: false,
                outsider: false,
            },
            id,
            &change,
            Some(NOON),
        )
        .unwrap();
        // The same rev again is stale.
        assert_eq!(
            s.update_entry(
                Viewer {
                    id: dad,
                    admin: true,
                    outsider: false
                },
                id,
                &change,
                Some(NOON)
            )
            .unwrap_err()
            .code,
            "stale"
        );
        assert_eq!(s.entries[0].amount, 45);
        assert_eq!(s.entries[0].rev, 2);
        s.delete_entry(
            Viewer {
                id: dad,
                admin: true,
                outsider: false,
            },
            id,
        )
        .unwrap();
        assert!(s.entries.is_empty());
    }

    #[test]
    fn amounts_dates_and_notes_are_checked() {
        let (mut s, dad, ..) = household();
        let a = person(dad, true);
        let bad = |amount| EntryInput {
            amount: Some(amount),
            ..tap("meditation")
        };
        assert_eq!(s.record(a, &bad(0), Some(NOON)).unwrap_err().status, 400);
        assert_eq!(s.record(a, &bad(1441), Some(NOON)).unwrap_err().status, 400);
        // `once` kinds ignore an amount.
        let once = EntryInput {
            amount: Some(99),
            ..tap("vegetarian")
        };
        let id = s.record(a, &once, Some(NOON)).unwrap();
        assert_eq!(s.entries.iter().find(|e| e.id == id).unwrap().amount, 0);
        let dated = |d: &str| EntryInput {
            date: Some(d.into()),
            ..tap("meditation")
        };
        assert!(s.record(a, &dated("2026-10-17"), Some(NOON)).is_ok());
        assert!(
            s.record(a, &dated("2026-08-20"), Some(NOON)).is_ok(),
            "59 days back"
        );
        assert_eq!(
            s.record(a, &dated("2026-08-19"), Some(NOON))
                .unwrap_err()
                .status,
            400
        );
        assert_eq!(
            s.record(a, &dated("2026-10-19"), Some(NOON))
                .unwrap_err()
                .status,
            400
        );
        // No clock and no date: can't know what today is.
        assert_eq!(
            s.record(a, &tap("meditation"), None).unwrap_err().code,
            "clock_not_set"
        );
        let long = EntryInput {
            note: "x".repeat(501),
            ..tap("meditation")
        };
        assert_eq!(s.record(a, &long, Some(NOON)).unwrap_err().status, 400);
    }

    #[test]
    fn custom_kinds_household_and_personal() {
        let (mut s, dad, kid, guest) = household();
        let input = |household| KindInput {
            name: "Tea ceremony".into(),
            icon: "🍵".into(),
            polarity: "merit".into(),
            category: "Ritual".into(),
            unit: "minutes".into(),
            default_amount: 15,
            visibility: String::new(),
            household,
        };
        let mine = s.add_kind(person(kid, false), &input(false)).unwrap();
        let ours = s.add_kind(person(dad, true), &input(true)).unwrap();
        assert_eq!((mine.as_str(), ours.as_str()), ("c1", "c2"));
        assert_eq!(s.kind(&ours).unwrap().category, "ritual");
        // Dad can't use the kid's personal kind, or see it.
        assert!(s.kinds_for(dad).iter().all(|k| k.key != mine));
        let for_kid = EntryInput {
            person: kid,
            ..tap(&mine)
        };
        assert!(s
            .record(
                person(dad, true),
                &EntryInput {
                    person: dad,
                    ..tap(&mine)
                },
                Some(NOON)
            )
            .is_err());
        assert!(s.record(person(dad, true), &for_kid, Some(NOON)).is_ok());
        // Guests can't add household kinds.
        let as_guest = Actor::Person {
            id: guest,
            admin: false,
            guest: true,
        };
        assert_eq!(s.add_kind(as_guest, &input(true)).unwrap_err().status, 403);
        // Built-ins can't be changed; archived customs can't be recorded.
        let archive = KindUpdate {
            archived: Some(true),
            ..Default::default()
        };
        assert_eq!(
            s.update_kind(person(dad, true), "meditation", &archive)
                .unwrap_err()
                .code,
            "builtin"
        );
        s.update_kind(person(dad, true), &ours, &archive).unwrap();
        assert_eq!(
            s.record(person(dad, true), &tap(&ours), Some(NOON))
                .unwrap_err()
                .code,
            "archived"
        );
    }

    #[test]
    fn pins_default_then_follow_the_person() {
        let (mut s, dad, ..) = household();
        assert_eq!(s.pins(dad), DEFAULT_PINS.map(String::from).to_vec());
        s.set_pins(
            dad,
            &["daimoku".into(), "meditation".into(), "daimoku".into()],
        )
        .unwrap();
        assert_eq!(s.pins(dad), ["daimoku", "meditation"]);
        assert_eq!(s.set_pins(dad, &["nope".into()]).unwrap_err().status, 404);
    }

    #[test]
    fn the_snapshot_window_is_bounded() {
        let mut s = State::new(Limits {
            pin_rounds: 2,
            entries: 3,
            ..Limits::board()
        });
        let dad = s.setup("Dad", "1234", 0).unwrap();
        for _ in 0..3 {
            s.record(person(dad, true), &tap("meditation"), Some(NOON))
                .unwrap();
        }
        let err = s
            .record(person(dad, true), &tap("meditation"), Some(NOON))
            .unwrap_err();
        assert_eq!((err.status, err.code), (409, "full"));
    }

    #[test]
    fn removing_someone_keeps_or_drops_their_entries() {
        let (mut s, dad, kid, guest) = household();
        s.record(person(kid, false), &tap("meditation"), Some(NOON))
            .unwrap();
        s.record(
            Actor::Person {
                id: guest,
                admin: false,
                guest: true,
            },
            &tap("meditation"),
            Some(NOON),
        )
        .unwrap();
        s.remove_person(dad, kid, true).unwrap();
        s.remove_person(dad, guest, false).unwrap();
        assert_eq!(s.entries.len(), 1, "the kid's stays, the guest's went");
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
        assert_eq!(view.person_name, "a former member");
    }

    #[test]
    fn public_demerits_still_show_only_the_category() {
        let (mut s, dad, kid, _) = household();
        let public = EntryInput {
            visibility: Some("public".into()),
            note: "detail".into(),
            ..tap("harsh_speech")
        };
        let id = s.record(person(kid, false), &public, Some(NOON)).unwrap();
        let entry = s.entries.iter().find(|e| e.id == id).unwrap().clone();
        let other = s
            .entry_view(
                Viewer {
                    id: 0,
                    admin: false,
                    outsider: false,
                },
                &entry,
            )
            .unwrap();
        assert!(other.generic);
        assert_eq!(other.kind_name, "A moment of reflection");
        assert!(other.note.is_empty());
        let own = s
            .entry_view(
                Viewer {
                    id: kid,
                    admin: false,
                    outsider: false,
                },
                &entry,
            )
            .unwrap();
        assert!(!own.generic && own.note == "detail");
        assert!(
            !s.entry_view(
                Viewer {
                    id: dad,
                    admin: true,
                    outsider: false
                },
                &entry
            )
            .unwrap()
            .generic
        );
    }

    /// Sprint 3 exit: the snapshot with a full entry window must fit the
    /// `mlstate` NVS partition (1 MiB) twice over, plus garbage collection.
    #[test]
    fn a_full_window_fits_the_state_partition() {
        use miniframework::wire::{self, Format};
        let mut s = State::new(Limits {
            pin_rounds: 2,
            ..Limits::board()
        });
        let dad = s.setup("Dad", "1234", 0).unwrap();
        for i in 2..=16 {
            s.add_person(&format!("Person {i}"), MEMBER, "", "", 0)
                .unwrap();
        }
        let kinds = [
            "meditation",
            "chanting",
            "prostrations",
            "reading",
            "vegetarian",
        ];
        for n in 0..s.limits.entries {
            let input = EntryInput {
                person: (n % 16) as u32 + 1,
                note: if n % 4 == 0 {
                    "a short note about it".into()
                } else {
                    String::new()
                },
                date: Some(
                    Date::parse("2026-10-18")
                        .unwrap()
                        .add_days(-((n / 80) as i64))
                        .iso(),
                ),
                ..tap(kinds[n % kinds.len()])
            };
            s.record(person(dad, true), &input, Some(NOON)).unwrap();
        }
        let bytes = wire::to_vec(Format::CborInt, &s.to_stored()).unwrap().len();
        println!("snapshot with {} entries: {bytes} bytes", s.entries.len());
        assert!(bytes < 300 * 1024, "{bytes} bytes");
    }

    #[test]
    fn entries_survive_the_snapshot() {
        let (mut s, dad, ..) = household();
        s.record(person(dad, true), &tap("meditation"), Some(NOON))
            .unwrap();
        s.set_pins(dad, &["daimoku".into()]).unwrap();
        s.timezone = "UTC0".into();
        let back = State::from_stored(s.to_stored(), s.limits.clone());
        assert_eq!(back.entries, s.entries);
        assert_eq!(back.next_entry, 2);
        assert_eq!(back.pins(dad), ["daimoku"]);
        assert_eq!(
            back.today(Some(NOON + 9 * 3600)).unwrap().iso(),
            "2026-10-19",
            "UTC zone was kept"
        );
    }
}
