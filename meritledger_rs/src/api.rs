//! Routes (`spec/` has the API as it grows).
//!
//! Auth is the `ml_session` cookie. Every non-GET request must come from the
//! board's own pages (`require_same_origin`). Every change is saved before
//! the response is sent; if saving fails the change is rolled back and the
//! client gets 500.
//!
//! Saving (Sprint 5, `docs/SD_CARD.md`): with an SD card, changed entries
//! are appended to the journal and the snapshot is rewritten only when
//! something else changed (people, kinds, settings, the id lease). Without
//! a card, everything is in the snapshot as before.
use crate::journal::Journal;
use crate::ledger::{Actor, Viewer, BACKFILL_DAYS};
use crate::messages::*;
use crate::pin::{self, Guard, GuardLimits};
use crate::sd::Sd;
use crate::state::{self, Limits, State, SCHEMA};
use crate::store::SlotStore;
use crate::transfer;
use crate::tz::{Date, Tz};
use crate::vows;
use miniframework::kv::Kv;
use miniframework::ops::{Check, Status as CheckStatus};
use miniframework::session::require_same_origin;
use miniframework::wire::{self, Format, Schema};
use miniframework::{uptime_ms, wall_ms, ApiError, Message, Reply, Request, Service};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::{Arc, Mutex, MutexGuard};

/// Wall-clock seconds, once SNTP has set the clock.
fn wall() -> Option<u64> {
    wall_ms().map(|ms| ms / 1000)
}

/// Verified against when the picked person doesn't exist, so a wrong id
/// costs the same time as a wrong PIN.
const DUMMY_PIN: &str =
    "pbkdf2-sha256$1600$00000000000000000000000000000000$0000000000000000000000000000000000000000000000000000000000000000";

struct Inner {
    state: State,
    store: SlotStore,
    guard: Guard,
    loaded_slot: u32,
    fallback: bool,
    snapshot_bytes: u32,
    /// CRC of the last snapshot written: a change that leaves it equal
    /// needs no snapshot commit.
    snapshot_crc: u32,
    /// A snapshot commit failed after its journal records were written;
    /// housekeeping retries it.
    snapshot_dirty: bool,
    /// Uptime of the last housekeeping flush.
    last_flush: u64,
    /// The SD card's entry journal, when entries live there.
    journal: Option<Journal>,
    /// Journal records a handler wants written with its change (tombstones
    /// of old entries, imported history).
    pending: Vec<StoredEntry>,
    /// Newest backup file, and how many there are.
    last_backup: String,
    backups: u32,
    /// The day the calendar cache was last warmed (RAM only).
    calendars_warmed: Option<Date>,
}

const BACKUP_DIR: &str = "ml/backup";
/// Vow post bodies, `<id>.md`.
const POSTS_DIR: &str = "ml/posts";
/// Household text bodies, `<id>.md`.
const TEXTS_DIR: &str = "ml/texts";

/// Records that turn `before` into `after`: new and changed entries, and
/// tombstones for removed ones.
fn entry_changes(before: &[StoredEntry], after: &[StoredEntry]) -> Vec<StoredEntry> {
    let old: HashMap<u32, &StoredEntry> = before.iter().map(|e| (e.id, e)).collect();
    let now: HashSet<u32> = after.iter().map(|e| e.id).collect();
    let mut out: Vec<StoredEntry> = after
        .iter()
        .filter(|e| old.get(&e.id).is_none_or(|o| *o != *e))
        .cloned()
        .collect();
    out.extend(
        before
            .iter()
            .filter(|e| !now.contains(&e.id))
            .map(|e| Journal::tombstone(e.id)),
    );
    out
}

/// Who is asking.
#[derive(Clone, Copy, Debug)]
pub struct Caller {
    pub id: u32,
    pub admin: bool,
    pub guest: bool,
}

impl Caller {
    fn actor(&self) -> Actor {
        Actor::Person {
            id: self.id,
            admin: self.admin,
            guest: self.guest,
        }
    }

    fn viewer(&self) -> Viewer {
        Viewer {
            id: self.id,
            admin: self.admin,
        }
    }
}

/// The app behind every route. Shared by the desktop server, the board
/// firmware and the tests.
pub struct App {
    inner: Mutex<Inner>,
    /// Build fingerprint shown in the UI footer (`git rev-parse --short`,
    /// `+` when the tree was dirty).
    pub build: &'static str,
    pub sd: Arc<Sd>,
    /// Exact origins (the Angular dev server) allowed to make changes.
    pub dev_origins: Vec<String>,
    /// Compiled-in code that first-run setup needs, if any.
    pub setup_code: Option<&'static str>,
    #[cfg(feature = "gate")]
    pub gate: Arc<crate::gate::Gate>,
}

impl App {
    /// Opens the state in `kv`. Two unreadable snapshot slots are an error:
    /// the board refuses to start empty over someone's data.
    pub fn open(
        kv: Arc<dyn Kv>,
        sd: Arc<Sd>,
        limits: Limits,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::open_with(kv, sd, limits, GuardLimits::default())
    }

    pub fn open_with(
        kv: Arc<dyn Kv>,
        sd: Arc<Sd>,
        limits: Limits,
        guard: GuardLimits,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (store, loaded) = SlotStore::open(kv, SCHEMA)?;
        let (mut state, loaded_slot, fallback, bytes) = match loaded {
            None => (State::new(limits), 0, false, 0),
            Some(l) => {
                let stored: StoredState = wire::decode(Format::CborInt, &l.payload)
                    .map_err(|e| format!("state snapshot: {e}"))?;
                (
                    State::from_stored(stored, limits),
                    l.slot as u32,
                    l.fallback,
                    l.payload.len() as u32,
                )
            }
        };
        let journal = Self::attach_journal(&mut state, &sd)?;
        let backups = sd.list(BACKUP_DIR).unwrap_or_default();
        let last_backup = backups
            .iter()
            .filter(|n| n.ends_with(".json"))
            .max()
            .cloned()
            .unwrap_or_default();
        let mut inner = Inner {
            state,
            store,
            guard: Guard::new(guard),
            loaded_slot,
            fallback,
            snapshot_bytes: bytes,
            snapshot_crc: 0,
            snapshot_dirty: false,
            last_flush: 0,
            journal,
            pending: Vec::new(),
            backups: backups.iter().filter(|n| n.ends_with(".json")).count() as u32,
            last_backup,
            calendars_warmed: None,
        };
        // With a card, write the snapshot once at boot: attaching the journal
        // may have moved entries off it and renewed the id lease. One write
        // per boot is nothing for flash. Without a card, nothing changed.
        if inner.state.journaled && !inner.state.people.is_empty() {
            Self::commit(&mut inner).map_err(|e| e.message)?;
        } else {
            let current = wire::to_vec(Format::CborInt, &inner.state.to_stored())
                .map_err(|e| e.to_string())?;
            inner.snapshot_crc = crc32fast::hash(&current);
        }
        Ok(Self {
            inner: Mutex::new(inner),
            build: "dev",
            sd,
            dev_origins: Vec::new(),
            setup_code: None,
            #[cfg(feature = "gate")]
            gate: Arc::new(crate::gate::Gate::default()),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Writes the state as the next generation.
    fn commit(inner: &mut Inner) -> Result<(), ApiError> {
        inner.state.commits += 1;
        let bytes =
            wire::to_vec(Format::CborInt, &inner.state.to_stored()).map_err(ApiError::from)?;
        if let Err(e) = inner.store.commit(SCHEMA, &bytes) {
            inner.state.commits -= 1;
            log::error!("state commit failed: {e}");
            return Err(ApiError::new(
                500,
                "storage_failed",
                "Could not save; nothing changed",
            ));
        }
        inner.snapshot_bytes = bytes.len() as u32;
        inner.snapshot_crc = crc32fast::hash(&bytes);
        inner.snapshot_dirty = false;
        inner.state.sessions.saved();
        Ok(())
    }

    /// Moves entries to the SD card's journal, when there is a card: the
    /// snapshot's entries are appended once (migration), the window loads
    /// into RAM, and older days become per-day totals.
    fn attach_journal(
        state: &mut State,
        sd: &Arc<Sd>,
    ) -> Result<Option<Journal>, Box<dyn std::error::Error>> {
        if !sd.present() {
            // A card used before may hold ids up to the lease: start after it.
            if state.used_card {
                state.next_entry = state.next_entry.max(state.entry_lease);
            }
            return Ok(None);
        }
        let err = |e: std::io::Error| format!("SD journal: {e}");
        let mut journal = Journal::open(sd.clone()).map_err(err)?;
        if !state.entries.is_empty() {
            // Moved once: replaying them after a crash is harmless (the last
            // record per id wins).
            log::info!(
                "journal: moving {} entries from the snapshot to the SD card",
                state.entries.len()
            );
            journal.append(&state.entries).map_err(err)?;
        }
        // Written with a card last time, the snapshot's next id is only the
        // lease; the first time, it is the real next id.
        let had_card = state.used_card;
        state.journaled = true;
        state.used_card = true;
        // The card knows which ids are taken (deleted ones too).
        let max_id = journal.info.max_id;
        state.next_entry = if had_card {
            max_id + 1
        } else {
            state.next_entry.max(max_id + 1)
        };
        if state.next_entry >= state.entry_lease {
            state.entry_lease = state.next_entry + 1_000;
        }
        // Today if the clock is set (it rarely is this early), else the
        // newest day in the newest file: a guess that can only keep more
        // in RAM, never less; the tick rolls the window once the clock is set.
        let newest = journal.newest_date().map_err(err)?;
        let reference = state
            .today(wall())
            .or_else(|| newest.as_deref().and_then(Date::parse));
        let start = reference.map_or(i64::MIN, |d| d.days() - BACKFILL_DAYS);
        let cut = if start == i64::MIN {
            String::new()
        } else {
            Date::from_days(start).iso()
        };
        state.entries.clear();
        let mut window = Vec::new();
        let mut older = Vec::new();
        journal
            .for_each(|e| {
                if e.date >= cut {
                    window.push(e);
                } else {
                    older.push(e);
                    // Fold in batches so the old ones never all sit in RAM.
                    if older.len() >= 1_024 {
                        state.fold_into_history(&older);
                        older.clear();
                    }
                }
            })
            .map_err(err)?;
        state.fold_into_history(&older);
        window.sort_by_key(|e| e.id);
        state.entries = window;
        state.window_start = start;
        log::info!(
            "journal: {} files, {} records; {} entries in the window, {} older days as totals",
            journal.info.files,
            journal.info.records,
            state.entries.len(),
            state.history.len()
        );
        Ok(Some(journal))
    }

    /// Saves a change: journal records with a card (then the snapshot only
    /// if it changed), the snapshot without one.
    fn save(inner: &mut Inner, before: &[StoredEntry]) -> Result<(), ApiError> {
        if !inner.state.journaled {
            return Self::commit(inner);
        }
        let mut records = std::mem::take(&mut inner.pending);
        records.extend(entry_changes(before, &inner.state.entries));
        if !records.is_empty() {
            let journal = inner
                .journal
                .as_mut()
                .ok_or_else(|| ApiError::new(500, "storage_failed", "The SD journal isn't open"))?;
            journal.append(&records).map_err(|e| {
                log::error!("journal append failed: {e}");
                ApiError::new(
                    500,
                    "storage_failed",
                    "Could not save to the SD card; nothing changed",
                )
            })?;
        }
        let bytes =
            wire::to_vec(Format::CborInt, &inner.state.to_stored()).map_err(ApiError::from)?;
        if crc32fast::hash(&bytes) != inner.snapshot_crc {
            // The journal already has the entries: a failed snapshot commit
            // must not undo them in RAM. Retry it in housekeeping.
            if let Err(e) = Self::commit(inner) {
                log::error!("snapshot commit after journal: {}", e.message);
                inner.snapshot_dirty = true;
            }
        }
        Ok(())
    }

    /// Runs `f` and saves; on any error the state is rolled back.
    fn mutate<R>(&self, f: impl FnOnce(&mut Inner) -> Result<R, ApiError>) -> Result<R, ApiError> {
        let mut inner = self.lock();
        let backup = inner.state.to_stored();
        let entries = inner.state.entries.clone();
        let history = inner.state.history.clone();
        let (journaled, window_start) = (inner.state.journaled, inner.state.window_start);
        let result = f(&mut inner).and_then(|r| Self::save(&mut inner, &entries).map(|_| r));
        if result.is_err() {
            let limits = inner.state.limits.clone();
            let mut state = State::from_stored(backup, limits);
            state.entries = entries;
            state.history = history;
            state.journaled = journaled;
            state.window_start = window_start;
            inner.state = state;
            inner.pending.clear();
        }
        result
    }

    /// Housekeeping, called from the main loop once a minute or so:
    /// persists session touches (flash wear), moves the window at the turn
    /// of the day, retries a failed snapshot commit, and makes the nightly
    /// backup.
    pub fn tick(&self) {
        let now = uptime_ms();
        {
            let mut inner = self.lock();
            if now.saturating_sub(inner.last_flush) < 60_000 {
                return;
            }
            inner.last_flush = now;
            let mut commit = inner.state.sessions.dirty() || inner.snapshot_dirty;
            inner.state.sessions.prune(wall());
            if let Some(today) = inner.state.today(wall()) {
                commit |= inner.state.roll_window(today);
            }
            if commit {
                if let Err(e) = Self::commit(&mut inner) {
                    log::warn!("housekeeping commit: {}", e.message);
                }
            }
        }
        self.backup_if_due();
        self.warm_calendars();
    }

    /// Computes last, this and next year's calendars for every system in
    /// use, once a day, so no one waits for the astronomy (about 0.6 s a
    /// year on the S3, whose FPU can't do f64). Outside the app lock.
    fn warm_calendars(&self) {
        let (systems, options, tz, today) = {
            let mut inner = self.lock();
            let Some(today) = inner.state.today(wall()) else {
                return;
            };
            if inner.calendars_warmed == Some(today) {
                return;
            }
            inner.calendars_warmed = Some(today);
            let mut systems = vec![inner.state.household_calendar()];
            for p in &inner.state.people {
                let s = inner.state.calendar_of(p.id);
                if !systems.contains(&s) {
                    systems.push(s);
                }
            }
            (
                systems,
                inner.state.calendar_options(),
                inner.state.tz.clone(),
                today,
            )
        };
        let started = std::time::Instant::now();
        // Last year too: the calendar feeds reach a year back.
        for year in [today.year - 1, today.year, today.year + 1] {
            let (from, to) = (
                Date {
                    year,
                    month: 1,
                    day: 1,
                },
                Date {
                    year,
                    month: 12,
                    day: 31,
                },
            );
            crate::calendar::chinese_months(&tz, from, to);
            for s in &systems {
                crate::calendar::observances(*s, options, &tz, from, to);
            }
        }
        log::info!(
            "calendar: warmed {} system(s) for {} to {} in {} ms",
            systems.len(),
            today.year - 1,
            today.year + 1,
            started.elapsed().as_millis()
        );
    }

    /// The nightly backup: after 03:00 household time, once a day, with a
    /// card. Built under the lock (fast, in RAM); written outside it.
    fn backup_if_due(&self) {
        let Some(t) = wall() else { return };
        if !self.sd.present() {
            return;
        }
        {
            let inner = self.lock();
            let local = t as i64 + inner.state.tz.offset_at(t as i64);
            if local.rem_euclid(86_400) < 3 * 3600 {
                return;
            }
            let name = format!("{}.json", inner.state.tz.local_date(t as i64).iso());
            if inner.last_backup >= name {
                return;
            }
        }
        if let Err(e) = self.backup_now(t) {
            log::error!("backup: {e}");
        }
    }

    /// Writes today's backup (`ml/backup/YYYY-MM-DD.json`, replacing one
    /// from earlier today) and prunes old ones. Returns the file name.
    pub fn backup_now(&self, t: u64) -> Result<String, String> {
        let (name, bytes) = {
            let inner = self.lock();
            let name = format!("{}.json", inner.state.tz.local_date(t as i64).iso());
            let (post, text) = self.bodies();
            let bodies = transfer::Bodies {
                post: &post,
                text: &text,
            };
            let doc = transfer::export(&inner.state, inner.journal.as_ref(), t, &bodies)
                .map_err(|e| format!("reading the journal: {e}"))?;
            let bytes = wire::to_vec(Format::Json, &doc).map_err(|e| format!("encoding: {e}"))?;
            (name, bytes)
        };
        self.sd
            .write_file(&format!("{BACKUP_DIR}/{name}"), &bytes)
            .map_err(|e| format!("writing {name}: {e}"))?;
        let names = self.sd.list(BACKUP_DIR).unwrap_or_default();
        for old in transfer::backups_to_delete(&names) {
            if let Err(e) = self.sd.remove(&format!("{BACKUP_DIR}/{old}")) {
                log::warn!("removing backup {old}: {e}");
            }
        }
        let count = self
            .sd
            .list(BACKUP_DIR)
            .map(|n| n.iter().filter(|n| n.ends_with(".json")).count() as u32)
            .unwrap_or(0);
        log::info!("backup {name}: {} bytes, {count} kept", bytes.len());
        let mut inner = self.lock();
        inner.last_backup = name.clone();
        inner.backups = count;
        Ok(name)
    }

    fn status(&self, req: &Request<'_>) -> Status {
        let inner = self.lock();
        Status {
            app: "meritledger".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            build: self.build.into(),
            clock_synced: wall().is_some(),
            secure: req.secure,
            setup_needed: inner.state.people.is_empty(),
            signup_open: !inner.state.signup_closed,
            setup_code_required: self.setup_code.is_some(),
        }
    }

    fn storage(&self) -> StorageInfo {
        let sd = self.sd.info();
        let inner = self.lock();
        StorageInfo {
            sd_present: sd.present,
            sd_total_bytes: sd.total_bytes,
            sd_free_bytes: sd.free_bytes,
            generation: inner.store.generation,
            snapshot_bytes: inner.snapshot_bytes,
            commits: inner.state.commits,
            people: inner.state.people.len() as u32,
            max_people: inner.state.limits.people as u32,
            sessions: inner.state.sessions.records().len() as u32,
            max_sessions: inner.state.limits.sessions as u32,
            loaded_slot: inner.loaded_slot,
            fallback_used: inner.fallback,
            journaled: inner.state.journaled,
            journal_files: inner.journal.as_ref().map_or(0, |j| j.info.files),
            journal_bytes: inner.journal.as_ref().map_or(0, |j| j.info.bytes),
            journal_records: inner.journal.as_ref().map_or(0, |j| j.info.records),
            torn_bytes: inner.journal.as_ref().map_or(0, |j| j.info.torn_bytes),
            damaged_files: inner
                .journal
                .as_ref()
                .map_or_else(Vec::new, |j| j.info.damaged_files.clone()),
            window_entries: inner.state.entries.len() as u32,
            history_days: inner.state.history.len() as u32,
            last_backup: inner.last_backup.clone(),
            backups: inner.backups,
            card_missing: inner.state.used_card && !sd.present,
        }
    }

    pub fn caller(&self, req: &Request<'_>) -> Result<Caller, ApiError> {
        let mut inner = self.lock();
        let id = inner
            .state
            .sessions
            .check(req, wall())
            .ok_or_else(|| ApiError::unauthorized("Sign in first"))?;
        match inner.state.person(id) {
            Some(p) if !p.disabled && !p.pin.is_empty() => Ok(Caller {
                id,
                admin: p.role == state::ADMIN,
                guest: p.role == state::GUEST,
            }),
            _ => {
                inner.state.sessions.revoke(req);
                Err(ApiError::unauthorized("Sign in first"))
            }
        }
    }

    fn admin(&self, req: &Request<'_>) -> Result<Caller, ApiError> {
        let caller = self.caller(req)?;
        if caller.admin {
            Ok(caller)
        } else {
            Err(ApiError::new(
                403,
                "admin_only",
                "Only an admin can do that",
            ))
        }
    }

    fn me(state: &State, id: u32) -> Result<Me, ApiError> {
        let p = state
            .person(id)
            .ok_or_else(|| ApiError::unauthorized("Sign in first"))?;
        Ok(Me {
            id: p.id,
            display_name: p.display_name.clone(),
            role: p.role.clone(),
            has_pin: !p.pin.is_empty(),
            email: p.email.clone(),
            sessions: state.sessions.count(id) as u32,
            created: p.created,
            calendar: p.calendar.clone(),
        })
    }

    fn people(state: &State) -> PeopleList {
        PeopleList {
            people: state
                .people
                .iter()
                .filter(|p| !p.disabled)
                .map(|p| PersonView {
                    id: p.id,
                    display_name: p.display_name.clone(),
                    role: p.role.clone(),
                    can_sign_in: !p.pin.is_empty(),
                })
                .collect(),
        }
    }

    fn admin_people(state: &State) -> AdminPeopleList {
        AdminPeopleList {
            people: state
                .people
                .iter()
                .map(|p| AdminPersonView {
                    id: p.id,
                    display_name: p.display_name.clone(),
                    role: p.role.clone(),
                    has_pin: !p.pin.is_empty(),
                    disabled: p.disabled,
                    email: p.email.clone(),
                    created: p.created,
                    last_seen: p.last_seen,
                    sessions: state.sessions.count(p.id) as u32,
                })
                .collect(),
        }
    }

    fn sign_in(
        inner: &mut Inner,
        id: u32,
        req: &Request<'_>,
        reply: &mut Reply<'_>,
    ) -> Result<(), ApiError> {
        let now = wall();
        let token = inner
            .state
            .sessions
            .create(id, now)
            .map_err(|_| ApiError::new(500, "no_randomness", "No random source"))?;
        if let (Some(now), Ok(p)) = (now, inner.state.person_mut(id)) {
            p.last_seen = now;
        }
        reply.header(
            "Set-Cookie",
            inner.state.sessions.set_cookie(&token, req.secure),
        );
        Ok(())
    }

    fn pin_sign_in(
        &self,
        body: &PinSignIn,
        req: &Request<'_>,
        reply: &mut Reply<'_>,
    ) -> Result<Me, ApiError> {
        let mono = uptime_ms();
        let wrong = || ApiError::new(401, "wrong_pin", "Wrong PIN");
        let mut inner = self.lock();
        let person = inner
            .state
            .person(body.person)
            .map(|p| (p.id, p.pin.clone(), p.disabled));
        let wait = inner.guard.wait(person.as_ref().map(|p| p.0), mono);
        if wait > 0 {
            let secs = wait.div_ceil(1000);
            reply.header("Retry-After", secs.to_string());
            return Err(ApiError::new(
                429,
                "locked",
                format!("Too many wrong PINs. Try again in {secs} s"),
            ));
        }
        let Some((id, stored, disabled)) = person else {
            pin::verify(&body.pin, DUMMY_PIN);
            inner.guard.failed(None, mono);
            return Err(wrong());
        };
        if stored.is_empty() {
            return Err(ApiError::new(
                403,
                "no_pin",
                "This person has no PIN yet. An admin can set one",
            ));
        }
        if !pin::verify(&body.pin, &stored) {
            inner.guard.failed(Some(id), mono);
            return Err(wrong());
        }
        if disabled {
            return Err(ApiError::new(403, "disabled", "This account is disabled"));
        }
        inner.guard.succeeded(id);
        drop(inner);
        self.mutate(|inner| {
            Self::sign_in(inner, id, req, reply)?;
            Self::me(&inner.state, id)
        })
    }

    fn settings(state: &State) -> Settings {
        Settings {
            calendar: state.household_calendar().key().into(),
            sunday_shift: state.sunday_shift,
            quarter_moons: state.quarter_moons,
            ten_fasting_days: state.ten_fasting_days,
            signup_open: !state.signup_closed,
            timezone: if state.timezone.is_empty() {
                state::DEFAULT_TZ.into()
            } else {
                state.timezone.clone()
            },
            timezone_label: crate::zones::label_of(if state.timezone.is_empty() {
                state::DEFAULT_TZ
            } else {
                &state.timezone
            })
            .unwrap_or("")
            .into(),
        }
    }

    /// Who is looking, signed in or not.
    fn viewer(&self, req: &Request<'_>) -> Viewer {
        self.caller(req).map_or(
            Viewer {
                id: 0,
                admin: false,
            },
            |c| c.viewer(),
        )
    }

    /// Records a practice from the board's own touch menu (Sprint 13):
    /// household trust, no session. Returns the entry id.
    pub fn record_from_board(&self, person: u32, kind: &str) -> Result<u32, ApiError> {
        let input = EntryInput {
            person,
            kind: kind.into(),
            ..Default::default()
        };
        self.mutate(|inner| inner.state.record(Actor::Board, &input, wall()))
    }

    /// One `{id}` path segment after `prefix`.
    fn id_after(req: &Request<'_>, prefix: &str) -> Option<String> {
        match req.rest(prefix).as_deref() {
            Some([id]) => Some(id.to_string()),
            _ => None,
        }
    }

    /// `GET /cal/<system>.ics`: two months back and a year and a bit ahead, with
    /// the household's options. Public: calendars hold no one's data.
    fn calendar_feed(&self, req: &Request<'_>, reply: &mut Reply<'_>) -> Result<(), ApiError> {
        let name = req
            .path
            .trim_start_matches("/cal/")
            .trim_end_matches(".ics");
        let system = crate::calendar::System::parse(name)
            .filter(|s| *s != crate::calendar::System::None)
            .ok_or_else(|| {
                ApiError::not_found("No such calendar: try chinese, theravada or japanese")
            })?;
        let now = wall().ok_or_else(|| {
            ApiError::new(
                503,
                "clock_not_set",
                "The board's clock isn't set yet; try again in a minute",
            )
        })?;
        let text = {
            let inner = self.lock();
            let today = inner.state.tz.local_date(now as i64);
            let list = crate::calendar::observances(
                system,
                inner.state.calendar_options(),
                &inner.state.tz,
                // Two months back is plenty for a phone's calendar, and a
                // smaller feed loads faster over a weak Wi-Fi signal.
                today.add_days(-60),
                today.add_days(400),
            );
            let title = match system {
                crate::calendar::System::Theravada => "Merit Ledger (Theravada-inspired)",
                crate::calendar::System::Chinese => "Merit Ledger (Chinese-inspired)",
                _ => "Merit Ledger (Japanese-inspired)",
            };
            crate::ics::feed(title, &list, now)
        };
        reply.owned(200, "text/calendar; charset=utf-8", text.into_bytes());
        reply.header("Cache-Control", "public, max-age=3600");
        Ok(())
    }

    /// Export, import, CSV and backups (Sprint 5), or `Ok(false)`.
    fn route_transfer(&self, req: &Request<'_>, reply: &mut Reply<'_>) -> Result<bool, ApiError> {
        let path = req.path;
        let read_failed = |e: std::io::Error| {
            log::error!("reading the journal: {e}");
            ApiError::new(500, "storage_failed", "Could not read the SD card")
        };
        match (req.method, path) {
            ("GET", "/api/v1/admin/export.json") => {
                self.admin(req)?;
                let now = wall().unwrap_or(0);
                let (bytes, name) = {
                    let inner = self.lock();
                    let (post, text) = self.bodies();
                    let bodies = transfer::Bodies {
                        post: &post,
                        text: &text,
                    };
                    let doc = transfer::export(&inner.state, inner.journal.as_ref(), now, &bodies)
                        .map_err(read_failed)?;
                    let day = inner
                        .state
                        .today(wall())
                        .map_or_else(|| "export".to_string(), |d| d.iso());
                    (
                        wire::to_vec(Format::Json, &doc).map_err(ApiError::from)?,
                        day,
                    )
                };
                reply.owned(200, "application/json", bytes);
                reply.header(
                    "Content-Disposition",
                    format!("attachment; filename=\"meritledger-{name}.json\""),
                );
            }
            ("GET", _) if path.starts_with("/api/v1/people/") && path.ends_with("/entries.csv") => {
                let c = self.caller(req)?;
                let person: u32 = match req.rest("/api/v1/people").as_deref() {
                    Some([id, "entries.csv"]) => id.parse().ok(),
                    _ => None,
                }
                .ok_or_else(|| ApiError::not_found("No such person"))?;
                if person != c.id && !c.admin {
                    return Err(ApiError::new(
                        403,
                        "forbidden",
                        "You can download your own entries",
                    ));
                }
                let (csv, name) = {
                    let inner = self.lock();
                    let entries =
                        transfer::entries_of(&inner.state, inner.journal.as_ref(), person, None)
                            .map_err(read_failed)?;
                    let name = inner
                        .state
                        .person(person)
                        .map_or_else(|| "entries".to_string(), |p| p.display_name.clone());
                    (
                        transfer::csv(&inner.state, &entries, c.viewer(), person),
                        name,
                    )
                };
                reply.owned(200, "text/csv; charset=utf-8", csv.into_bytes());
                let safe: String = name
                    .chars()
                    .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
                    .collect();
                reply.header(
                    "Content-Disposition",
                    format!("attachment; filename=\"meritledger-{safe}.csv\""),
                );
            }
            ("POST", "/api/v1/admin/import") => {
                self.admin(req)?;
                let dry_run = req.param("dry_run").is_some_and(|v| v != "0");
                let mut body = Vec::new();
                req.body_reader()
                    .read_to_end(&mut body)
                    .map_err(|e| ApiError::bad_request(format!("Reading the upload: {e}")))?;
                let doc: ExportDoc = wire::decode(Format::Json, &body).map_err(|e| {
                    ApiError::bad_request(format!("Not a Merit Ledger export: {e}"))
                })?;
                let existing: HashSet<transfer::Fingerprint> = {
                    let inner = self.lock();
                    transfer::all_entries(&inner.state, inner.journal.as_ref())
                        .map_err(read_failed)?
                        .iter()
                        .map(transfer::fingerprint)
                        .collect()
                };
                let report = if dry_run {
                    let mut inner = self.lock();
                    inner
                        .state
                        .import(&doc, &existing, false, wall())
                        .map_err(ApiError::bad_request)?
                        .0
                } else {
                    self.mutate(|inner| {
                        let (report, older) = inner
                            .state
                            .import(&doc, &existing, true, wall())
                            .map_err(ApiError::bad_request)?;
                        inner.pending.extend(older);
                        self.spill_bodies(&mut inner.state)?;
                        Ok(report)
                    })?
                };
                reply.wire(req, &report);
            }
            ("POST", "/api/v1/admin/backups") => {
                self.admin(req)?;
                if !self.sd.present() {
                    return Err(ApiError::new(409, "no_card", "Backups need an SD card"));
                }
                let now = wall().ok_or_else(|| {
                    ApiError::new(
                        503,
                        "clock_not_set",
                        "The board's clock isn't set yet; try again in a minute",
                    )
                })?;
                self.backup_now(now).map_err(|e| {
                    ApiError::new(500, "storage_failed", format!("Backup failed: {e}"))
                })?;
                let names = self
                    .sd
                    .list(BACKUP_DIR)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|n| n.ends_with(".json"))
                    .rev()
                    .collect();
                reply.wire_status(req, 201, &BackupList { names });
            }
            ("GET", "/api/v1/admin/backups") => {
                self.admin(req)?;
                let names = self
                    .sd
                    .list(BACKUP_DIR)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|n| n.ends_with(".json"))
                    .rev()
                    .collect();
                reply.wire(req, &BackupList { names });
            }
            ("GET", _) if path.starts_with("/api/v1/admin/backups/") => {
                self.admin(req)?;
                let name = Self::id_after(req, "/api/v1/admin/backups")
                    .filter(|n| {
                        n.len() == 15 && n.ends_with(".json") && Date::parse(&n[..10]).is_some()
                    })
                    .ok_or_else(|| ApiError::not_found("No such backup"))?;
                let file = self
                    .sd
                    .path(&format!("{BACKUP_DIR}/{name}"))
                    .and_then(std::fs::File::open)
                    .map_err(|_| ApiError::not_found("No such backup"))?;
                let len = file.metadata().map_err(read_failed)?.len();
                reply.stream(200, "application/json", len, Box::new(file));
                reply.header(
                    "Content-Disposition",
                    format!("attachment; filename=\"meritledger-{name}\""),
                );
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Practice routes (Sprint 3), or `Ok(false)` when `path` isn't one.
    fn route_practice(&self, req: &Request<'_>, reply: &mut Reply<'_>) -> Result<bool, ApiError> {
        let path = req.path;
        match (req.method, path) {
            ("GET", "/api/v1/kinds") => {
                let c = self.caller(req)?;
                reply.wire(
                    req,
                    &KindList {
                        kinds: self.lock().state.kinds_for(c.id),
                    },
                );
            }
            ("POST", "/api/v1/kinds") => {
                let c = self.caller(req)?;
                let body: KindInput = req.decode()?;
                let kinds = self.mutate(|inner| {
                    inner.state.add_kind(c.actor(), &body)?;
                    Ok(inner.state.kinds_for(c.id))
                })?;
                reply.wire_status(req, 201, &KindList { kinds });
            }
            ("PATCH", _) if path.starts_with("/api/v1/kinds/") => {
                let c = self.caller(req)?;
                let key = Self::id_after(req, "/api/v1/kinds")
                    .ok_or_else(|| ApiError::not_found("No such practice"))?;
                let body: KindUpdate = req.decode()?;
                let kinds = self.mutate(|inner| {
                    inner.state.update_kind(c.actor(), &key, &body)?;
                    Ok(inner.state.kinds_for(c.id))
                })?;
                reply.wire(req, &KindList { kinds });
            }
            ("GET", "/api/v1/timezones") => {
                let zones = crate::zones::ZONES
                    .iter()
                    .map(|(id, label, _)| ZoneView {
                        id: (*id).into(),
                        label: (*label).into(),
                    })
                    .collect();
                reply.wire(req, &ZoneList { zones });
            }
            ("GET", "/api/v1/calendar") => {
                // Public. The viewer's own calendar, or `?system=`.
                let viewer = self.viewer(req);
                let inner = self.lock();
                let system = match req.param("system") {
                    Some(s) => crate::calendar::System::parse(&s).ok_or_else(|| {
                        ApiError::bad_request("system: chinese, theravada, japanese or none")
                    })?,
                    None if viewer.id != 0 => inner.state.calendar_of(viewer.id),
                    None => inner.state.household_calendar(),
                };
                let date = |name: &str| -> Result<Option<Date>, ApiError> {
                    req.param(name)
                        .map(|d| {
                            Date::parse(&d)
                                .ok_or_else(|| ApiError::bad_request("Dates look like 2026-10-18"))
                        })
                        .transpose()
                };
                let today = inner.state.today(wall());
                let from = date("from")?
                    .or(today.map(|t| Date { day: 1, ..t }))
                    .ok_or_else(|| {
                        ApiError::bad_request("Give ?from= and ?to= (the clock isn't set)")
                    })?;
                let to = date("to")?.unwrap_or(from.add_days(41));
                if to < from || to.days() - from.days() > 120 {
                    return Err(ApiError::bad_request("A calendar range is 1 to 120 days"));
                }
                let view = crate::calendar::calendar_view(
                    system,
                    inner.state.calendar_options(),
                    &inner.state.tz,
                    from,
                    to,
                );
                reply.wire(req, &view);
            }
            ("GET", "/api/v1/timers") => {
                let c = self.caller(req)?;
                let timers = self.lock().state.timers_for(c.id);
                reply.wire(req, &TimerList { timers });
            }
            ("POST", "/api/v1/timers") => {
                let c = self.caller(req)?;
                let body: TimerInput = req.decode()?;
                let timers = self.mutate(|inner| {
                    inner.state.add_timer(c.actor(), &body)?;
                    Ok(inner.state.timers_for(c.id))
                })?;
                reply.wire_status(req, 201, &TimerList { timers });
            }
            ("PUT", _) if path.starts_with("/api/v1/timers/") => {
                let c = self.caller(req)?;
                let key = Self::id_after(req, "/api/v1/timers")
                    .ok_or_else(|| ApiError::not_found("No such timer"))?;
                let body: TimerInput = req.decode()?;
                let timers = self.mutate(|inner| {
                    inner.state.replace_timer(c.actor(), &key, &body)?;
                    Ok(inner.state.timers_for(c.id))
                })?;
                reply.wire(req, &TimerList { timers });
            }
            ("PUT", "/api/v1/me/pins") => {
                let c = self.caller(req)?;
                let body: PinsInput = req.decode()?;
                let kinds = self.mutate(|inner| {
                    inner.state.set_pins(c.id, &body.kinds)?;
                    Ok(inner.state.kinds_for(c.id))
                })?;
                reply.wire(req, &KindList { kinds });
            }
            ("GET", "/api/v1/today") => {
                let c = self.caller(req)?;
                let person = req
                    .param_num::<u32>("person")?
                    .filter(|p| *p != 0)
                    .unwrap_or(c.id);
                let today = self.lock().state.today_view(c.viewer(), person, wall())?;
                reply.wire(req, &today);
            }
            ("POST", "/api/v1/entries") => {
                let c = self.caller(req)?;
                let body: EntryInput = req.decode()?;
                let today = self.mutate(|inner| {
                    let id = inner.state.record(c.actor(), &body, wall())?;
                    let person = inner
                        .state
                        .entries
                        .iter()
                        .find(|e| e.id == id)
                        .map_or(c.id, |e| e.person);
                    inner.state.today_view(c.viewer(), person, wall())
                })?;
                reply.wire_status(req, 201, &today);
            }
            (method @ ("PATCH" | "DELETE"), _) if path.starts_with("/api/v1/entries/") => {
                let c = self.caller(req)?;
                let id: u32 = Self::id_after(req, "/api/v1/entries")
                    .and_then(|id| id.parse().ok())
                    .ok_or_else(|| ApiError::not_found("No such entry"))?;
                let body: Option<EntryUpdate> = if method == "PATCH" {
                    Some(req.decode()?)
                } else {
                    None
                };
                let today = self.mutate(|inner| {
                    let person = inner
                        .state
                        .entries
                        .iter()
                        .find(|e| e.id == id)
                        .map_or(c.id, |e| e.person);
                    match &body {
                        Some(change) => inner.state.update_entry(c.viewer(), id, change, wall())?,
                        None => inner.state.delete_entry(c.viewer(), id)?,
                    }
                    inner.state.today_view(c.viewer(), person, wall())
                })?;
                reply.wire(req, &today);
            }
            ("GET", _) if path.starts_with("/api/v1/people/") && path.ends_with("/profile") => {
                // Public: anyone on the Wi-Fi sees what isn't private.
                let viewer = self.viewer(req);
                let person: u32 = match req.rest("/api/v1/people").as_deref() {
                    Some([id, "profile"]) => id.parse().ok(),
                    _ => None,
                }
                .ok_or_else(|| ApiError::not_found("No such person"))?;
                let profile = self.lock().state.profile(viewer, person, wall())?;
                reply.wire(req, &profile);
            }
            ("GET", "/api/v1/feed") => {
                let viewer = self.viewer(req);
                let offset = req.param_num::<usize>("offset")?.unwrap_or(0);
                let limit = req.param_num::<usize>("limit")?.unwrap_or(30).clamp(1, 100);
                let feed = self.lock().state.feed(viewer, offset, limit);
                reply.wire(req, &feed);
            }
            ("POST", _) if path.starts_with("/api/v1/entries/") && path.ends_with("/rejoice") => {
                let c = self.caller(req)?;
                let id: u32 = match req.rest("/api/v1/entries").as_deref() {
                    Some([id, "rejoice"]) => id.parse().ok(),
                    _ => None,
                }
                .ok_or_else(|| ApiError::not_found("No such entry"))?;
                let view = self.mutate(|inner| {
                    inner.state.rejoice(c.actor(), id, wall())?;
                    let entry = inner
                        .state
                        .entries
                        .iter()
                        .find(|e| e.id == id)
                        .ok_or_else(|| ApiError::not_found("No such entry"))?;
                    inner
                        .state
                        .entry_view(c.viewer(), entry)
                        .ok_or_else(|| ApiError::not_found("No such entry"))
                })?;
                reply.wire(req, &view);
            }
            ("GET", _) if path.starts_with("/api/v1/people/") && path.ends_with("/entries") => {
                // Public, like profiles: private entries only to their person
                // and admins.
                let viewer = self.viewer(req);
                let person: u32 = match req.rest("/api/v1/people").as_deref() {
                    Some([id, "entries"]) => id.parse().ok(),
                    _ => None,
                }
                .ok_or_else(|| ApiError::not_found("No such person"))?;
                let inner = self.lock();
                let today = inner.state.today(wall());
                let date = |name: &str, fallback: Option<Date>| -> Result<Date, ApiError> {
                    match req.param(name) {
                        Some(d) => Date::parse(&d)
                            .ok_or_else(|| ApiError::bad_request("Dates look like 2026-10-18")),
                        None => fallback.ok_or_else(|| {
                            ApiError::bad_request(format!("Give ?{name}= (the clock is not set)"))
                        }),
                    }
                };
                let to = date("to", today)?;
                let from = date("from", Some(to.add_days(-29)))?;
                let limit = req.param_num::<usize>("limit")?.unwrap_or(200).min(1000);
                let entries = if inner.state.journaled && from.days() < inner.state.window_start {
                    // Before the window: read the journal back.
                    let all = transfer::entries_of(
                        &inner.state,
                        inner.journal.as_ref(),
                        person,
                        Some((from, to)),
                    )
                    .map_err(|e| {
                        log::error!("reading the journal: {e}");
                        ApiError::new(500, "storage_failed", "Could not read the SD card")
                    })?;
                    inner.state.views_of(&all, viewer, person, from, to, limit)
                } else {
                    inner.state.entries_for(viewer, person, from, to, limit)
                };
                reply.wire(req, &EntryList { entries });
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// A post's markdown, from the card or the snapshot.
    fn post_body(sd: &Sd, post: &StoredPost) -> String {
        if !post.on_card {
            return post.body.clone();
        }
        match sd.read(&format!("{POSTS_DIR}/{}.md", post.id)) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(e) => {
                log::warn!("post {}: {e}", post.id);
                "*(This post's text is on the SD card, which isn't readable right now.)*".into()
            }
        }
    }

    /// A household text's markdown, from the card or the snapshot.
    fn text_body(sd: &Sd, text: &StoredText) -> String {
        if !text.on_card {
            return text.body.clone();
        }
        match sd.read(&format!("{TEXTS_DIR}/{}.md", text.id)) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(e) => {
                log::warn!("text {}: {e}", text.id);
                "*(This text is on the SD card, which isn't readable right now.)*".into()
            }
        }
    }

    /// Writes a body to `dir/<id>.md` when it belongs on the card.
    fn write_body(&self, dir: &str, write: vows::BodyWrite) -> Result<(), ApiError> {
        if let vows::BodyWrite::Card { id, body } = write {
            self.sd
                .write_file(&format!("{dir}/{id}.md"), body.as_bytes())
                .map_err(|e| {
                    log::error!("writing {dir}/{id}.md: {e}");
                    ApiError::new(
                        500,
                        "storage_failed",
                        "Could not save to the SD card; nothing changed",
                    )
                })?;
        }
        Ok(())
    }

    fn remove_bodies(&self, dir: &str, ids: &[u32]) {
        for id in ids {
            if let Err(e) = self.sd.remove(&format!("{dir}/{id}.md")) {
                log::warn!("removing {dir}/{id}.md: {e}");
            }
        }
    }

    /// Imported posts and texts arrive with their bodies: move them to the card.
    fn spill_bodies(&self, state: &mut State) -> Result<(), ApiError> {
        if !self.sd.present() {
            return Ok(());
        }
        for post in state
            .posts
            .iter_mut()
            .filter(|p| !p.on_card && !p.body.is_empty())
        {
            let body = std::mem::take(&mut post.body);
            post.on_card = true;
            self.write_body(POSTS_DIR, vows::BodyWrite::Card { id: post.id, body })?;
        }
        for text in state
            .texts
            .iter_mut()
            .filter(|t| !t.on_card && !t.body.is_empty())
        {
            let body = std::mem::take(&mut text.body);
            text.on_card = true;
            self.write_body(TEXTS_DIR, vows::BodyWrite::Card { id: text.id, body })?;
        }
        Ok(())
    }

    fn bodies(
        &self,
    ) -> (
        impl Fn(&StoredPost) -> String + '_,
        impl Fn(&StoredText) -> String + '_,
    ) {
        (
            move |p: &StoredPost| Self::post_body(&self.sd, p),
            move |t: &StoredText| Self::text_body(&self.sd, t),
        )
    }

    /// Household texts and built practices (Sprint 8). Reading is public
    /// (readers at a group practice needn't sign in); private practices
    /// are their builder's.
    fn route_practice_docs(
        &self,
        req: &Request<'_>,
        reply: &mut Reply<'_>,
    ) -> Result<bool, ApiError> {
        let path = req.path;
        let not_found = |what: &str| ApiError::not_found(format!("No such {what}"));
        match (req.method, path) {
            ("GET", "/api/v1/texts") => {
                let viewer = self.viewer(req);
                let texts = self.lock().state.texts_for(viewer);
                reply.wire(req, &TextList { texts });
            }
            ("POST", "/api/v1/texts") => {
                let c = self.caller(req)?;
                let body: TextInput = req.decode()?;
                let card = self.sd.present();
                let texts = self.mutate(|inner| {
                    let (_, write) = inner.state.add_text(c.actor(), &body, card, wall())?;
                    self.write_body(TEXTS_DIR, write)?;
                    Ok(inner.state.texts_for(c.viewer()))
                })?;
                reply.wire_status(req, 201, &TextList { texts });
            }
            (_, _) if path.starts_with("/api/v1/texts/") => {
                let id = Self::id_after(req, "/api/v1/texts")
                    .and_then(|id| crate::practice_docs::household_id(&id))
                    .ok_or_else(|| not_found("text"))?;
                match req.method {
                    "GET" => {
                        let viewer = self.viewer(req);
                        let view = self
                            .lock()
                            .state
                            .text_with_body(viewer, id, &|t| Self::text_body(&self.sd, t))?;
                        reply.wire(req, &view);
                    }
                    "PUT" => {
                        let c = self.caller(req)?;
                        let body: TextInput = req.decode()?;
                        let card = self.sd.present();
                        let view = self.mutate(|inner| {
                            let write = inner.state.update_text(c.actor(), id, &body, card)?;
                            self.write_body(TEXTS_DIR, write)?;
                            inner
                                .state
                                .text_with_body(c.viewer(), id, &|t| Self::text_body(&self.sd, t))
                        })?;
                        reply.wire(req, &view);
                    }
                    "DELETE" => {
                        let c = self.caller(req)?;
                        let (on_card, texts) = self.mutate(|inner| {
                            let on_card = inner.state.delete_text(c.actor(), id)?;
                            Ok((on_card, inner.state.texts_for(c.viewer())))
                        })?;
                        if on_card {
                            self.remove_bodies(TEXTS_DIR, &[id]);
                        }
                        reply.wire(req, &TextList { texts });
                    }
                    _ => return Err(not_found("text route")),
                }
            }
            ("GET", "/api/v1/practices") => {
                let viewer = self.viewer(req);
                let practices = self.lock().state.practices_for(viewer);
                reply.wire(req, &PracticeList { practices });
            }
            ("POST", "/api/v1/practices") => {
                let c = self.caller(req)?;
                let body: PracticeInput = req.decode()?;
                let view = self.mutate(|inner| {
                    let id = inner.state.add_practice(c.actor(), &body, wall())?;
                    inner.state.practice(c.viewer(), id)
                })?;
                reply.wire_status(req, 201, &view);
            }
            (_, _) if path.starts_with("/api/v1/practices/") => {
                let parts = req.rest("/api/v1/practices").unwrap_or_default();
                let id: u32 = parts
                    .first()
                    .and_then(|id| id.parse().ok())
                    .ok_or_else(|| not_found("practice"))?;
                match (req.method, &parts[1..]) {
                    ("GET", []) => {
                        let viewer = self.viewer(req);
                        let view = self.lock().state.practice(viewer, id)?;
                        reply.wire(req, &view);
                    }
                    ("PUT", []) => {
                        let c = self.caller(req)?;
                        let body: PracticeInput = req.decode()?;
                        let view = self.mutate(|inner| {
                            inner.state.replace_practice(c.actor(), id, &body)?;
                            inner.state.practice(c.viewer(), id)
                        })?;
                        reply.wire(req, &view);
                    }
                    ("DELETE", []) => {
                        let c = self.caller(req)?;
                        self.mutate(|inner| inner.state.delete_practice(c.actor(), id))?;
                        reply.wire(req, &Done { ok: true });
                    }
                    ("POST", ["done"]) => {
                        let c = self.caller(req)?;
                        let body: PracticeDone = if req.body_bytes()?.is_empty() {
                            PracticeDone::default()
                        } else {
                            req.decode()?
                        };
                        let today = self.mutate(|inner| {
                            inner
                                .state
                                .did_practice(c.actor(), id, &body.date, wall())?;
                            inner.state.today_view(c.viewer(), c.id, wall())
                        })?;
                        reply.wire_status(req, 201, &today);
                    }
                    _ => return Err(not_found("practice route")),
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn vow_detail(&self, state: &State, viewer: Viewer, id: u32) -> Result<VowDetail, ApiError> {
        state.vow_detail(viewer, id, &|p| Self::post_body(&self.sd, p))
    }

    fn vows_page(&self, state: &State, viewer: Viewer, person: u32) -> Result<VowsPage, ApiError> {
        state.vows_page(viewer, person, 30, &|p| Self::post_body(&self.sd, p))
    }

    /// Vows, their posts and precepts (Sprint 9). Reading is public, like
    /// profiles; changing is for the vow's person (or an admin).
    fn route_vows(&self, req: &Request<'_>, reply: &mut Reply<'_>) -> Result<bool, ApiError> {
        let path = req.path;
        let not_found = |what: &str| ApiError::not_found(format!("No such {what}"));
        match (req.method, path) {
            ("GET", "/api/v1/precepts") => {
                reply.wire(
                    req,
                    &PreceptCatalog {
                        sets: crate::precepts::sets().to_vec(),
                    },
                );
            }
            ("GET", _) if path.starts_with("/api/v1/people/") && path.ends_with("/vows") => {
                let viewer = self.viewer(req);
                let person: u32 = match req.rest("/api/v1/people").as_deref() {
                    Some([id, "vows"]) => id.parse().ok(),
                    _ => None,
                }
                .ok_or_else(|| not_found("person"))?;
                let page = self.vows_page(&self.lock().state, viewer, person)?;
                reply.wire(req, &page);
            }
            ("PUT" | "DELETE", _) if path.starts_with("/api/v1/me/precepts/") => {
                let c = self.caller(req)?;
                let set = Self::id_after(req, "/api/v1/me/precepts")
                    .ok_or_else(|| not_found("precept set"))?;
                let page = if req.method == "PUT" {
                    let body: PreceptsInput = req.decode()?;
                    self.mutate(|inner| {
                        inner.state.set_precepts(c.actor(), &set, &body, wall())?;
                        self.vows_page(&inner.state, c.viewer(), c.id)
                    })?
                } else {
                    self.mutate(|inner| {
                        inner.state.remove_precepts(c.actor(), &set)?;
                        self.vows_page(&inner.state, c.viewer(), c.id)
                    })?
                };
                reply.wire(req, &page);
            }
            ("GET", "/api/v1/vows") => {
                let c = self.caller(req)?;
                let page = self.vows_page(&self.lock().state, c.viewer(), c.id)?;
                reply.wire(req, &page);
            }
            ("POST", "/api/v1/vows") => {
                let c = self.caller(req)?;
                let body: VowInput = req.decode()?;
                let detail = self.mutate(|inner| {
                    let id = inner.state.add_vow(c.actor(), &body, wall())?;
                    self.vow_detail(&inner.state, c.viewer(), id)
                })?;
                reply.wire_status(req, 201, &detail);
            }
            (_, _) if path.starts_with("/api/v1/vows/") => {
                let parts = req.rest("/api/v1/vows").unwrap_or_default();
                let id: u32 = parts
                    .first()
                    .and_then(|id| id.parse().ok())
                    .ok_or_else(|| not_found("vow"))?;
                match (req.method, &parts[1..]) {
                    ("GET", []) => {
                        let viewer = self.viewer(req);
                        let detail = self.vow_detail(&self.lock().state, viewer, id)?;
                        reply.wire(req, &detail);
                    }
                    ("PATCH", []) => {
                        let c = self.caller(req)?;
                        let body: VowUpdate = req.decode()?;
                        let detail = self.mutate(|inner| {
                            inner.state.update_vow(c.actor(), id, &body)?;
                            self.vow_detail(&inner.state, c.viewer(), id)
                        })?;
                        reply.wire(req, &detail);
                    }
                    ("DELETE", []) => {
                        let c = self.caller(req)?;
                        let gone = self.mutate(|inner| inner.state.delete_vow(c.actor(), id))?;
                        self.remove_bodies(POSTS_DIR, &gone);
                        reply.wire(req, &Done { ok: true });
                    }
                    ("POST", ["posts"]) => {
                        let c = self.caller(req)?;
                        let body: PostInput = req.decode()?;
                        let card = self.sd.present();
                        let detail = self.mutate(|inner| {
                            let (_, write) =
                                inner.state.add_post(c.actor(), id, &body, card, wall())?;
                            self.write_body(POSTS_DIR, write)?;
                            self.vow_detail(&inner.state, c.viewer(), id)
                        })?;
                        reply.wire_status(req, 201, &detail);
                    }
                    ("POST", [action]) => {
                        let c = self.caller(req)?;
                        let action =
                            vows::Action::parse(action).ok_or_else(|| not_found("action"))?;
                        let body: VowActionInput = if req.body_bytes()?.is_empty() {
                            VowActionInput::default()
                        } else {
                            req.decode()?
                        };
                        let detail = self.mutate(|inner| {
                            inner
                                .state
                                .vow_action(c.actor(), id, action, &body, wall())?;
                            self.vow_detail(&inner.state, c.viewer(), id)
                        })?;
                        reply.wire(req, &detail);
                    }
                    _ => return Err(not_found("vow route")),
                }
            }
            (_, _) if path.starts_with("/api/v1/posts/") => {
                let id: u32 = Self::id_after(req, "/api/v1/posts")
                    .and_then(|id| id.parse().ok())
                    .ok_or_else(|| not_found("post"))?;
                match req.method {
                    "GET" => {
                        let viewer = self.viewer(req);
                        let inner = self.lock();
                        let view = inner
                            .state
                            .posts
                            .iter()
                            .find(|p| p.id == id)
                            .and_then(|p| {
                                inner
                                    .state
                                    .post_view(viewer, p, &|p| Self::post_body(&self.sd, p))
                            })
                            .ok_or_else(|| not_found("post"))?;
                        reply.wire(req, &view);
                    }
                    "PUT" => {
                        let c = self.caller(req)?;
                        let body: PostInput = req.decode()?;
                        let card = self.sd.present();
                        let detail = self.mutate(|inner| {
                            let write =
                                inner
                                    .state
                                    .update_post(c.actor(), id, &body, card, wall())?;
                            self.write_body(POSTS_DIR, write)?;
                            let vow = inner
                                .state
                                .posts
                                .iter()
                                .find(|p| p.id == id)
                                .map_or(0, |p| p.vow);
                            self.vow_detail(&inner.state, c.viewer(), vow)
                        })?;
                        reply.wire(req, &detail);
                    }
                    "DELETE" => {
                        let c = self.caller(req)?;
                        let (on_card, detail) = self.mutate(|inner| {
                            let vow = inner
                                .state
                                .posts
                                .iter()
                                .find(|p| p.id == id)
                                .map_or(0, |p| p.vow);
                            let on_card = inner.state.delete_post(c.actor(), id)?;
                            Ok((on_card, self.vow_detail(&inner.state, c.viewer(), vow)?))
                        })?;
                        if on_card {
                            self.remove_bodies(POSTS_DIR, &[id]);
                        }
                        reply.wire(req, &detail);
                    }
                    _ => return Err(not_found("post route")),
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn route(&self, req: &Request<'_>, reply: &mut Reply<'_>) -> Result<(), ApiError> {
        let path = req.path;
        if req.method == "GET" && path.starts_with("/cal/") && path.ends_with(".ics") {
            return self.calendar_feed(req, reply);
        }
        if !path.starts_with("/api/") {
            return Ok(());
        }
        require_same_origin(req, &self.dev_origins)?;
        let now = wall().unwrap_or(0);
        if self.route_practice(req, reply)?
            || self.route_transfer(req, reply)?
            || self.route_vows(req, reply)?
            || self.route_practice_docs(req, reply)?
        {
            return Ok(());
        }
        match (req.method, path) {
            ("GET", "/api/v1/status") => reply.wire(req, &self.status(req)),

            // ---- setup and sign-in -------------------------------------
            ("POST", "/api/v1/setup") => {
                let body: SetupInput = req.decode()?;
                if let Some(code) = self.setup_code {
                    if body.setup_code.trim() != code {
                        return Err(ApiError::new(403, "setup_code", "Wrong setup code"));
                    }
                }
                let me = self.mutate(|inner| {
                    let id = inner.state.setup(&body.display_name, &body.pin, now)?;
                    Self::sign_in(inner, id, req, reply)?;
                    Self::me(&inner.state, id)
                })?;
                reply.wire_status(req, 201, &me);
            }
            ("POST", "/api/v1/auth/signup") => {
                let body: SignUpInput = req.decode()?;
                let me = self.mutate(|inner| {
                    let id = inner.state.signup(&body.display_name, &body.pin, now)?;
                    Self::sign_in(inner, id, req, reply)?;
                    Self::me(&inner.state, id)
                })?;
                reply.wire_status(req, 201, &me);
            }
            ("POST", "/api/v1/auth/pin") => {
                let body: PinSignIn = req.decode()?;
                let me = self.pin_sign_in(&body, req, reply)?;
                reply.wire(req, &me);
            }
            ("POST", "/api/v1/auth/logout") => {
                let all = req.param("all").is_some_and(|v| v != "0");
                let caller = self.caller(req).ok();
                self.mutate(|inner| {
                    match (all, caller) {
                        (true, Some(c)) => {
                            inner.state.sessions.revoke_subject(c.id);
                        }
                        _ => {
                            inner.state.sessions.revoke(req);
                        }
                    }
                    Ok(())
                })?;
                reply.header(
                    "Set-Cookie",
                    self.lock().state.sessions.clear_cookie(req.secure),
                );
                reply.wire(req, &Done { ok: true });
            }

            // ---- people ------------------------------------------------
            ("GET", "/api/v1/people") => reply.wire(req, &Self::people(&self.lock().state)),
            ("GET", "/api/v1/me") => {
                let c = self.caller(req)?;
                reply.wire(req, &Self::me(&self.lock().state, c.id)?);
            }
            ("PATCH", "/api/v1/me") => {
                let c = self.caller(req)?;
                let body: MeUpdate = req.decode()?;
                let me = self.mutate(|inner| {
                    inner.state.update_me(
                        c.id,
                        body.display_name.as_deref(),
                        body.email.as_deref(),
                    )?;
                    if let Some(calendar) = &body.calendar {
                        inner.state.set_my_calendar(c.id, calendar)?;
                    }
                    Self::me(&inner.state, c.id)
                })?;
                reply.wire(req, &me);
            }
            ("POST", "/api/v1/me/pin") => {
                let c = self.caller(req)?;
                let body: PinChange = req.decode()?;
                self.mutate(|inner| inner.state.change_pin(c.id, &body.current, &body.new_pin))?;
                reply.wire(req, &Done { ok: true });
            }

            // ---- admin -------------------------------------------------
            ("GET", "/api/v1/admin/people") => {
                self.admin(req)?;
                reply.wire(req, &Self::admin_people(&self.lock().state));
            }
            ("POST", "/api/v1/admin/people") => {
                self.admin(req)?;
                let body: NewPerson = req.decode()?;
                let list = self.mutate(|inner| {
                    inner.state.add_person(
                        &body.display_name,
                        &body.role,
                        &body.pin,
                        &body.email,
                        now,
                    )?;
                    Ok(Self::admin_people(&inner.state))
                })?;
                reply.wire_status(req, 201, &list);
            }
            ("GET", "/api/v1/admin/settings") => {
                self.admin(req)?;
                reply.wire(req, &Self::settings(&self.lock().state));
            }
            ("PATCH", "/api/v1/admin/settings") => {
                self.admin(req)?;
                let body: SettingsUpdate = req.decode()?;
                let settings = self.mutate(|inner| {
                    if let Some(open) = body.signup_open {
                        inner.state.signup_closed = !open;
                    }
                    if let Some(calendar) = &body.calendar {
                        let system = crate::calendar::System::parse(calendar).ok_or_else(|| {
                            ApiError::bad_request("calendar: chinese, theravada, japanese or none")
                        })?;
                        inner.state.calendar = system.key().into();
                    }
                    if let Some(on) = body.sunday_shift {
                        inner.state.sunday_shift = on;
                    }
                    if let Some(on) = body.quarter_moons {
                        inner.state.quarter_moons = on;
                    }
                    if let Some(on) = body.ten_fasting_days {
                        inner.state.ten_fasting_days = on;
                    }
                    if let Some(zone) = &body.timezone {
                        // A place from the picker or the phone, else a rule.
                        let zone = match crate::zones::by_name(zone.trim()) {
                            Some((_, _, posix)) => posix,
                            None => zone.trim(),
                        };
                        inner.state.tz = Tz::parse(zone).map_err(|e| {
                            ApiError::bad_request(format!(
                                "Unknown time zone: pick a place from the list ({e})"
                            ))
                        })?;
                        inner.state.timezone = zone.to_string();
                    }
                    Ok(Self::settings(&inner.state))
                })?;
                reply.wire(req, &settings);
            }
            ("GET", "/api/v1/admin/storage") => {
                self.admin(req)?;
                reply.wire(req, &self.storage());
            }
            (method, _) if path.starts_with("/api/v1/admin/people/") => {
                let c = self.admin(req)?;
                let id: u32 = match req.rest("/api/v1/admin/people").as_deref() {
                    Some([id]) => id.parse().ok(),
                    _ => None,
                }
                .ok_or_else(|| ApiError::not_found("No such person"))?;
                match method {
                    "PATCH" => {
                        let body: AdminPersonUpdate = req.decode()?;
                        let list = self.mutate(|inner| {
                            inner.state.admin_update(c.id, id, &body)?;
                            Ok(Self::admin_people(&inner.state))
                        })?;
                        if body.pin.is_some() {
                            self.lock().guard.clear(id);
                        }
                        reply.wire(req, &list);
                    }
                    "DELETE" => {
                        let keep = match req.param("entries").as_deref() {
                            Some("keep") => true,
                            Some("delete") => false,
                            _ => {
                                return Err(ApiError::bad_request(
                                    "Say what happens to their entries: ?entries=keep or ?entries=delete",
                                ))
                            }
                        };
                        let list = self.mutate(|inner| {
                            inner.state.remove_person(c.id, id, keep)?;
                            if !keep && inner.state.journaled {
                                if let Some(j) = &inner.journal {
                                    let mut old = Vec::new();
                                    j.for_each(|e| {
                                        if e.person == id {
                                            old.push(Journal::tombstone(e.id));
                                        }
                                    })
                                    .map_err(|e| {
                                        log::error!("reading the journal: {e}");
                                        ApiError::new(
                                            500,
                                            "storage_failed",
                                            "Could not read the SD card",
                                        )
                                    })?;
                                    inner.pending.extend(old);
                                }
                            }
                            Ok(Self::admin_people(&inner.state))
                        })?;
                        reply.wire(req, &list);
                    }
                    _ => return Err(ApiError::new(405, "method", "Method not allowed")),
                }
            }

            // ---- Sprint 1 hardware gate (never in a release) ------------
            #[cfg(feature = "gate")]
            ("GET", "/api/v1/gate") => reply.wire(req, &self.gate.stats()),
            #[cfg(feature = "gate")]
            ("POST", "/api/v1/gate") => {
                let secs = req.param_num::<u64>("secs")?.unwrap_or(300).min(3600);
                self.gate.start(secs);
                reply.wire_status(req, 202, &self.gate.stats());
            }
            #[cfg(feature = "gate")]
            ("GET", "/api/v1/gate/storage") => reply.wire(req, &self.storage()),
            #[cfg(feature = "gate")]
            ("POST", "/api/v1/gate/journal") => {
                // Gate builds only: a scratch journal under ml/gate/bench,
                // never the household's. Blocks the serving task meanwhile.
                let n = req
                    .param_num::<u32>("entries")?
                    .unwrap_or(7_300)
                    .min(100_000);
                let scratch = self
                    .sd
                    .sub("ml/gate/bench")
                    .map_err(|e| ApiError::new(503, "sd", e.to_string()))?;
                let bench = crate::gate::journal_bench(Arc::new(scratch), n)
                    .map_err(|e| ApiError::new(503, "sd", e.to_string()))?;
                reply.wire(req, &bench);
            }
            #[cfg(feature = "gate")]
            ("POST", "/api/v1/gate/sd") => {
                // Blocks the serving task for the length of the test (about
                // a second for 1 MiB).
                let bytes = req
                    .param_num::<usize>("bytes")?
                    .unwrap_or(1 << 20)
                    .min(8 << 20);
                let t = self
                    .sd
                    .self_test("ml/gate/selftest.bin", bytes)
                    .map_err(|e| ApiError::new(503, "sd", e.to_string()))?;
                reply.wire(
                    req,
                    &SdTestResult {
                        bytes: t.bytes,
                        write_ms: t.write_ms,
                        read_ms: t.read_ms,
                    },
                );
            }
            // Untouched reply: the site sends a 404.
            _ => {}
        }
        Ok(())
    }
}

impl Service for App {
    /// An import can be several megabytes: read as it arrives.
    fn streamed_body(&self, method: &str, path: &str) -> Option<usize> {
        (method == "POST" && path == "/api/v1/admin/import").then_some(16 * 1024 * 1024)
    }

    fn checks(&self) -> Vec<Check> {
        let s = self.storage();
        let mut checks = vec![Check::usage(
            "store.people",
            "People",
            s.people as f64,
            s.max_people as f64,
            80.0,
            95.0,
        )];
        checks.push(if s.fallback_used {
            Check::new(
                "store.slot",
                CheckStatus::Warn,
                format!(
                    "Booted from slot {}: the other snapshot was invalid",
                    s.loaded_slot
                ),
            )
        } else {
            Check::new("store.slot", CheckStatus::Ok, "Newest snapshot loaded")
        });
        // The SD card is optional; without one there is nothing to judge.
        if s.sd_present {
            checks.push(Check::usage(
                "sd.space",
                "SD card",
                s.sd_total_bytes.saturating_sub(s.sd_free_bytes) as f64,
                s.sd_total_bytes as f64,
                80.0,
                95.0,
            ));
        }
        if s.journaled {
            let damaged = s.damaged_files.len();
            checks.push(if damaged > 0 || s.torn_bytes > 0 {
                Check::new(
                    "sd.journal",
                    CheckStatus::Warn,
                    format!(
                        "{damaged} damaged journal files, {} torn bytes",
                        s.torn_bytes
                    ),
                )
            } else {
                Check::new(
                    "sd.journal",
                    CheckStatus::Ok,
                    format!("{} journal records intact", s.journal_records),
                )
            });
        }
        checks.into_iter().map(|c| c.runbook("storage")).collect()
    }

    fn handle(&self, req: &Request<'_>, reply: &mut Reply<'_>) {
        if let Err(e) = self.route(req, reply) {
            reply.fail(req, &e);
        }
    }

    fn schemas(&self) -> Vec<&'static Schema> {
        #[allow(unused_mut)]
        let mut schemas = vec![
            Status::SCHEMA,
            StorageInfo::SCHEMA,
            Done::SCHEMA,
            PeopleList::SCHEMA,
            Me::SCHEMA,
            SetupInput::SCHEMA,
            SignUpInput::SCHEMA,
            PinSignIn::SCHEMA,
            MeUpdate::SCHEMA,
            PinChange::SCHEMA,
            AdminPeopleList::SCHEMA,
            NewPerson::SCHEMA,
            AdminPersonUpdate::SCHEMA,
            Settings::SCHEMA,
            SettingsUpdate::SCHEMA,
            KindList::SCHEMA,
            KindInput::SCHEMA,
            KindUpdate::SCHEMA,
            PinsInput::SCHEMA,
            TodayView::SCHEMA,
            EntryInput::SCHEMA,
            EntryUpdate::SCHEMA,
            EntryList::SCHEMA,
            EntryView::SCHEMA,
            ProfileView::SCHEMA,
            FeedView::SCHEMA,
            ExportDoc::SCHEMA,
            ImportReport::SCHEMA,
            BackupList::SCHEMA,
            TimerList::SCHEMA,
            TimerInput::SCHEMA,
            CalendarView::SCHEMA,
            ZoneList::SCHEMA,
            PreceptCatalog::SCHEMA,
            PreceptsInput::SCHEMA,
            VowsPage::SCHEMA,
            VowDetail::SCHEMA,
            VowInput::SCHEMA,
            VowUpdate::SCHEMA,
            VowActionInput::SCHEMA,
            PostInput::SCHEMA,
            PostView::SCHEMA,
            TextList::SCHEMA,
            TextView::SCHEMA,
            TextInput::SCHEMA,
            PracticeList::SCHEMA,
            PracticeView::SCHEMA,
            PracticeInput::SCHEMA,
            PracticeDone::SCHEMA,
        ];
        #[cfg(feature = "gate")]
        schemas.extend([crate::gate::GateStats::SCHEMA, SdTestResult::SCHEMA]);
        schemas
    }
}
