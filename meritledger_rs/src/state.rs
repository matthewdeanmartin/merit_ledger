//! The domain state and its rules, free of HTTP and storage so it can be
//! tested directly. `api.rs` locks it, calls these, and commits.
//!
//! People (`spec/domain.md` § People): the household trusts itself, so
//! sign-in is "tap your name, enter your PIN": names are unique and public.
//! Someone without a PIN can't sign in; others record for them.
use crate::messages::{
    AdminPersonUpdate, StoredDay, StoredEntry, StoredKind, StoredPerson, StoredPost,
    StoredPracticeDoc, StoredPrecepts, StoredSession, StoredState, StoredText, StoredTimer,
    StoredVow,
};
use crate::pin;
use crate::stats::Tally;
use crate::tz::Tz;
use miniframework::session::{Record, SessionConfig, Sessions};
use miniframework::ApiError;
use std::collections::BTreeMap;

/// Snapshot schema this firmware writes (and the newest it can read).
pub const SCHEMA: u16 = 1;

pub const ADMIN: &str = "admin";
pub const MEMBER: &str = "member";
pub const GUEST: &str = "guest";

/// Bounds that differ between a board and a desktop.
#[derive(Clone, Debug)]
pub struct Limits {
    pub people: usize,
    pub sessions: usize,
    pub sessions_per_person: usize,
    /// PBKDF2 rounds for new PIN hashes.
    pub pin_rounds: u32,
    /// Custom practice kinds (household and personal together).
    pub kinds: usize,
    /// Entries kept in the snapshot (Sprint 5 moves older ones to the SD card).
    pub entries: usize,
    /// Kinds on one Today page.
    pub pins: usize,
    /// Custom timer presets (household and personal together).
    pub timers: usize,
    /// Vows, and posts on them (everyone's together).
    pub vows: usize,
    pub posts: usize,
    /// Bytes of post bodies kept in the snapshot when there's no SD card.
    pub snapshot_post_bytes: usize,
    /// Household texts and built practices.
    pub texts: usize,
    pub practices: usize,
    /// Bytes of text bodies kept in the snapshot when there's no SD card.
    pub snapshot_text_bytes: usize,
}

impl Limits {
    pub fn board() -> Self {
        Self {
            people: 16,
            sessions: 64,
            sessions_per_person: 4,
            // ~100 ms per check on the S3 (1,000 rounds measured 63 ms,
            // October 18, 2026; the firmware logs it at every boot).
            pin_rounds: 1_600,
            kinds: 256,
            // ~45 bytes each as CBOR: 4,096 is ~180 KB, a third of what
            // the 1 MiB `mlstate` partition can hold with both slots.
            entries: 4_096,
            pins: 24,
            timers: 64,
            // ~120 bytes each plus ~60 per timeline event.
            vows: 256,
            // ~80 bytes each in the snapshot; bodies on the card.
            posts: 1_024,
            snapshot_post_bytes: 16 * 1024,
            texts: 64,
            // ~40 bytes per element, up to 40 elements.
            practices: 128,
            snapshot_text_bytes: 16 * 1024,
        }
    }
}

pub struct State {
    pub people: Vec<StoredPerson>,
    pub sessions: Sessions,
    pub next_person: u32,
    pub signup_closed: bool,
    pub commits: u64,
    pub kinds: Vec<StoredKind>,
    pub next_kind: u32,
    pub entries: Vec<StoredEntry>,
    pub next_entry: u32,
    /// The household zone as configured (empty = [`DEFAULT_TZ`]) and parsed.
    pub timezone: String,
    pub tz: Tz,
    /// Entries live in the SD card's journal (`journal.rs`), not the
    /// snapshot. Set by `App::open` when a card is present.
    pub journaled: bool,
    /// Days since 1970-01-01 of the oldest day kept in `entries` (the
    /// window); older days are only in `history`.
    pub window_start: i64,
    /// Per-day totals before the window: from the journal at boot, plus
    /// `archived_days`. RAM only.
    pub history: BTreeMap<(u32, i64), Tally>,
    /// Totals of entries that aged out without a card (persisted).
    pub archived_days: Vec<StoredDay>,
    /// With a journal, ids up to here are promised to no one else: the
    /// snapshot records this, not every new id (`ledger::record`).
    pub entry_lease: u32,
    /// Custom timer presets (`timers.rs`).
    pub timers: Vec<StoredTimer>,
    pub next_timer: u32,
    /// The household calendar (`calendar.rs`); empty = [`DEFAULT_CALENDAR`].
    pub calendar: String,
    pub sunday_shift: bool,
    pub quarter_moons: bool,
    pub ten_fasting_days: bool,
    /// Entries have been on an SD card (persisted).
    pub used_card: bool,
    /// Vows and their posts (`vows.rs`), and precepts taken.
    pub vows: Vec<StoredVow>,
    pub next_vow: u32,
    pub posts: Vec<StoredPost>,
    pub next_post: u32,
    pub precepts: Vec<StoredPrecepts>,
    /// Household texts and built practices (`practice_docs.rs`).
    pub texts: Vec<StoredText>,
    pub next_text: u32,
    pub practices: Vec<StoredPracticeDoc>,
    pub next_practice: u32,
    pub limits: Limits,
}

/// The household's calendar until it picks one: its practice descends from
/// Chinese Buddhism (`spec/v2_decisions.md`).
pub const DEFAULT_CALENDAR: &str = "chinese";

/// New York, until the household picks its own.
pub const DEFAULT_TZ: &str = "EST5EDT,M3.2.0,M11.1.0";

fn parse_tz(spec: &str) -> Tz {
    let spec = if spec.is_empty() { DEFAULT_TZ } else { spec };
    Tz::parse(spec).unwrap_or_else(|e| {
        log::warn!("time zone {spec:?}: {e}; using {DEFAULT_TZ}");
        Tz::parse(DEFAULT_TZ).expect("default zone parses")
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex32(s: &str) -> Option<[u8; 32]> {
    let mut out = [0u8; 32];
    if s.len() != 64 {
        return None;
    }
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = u8::from_str_radix(s.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

fn session_config(limits: &Limits) -> SessionConfig {
    SessionConfig {
        per_subject: limits.sessions_per_person,
        capacity: limits.sessions,
        ..SessionConfig::new("ml_session")
    }
}

pub fn normalize_name(raw: &str) -> Result<String, ApiError> {
    let name = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let len = name.chars().count();
    if (1..=32).contains(&len) && !name.chars().any(char::is_control) {
        Ok(name)
    } else {
        Err(ApiError::bad_request("Name: 1 to 32 characters"))
    }
}

/// Empty is allowed (no email); otherwise it must look like `x@y`.
pub fn normalize_email(raw: &str) -> Result<String, ApiError> {
    let email = raw.trim().to_lowercase();
    let ok = email.is_empty()
        || (email.len() <= 64
            && !email.chars().any(char::is_whitespace)
            && email.split_once('@').is_some_and(|(user, host)| {
                !user.is_empty() && !host.is_empty() && !host.contains('@')
            }));
    if ok {
        Ok(email)
    } else {
        Err(ApiError::bad_request(
            "That doesn't look like an email address",
        ))
    }
}

pub fn check_pin(pin: &str) -> Result<(), ApiError> {
    if pin::valid(pin) {
        Ok(())
    } else {
        Err(ApiError::bad_request("PIN: 4 to 8 digits"))
    }
}

fn check_role(role: &str) -> Result<&'static str, ApiError> {
    match role {
        ADMIN => Ok(ADMIN),
        MEMBER => Ok(MEMBER),
        GUEST => Ok(GUEST),
        _ => Err(ApiError::bad_request("role must be admin, member or guest")),
    }
}

fn pin_hash(pin: &str, rounds: u32) -> Result<String, ApiError> {
    pin::hash(pin, rounds).map_err(|_| ApiError::new(500, "no_randomness", "No random source"))
}

impl State {
    pub fn new(limits: Limits) -> Self {
        Self {
            people: Vec::new(),
            sessions: Sessions::new(session_config(&limits)),
            next_person: 1,
            signup_closed: false,
            commits: 0,
            kinds: Vec::new(),
            next_kind: 1,
            entries: Vec::new(),
            next_entry: 1,
            timezone: String::new(),
            tz: parse_tz(""),
            journaled: false,
            window_start: i64::MIN,
            history: BTreeMap::new(),
            archived_days: Vec::new(),
            entry_lease: 0,
            timers: Vec::new(),
            next_timer: 1,
            calendar: String::new(),
            sunday_shift: false,
            quarter_moons: false,
            ten_fasting_days: false,
            used_card: false,
            vows: Vec::new(),
            next_vow: 1,
            posts: Vec::new(),
            next_post: 1,
            precepts: Vec::new(),
            texts: Vec::new(),
            next_text: 1,
            practices: Vec::new(),
            next_practice: 1,
            limits,
        }
    }

    pub fn from_stored(stored: StoredState, limits: Limits) -> Self {
        let records = stored
            .sessions
            .iter()
            .filter_map(|s| {
                Some(Record {
                    hash: unhex32(&s.hash)?,
                    subject: s.person,
                    created: s.created,
                    last_seen: s.last_seen,
                })
            })
            .collect();
        Self {
            next_person: stored
                .next_person
                .max(stored.people.iter().map(|p| p.id + 1).max().unwrap_or(1)),
            people: stored.people,
            sessions: Sessions::restore(session_config(&limits), records),
            signup_closed: stored.signup_closed,
            commits: stored.commits,
            next_kind: stored.next_kind.max(1),
            kinds: stored.kinds,
            next_entry: stored
                .next_entry
                .max(stored.entries.iter().map(|e| e.id + 1).max().unwrap_or(1)),
            entries: stored.entries,
            tz: parse_tz(&stored.timezone),
            journaled: false,
            window_start: i64::MIN,
            history: stored
                .archived_days
                .iter()
                .map(|d| ((d.person, i64::from(d.day)), Tally::from_stored(d)))
                .collect(),
            entry_lease: stored.entry_lease,
            next_timer: stored.next_timer.max(1),
            timers: stored.timers,
            calendar: stored.calendar,
            sunday_shift: stored.sunday_shift,
            quarter_moons: stored.quarter_moons,
            ten_fasting_days: stored.ten_fasting_days,
            used_card: stored.used_card,
            archived_days: stored.archived_days,
            timezone: stored.timezone,
            next_vow: stored
                .next_vow
                .max(stored.vows.iter().map(|v| v.id + 1).max().unwrap_or(1)),
            vows: stored.vows,
            next_post: stored
                .next_post
                .max(stored.posts.iter().map(|p| p.id + 1).max().unwrap_or(1)),
            posts: stored.posts,
            precepts: stored.precepts,
            next_text: stored
                .next_text
                .max(stored.texts.iter().map(|t| t.id + 1).max().unwrap_or(1)),
            texts: stored.texts,
            next_practice: stored
                .next_practice
                .max(stored.practices.iter().map(|p| p.id + 1).max().unwrap_or(1)),
            practices: stored.practices,
            limits,
        }
    }

    pub fn to_stored(&self) -> StoredState {
        StoredState {
            schema: SCHEMA as u32,
            next_person: self.next_person,
            people: self.people.clone(),
            sessions: self
                .sessions
                .records()
                .iter()
                .map(|r| StoredSession {
                    hash: hex(&r.hash),
                    person: r.subject,
                    created: r.created,
                    last_seen: r.last_seen,
                })
                .collect(),
            signup_closed: self.signup_closed,
            commits: self.commits,
            kinds: self.kinds.clone(),
            next_kind: self.next_kind,
            entries: if self.journaled {
                Vec::new()
            } else {
                self.entries.clone()
            },
            // With a journal the next id comes from the card at boot; storing
            // it here would rewrite the snapshot on every tap.
            next_entry: if self.journaled {
                self.entry_lease
            } else {
                self.next_entry
            },
            entry_lease: self.entry_lease,
            timezone: self.timezone.clone(),
            archived_days: self.archived_days.clone(),
            used_card: self.used_card,
            timers: self.timers.clone(),
            next_timer: self.next_timer,
            calendar: self.calendar.clone(),
            sunday_shift: self.sunday_shift,
            quarter_moons: self.quarter_moons,
            ten_fasting_days: self.ten_fasting_days,
            vows: self.vows.clone(),
            next_vow: self.next_vow,
            posts: self.posts.clone(),
            next_post: self.next_post,
            precepts: self.precepts.clone(),
            texts: self.texts.clone(),
            next_text: self.next_text,
            practices: self.practices.clone(),
            next_practice: self.next_practice,
        }
    }

    pub fn person(&self, id: u32) -> Option<&StoredPerson> {
        self.people.iter().find(|p| p.id == id)
    }

    pub fn person_mut(&mut self, id: u32) -> Result<&mut StoredPerson, ApiError> {
        self.people
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| ApiError::not_found("No such person"))
    }

    fn name_taken(&self, name: &str, except: Option<u32>) -> bool {
        let wanted = name.to_lowercase();
        self.people
            .iter()
            .any(|p| Some(p.id) != except && p.display_name.to_lowercase() == wanted)
    }

    fn active_admins(&self) -> usize {
        self.people
            .iter()
            .filter(|p| p.role == ADMIN && !p.disabled)
            .count()
    }

    /// Adds someone. `pin` may be empty: they can't sign in, and others
    /// record for them.
    pub fn add_person(
        &mut self,
        name: &str,
        role: &str,
        pin: &str,
        email: &str,
        now: u64,
    ) -> Result<u32, ApiError> {
        let display_name = normalize_name(name)?;
        let role = check_role(role)?;
        let email = normalize_email(email)?;
        if !pin.is_empty() {
            check_pin(pin)?;
        }
        if self.name_taken(&display_name, None) {
            return Err(ApiError::new(
                409,
                "name_taken",
                "Someone already has that name; sign-in picks people by name",
            ));
        }
        if self.people.len() >= self.limits.people {
            return Err(ApiError::new(
                409,
                "full",
                format!("This board holds at most {} people", self.limits.people),
            ));
        }
        let pin = if pin.is_empty() {
            String::new()
        } else {
            pin_hash(pin, self.limits.pin_rounds)?
        };
        let id = self.next_person;
        self.people.push(StoredPerson {
            id,
            display_name,
            role: role.into(),
            pin,
            email,
            created: now,
            last_seen: now,
            disabled: false,
            pinned: Vec::new(),
            calendar: String::new(),
        });
        self.next_person += 1;
        Ok(id)
    }

    /// First-run setup: the first person, an admin with a PIN. Atomic under
    /// the caller's lock, so a second concurrent claim gets 409.
    pub fn setup(&mut self, name: &str, pin: &str, now: u64) -> Result<u32, ApiError> {
        if !self.people.is_empty() {
            return Err(ApiError::new(
                409,
                "already_set_up",
                "Merit Ledger is already set up. Sign in instead",
            ));
        }
        check_pin(pin)?;
        self.add_person(name, ADMIN, pin, "", now)
    }

    /// A visitor makes their own account: always a guest with a PIN. An
    /// admin can promote them.
    pub fn signup(&mut self, name: &str, pin: &str, now: u64) -> Result<u32, ApiError> {
        if self.people.is_empty() {
            return Err(ApiError::new(
                409,
                "setup_needed",
                "Set up Merit Ledger first: the first person becomes admin",
            ));
        }
        if self.signup_closed {
            return Err(ApiError::new(
                403,
                "signup_closed",
                "Sign-up is closed. Ask an admin to add you",
            ));
        }
        check_pin(pin)?;
        self.add_person(name, GUEST, pin, "", now)
    }

    /// The household's calendar system.
    pub fn household_calendar(&self) -> crate::calendar::System {
        let key = if self.calendar.is_empty() {
            DEFAULT_CALENDAR
        } else {
            &self.calendar
        };
        crate::calendar::System::parse(key).unwrap_or(crate::calendar::System::Chinese)
    }

    /// `person`'s calendar: their own choice, else the household's.
    pub fn calendar_of(&self, person: u32) -> crate::calendar::System {
        self.person(person)
            .and_then(|p| crate::calendar::System::parse(&p.calendar))
            .unwrap_or_else(|| self.household_calendar())
    }

    pub fn calendar_options(&self) -> crate::calendar::Options {
        crate::calendar::Options {
            sunday_shift: self.sunday_shift,
            quarter_moons: self.quarter_moons,
            ten_fasting_days: self.ten_fasting_days,
        }
    }

    /// Sets `person`'s own calendar ("" = the household's).
    pub fn set_my_calendar(&mut self, id: u32, calendar: &str) -> Result<(), ApiError> {
        if !calendar.is_empty() && crate::calendar::System::parse(calendar).is_none() {
            return Err(ApiError::bad_request(
                "calendar: chinese, theravada, japanese, none, or \"\" for the household's",
            ));
        }
        self.person_mut(id)?.calendar = calendar.to_string();
        Ok(())
    }

    pub fn update_me(
        &mut self,
        id: u32,
        name: Option<&str>,
        email: Option<&str>,
    ) -> Result<(), ApiError> {
        let name = name.map(normalize_name).transpose()?;
        if let Some(name) = &name {
            if self.name_taken(name, Some(id)) {
                return Err(ApiError::new(
                    409,
                    "name_taken",
                    "Someone already has that name",
                ));
            }
        }
        let email = email.map(normalize_email).transpose()?;
        let person = self.person_mut(id)?;
        if let Some(name) = name {
            person.display_name = name;
        }
        if let Some(email) = email {
            person.email = email;
        }
        Ok(())
    }

    pub fn change_pin(&mut self, id: u32, current: &str, new_pin: &str) -> Result<(), ApiError> {
        check_pin(new_pin)?;
        let rounds = self.limits.pin_rounds;
        let person = self.person_mut(id)?;
        if !person.pin.is_empty() && !pin::verify(current, &person.pin) {
            return Err(ApiError::new(403, "wrong_pin", "Current PIN is wrong"));
        }
        person.pin = pin_hash(new_pin, rounds)?;
        Ok(())
    }

    /// Applies an admin's change to person `id`, acting as `actor`.
    pub fn admin_update(
        &mut self,
        actor: u32,
        id: u32,
        change: &AdminPersonUpdate,
    ) -> Result<(), ApiError> {
        let rounds = self.limits.pin_rounds;
        let role = change.role.as_deref().map(check_role).transpose()?;
        let pin = match change.pin.as_deref() {
            None => None,
            Some("") => Some(String::new()),
            Some(p) => Some(check_pin(p).and_then(|_| pin_hash(p, rounds))?),
        };
        let name = change
            .display_name
            .as_deref()
            .map(normalize_name)
            .transpose()?;
        if let Some(name) = &name {
            if self.name_taken(name, Some(id)) {
                return Err(ApiError::new(
                    409,
                    "name_taken",
                    "Someone already has that name",
                ));
            }
        }
        // Change a copy, check the invariants, then store it.
        let mut person = self.person_mut(id)?.clone();
        let was_active_admin = person.role == ADMIN && !person.disabled;
        if let Some(role) = role {
            person.role = role.into();
        }
        if let Some(disabled) = change.disabled {
            person.disabled = disabled;
        }
        if let Some(pin) = pin {
            person.pin = pin;
        }
        if let Some(name) = name {
            person.display_name = name;
        }
        let still_active_admin = person.role == ADMIN && !person.disabled;
        if was_active_admin && !still_active_admin && self.active_admins() <= 1 {
            return Err(ApiError::new(
                409,
                "last_admin",
                "Someone has to stay an admin. Make another admin first",
            ));
        }
        if id == actor && person.disabled {
            return Err(ApiError::new(
                409,
                "self_disable",
                "You can't disable yourself",
            ));
        }
        let lost_access = person.disabled || person.pin.is_empty();
        *self.person_mut(id)? = person;
        if lost_access {
            self.sessions.revoke_subject(id);
        }
        Ok(())
    }

    /// Removes someone and their sessions. Their entries and personal kinds
    /// are kept (shown as "a former member") or deleted, as the admin chose.
    pub fn remove_person(
        &mut self,
        actor: u32,
        id: u32,
        keep_entries: bool,
    ) -> Result<(), ApiError> {
        if id == actor {
            return Err(ApiError::new(
                409,
                "self_remove",
                "You can't remove yourself",
            ));
        }
        let person = self
            .person(id)
            .ok_or_else(|| ApiError::not_found("No such person"))?;
        if person.role == ADMIN && !person.disabled && self.active_admins() <= 1 {
            return Err(ApiError::new(
                409,
                "last_admin",
                "Someone has to stay an admin",
            ));
        }
        self.people.retain(|p| p.id != id);
        self.sessions.revoke_subject(id);
        if !keep_entries {
            self.entries.retain(|e| e.person != id);
            self.history.retain(|(p, _), _| *p != id);
            self.archived_days.retain(|d| d.person != id);
            self.kinds.retain(|k| k.owner != id);
            self.vows.retain(|v| v.person != id);
            self.posts.retain(|p| p.person != id);
            self.precepts.retain(|p| p.person != id);
            self.practices.retain(|p| p.owner != id);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> Limits {
        Limits {
            pin_rounds: 2,
            ..Limits::board()
        }
    }

    fn update(role: Option<&str>, disabled: Option<bool>) -> AdminPersonUpdate {
        AdminPersonUpdate {
            role: role.map(str::to_string),
            disabled,
            ..Default::default()
        }
    }

    #[test]
    fn setup_once_then_signup_is_guest() {
        let mut s = State::new(limits());
        let dad = s.setup("Dad", "1234", 0).unwrap();
        assert_eq!(s.person(dad).unwrap().role, ADMIN);
        assert_eq!(s.setup("Again", "1234", 0).unwrap_err().status, 409);
        let guest = s.signup("Visitor", "5678", 0).unwrap();
        assert_eq!(s.person(guest).unwrap().role, GUEST);
        s.signup_closed = true;
        assert_eq!(s.signup("Late", "5678", 0).unwrap_err().status, 403);
    }

    #[test]
    fn names_are_unique_ignoring_case_and_spacing() {
        let mut s = State::new(limits());
        s.setup("Dad", "1234", 0).unwrap();
        let err = s.add_person("  dad ", MEMBER, "", "", 0).unwrap_err();
        assert_eq!(err.code, "name_taken");
        let kid = s.add_person("Ada  Lovelace", MEMBER, "", "", 0).unwrap();
        assert_eq!(s.person(kid).unwrap().display_name, "Ada Lovelace");
    }

    #[test]
    fn people_without_a_pin_are_allowed() {
        let mut s = State::new(limits());
        s.setup("Dad", "1234", 0).unwrap();
        let kid = s.add_person("Kid", MEMBER, "", "", 0).unwrap();
        assert!(s.person(kid).unwrap().pin.is_empty());
        // Setting a first PIN needs no current one.
        s.change_pin(kid, "", "4321").unwrap();
        assert!(pin::verify("4321", &s.person(kid).unwrap().pin));
        assert_eq!(s.change_pin(kid, "0000", "1111").unwrap_err().status, 403);
    }

    #[test]
    fn full_board_says_so() {
        let mut s = State::new(Limits {
            people: 2,
            ..limits()
        });
        s.setup("A", "1234", 0).unwrap();
        s.add_person("B", MEMBER, "", "", 0).unwrap();
        let err = s.add_person("C", MEMBER, "", "", 0).unwrap_err();
        assert_eq!((err.status, err.code), (409, "full"));
    }

    #[test]
    fn someone_stays_admin() {
        let mut s = State::new(limits());
        let dad = s.setup("Dad", "1234", 0).unwrap();
        let mum = s.add_person("Mum", MEMBER, "1234", "", 0).unwrap();
        assert_eq!(
            s.admin_update(dad, dad, &update(Some(MEMBER), None))
                .unwrap_err()
                .code,
            "last_admin"
        );
        s.admin_update(dad, mum, &update(Some(ADMIN), None))
            .unwrap();
        s.admin_update(mum, dad, &update(Some(MEMBER), None))
            .unwrap();
        assert_eq!(
            s.remove_person(dad, mum, true).unwrap_err().code,
            "last_admin"
        );
        assert_eq!(
            s.remove_person(mum, mum, true).unwrap_err().code,
            "self_remove"
        );
    }

    #[test]
    fn disabling_or_removing_a_pin_ends_sessions() {
        let mut s = State::new(limits());
        let dad = s.setup("Dad", "1234", 0).unwrap();
        let kid = s.add_person("Kid", MEMBER, "1234", "", 0).unwrap();
        s.sessions.create(kid, Some(10)).unwrap();
        assert_eq!(s.sessions.count(kid), 1);
        let no_pin = AdminPersonUpdate {
            pin: Some(String::new()),
            ..Default::default()
        };
        s.admin_update(dad, kid, &no_pin).unwrap();
        assert_eq!(s.sessions.count(kid), 0);
        assert!(s.person(kid).unwrap().pin.is_empty());
    }

    #[test]
    fn round_trips_through_the_snapshot() {
        let mut s = State::new(limits());
        let dad = s.setup("Dad", "1234", 5).unwrap();
        s.add_person("Kid", MEMBER, "", "kid@home.example", 6)
            .unwrap();
        s.sessions.create(dad, Some(7)).unwrap();
        s.signup_closed = true;
        let back = State::from_stored(s.to_stored(), limits());
        assert_eq!(back.people, s.people);
        assert_eq!(back.sessions.records(), s.sessions.records());
        assert!(back.signup_closed);
        assert_eq!(back.next_person, 3);
    }
}
