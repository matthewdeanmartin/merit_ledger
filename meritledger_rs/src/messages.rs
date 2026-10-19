//! Every message Merit Ledger sends, accepts or stores. Tags are protobuf
//! field numbers and CBOR integer keys: add fields with new numbers, never
//! renumber. Stored messages (`Stored*`) are the snapshot's schema.
use miniframework::message;

// ---- API -----------------------------------------------------------------

message! {
    /// `GET /api/v1/status`: public, drives the first page the UI shows.
    pub struct Status {
        1 app: String,
        2 version: String,
        3 build: String,
        /// The board's clock has been set by SNTP since boot. Calendars,
        /// "done today" and reminders need it.
        4 clock_synced: bool,
        /// Arrived over TLS. The meditation timer needs a secure context.
        5 secure: bool,
        /// Nobody exists yet: the next visitor sets the board up as admin.
        6 setup_needed: bool,
        /// Anyone on the Wi-Fi may make a (guest) account.
        7 signup_open: bool,
        /// A setup code is compiled in and `/api/v1/setup` needs it.
        8 setup_code_required: bool,
    }
}

message! {
    /// `GET /api/v1/admin/storage`: what the board is keeping, and where.
    pub struct StorageInfo {
        1 sd_present: bool,
        2 sd_total_bytes: u64,
        3 sd_free_bytes: u64,
        4 generation: u64,
        5 snapshot_bytes: u32,
        6 commits: u64,
        7 people: u32,
        8 max_people: u32,
        9 sessions: u32,
        10 max_sessions: u32,
        /// The slot boot loaded from, and whether the other one was invalid.
        11 loaded_slot: u32,
        12 fallback_used: bool,
        /// Entries are in the SD journal (otherwise in the snapshot).
        13 journaled: bool,
        14 journal_files: u32,
        15 journal_bytes: u64,
        16 journal_records: u64,
        /// Cut off the newest journal file at boot (power lost mid-write).
        17 torn_bytes: u64,
        /// Older journal files with a bad record (a card fault).
        18 damaged_files: Vec<String>,
        /// Entries in RAM (the last 60 days), and days kept only as totals.
        19 window_entries: u32,
        20 history_days: u32,
        /// Newest backup file name, "" if none.
        21 last_backup: String,
        22 backups: u32,
        /// The card was used before but is missing now.
        23 card_missing: bool,
    }
}

message! {
    /// `POST /api/v1/gate/sd` (gate builds): one write-sync-read round trip.
    pub struct SdTestResult {
        1 bytes: u64,
        2 write_ms: u64,
        3 read_ms: u64,
    }
}

message! {
    /// A body-less success.
    pub struct Done {
        1 ok: bool,
    }
}

// ---- People --------------------------------------------------------------

message! {
    /// Someone in the household, as anyone on the Wi-Fi sees them: the
    /// sign-in picker and the board's "record for whom" list.
    pub struct PersonView {
        1 id: u32,
        2 display_name: String,
        /// `admin`, `member` or `guest`.
        3 role: String,
        /// Has a PIN, so can sign in. Without one, others record for them.
        4 can_sign_in: bool,
    }
}

message! {
    /// `GET /api/v1/people`.
    pub struct PeopleList {
        1 people: Vec<PersonView>,
    }
}

message! {
    /// `GET /api/v1/me`: the signed-in person.
    pub struct Me {
        1 id: u32,
        2 display_name: String,
        3 role: String,
        4 has_pin: bool,
        /// For invitations and notifications later; may be empty.
        5 email: String,
        /// Signed-in browsers.
        6 sessions: u32,
        7 created: u64,
        /// Their own calendar system, "" = the household's.
        8 calendar: String,
    }
}

message! {
    /// `POST /api/v1/setup`: the first person, who becomes admin.
    pub struct SetupInput {
        1 display_name: String,
        2 pin: String,
        3 setup_code: String,
    }
}

message! {
    /// `POST /api/v1/auth/signup`: a guest makes their own account.
    pub struct SignUpInput {
        1 display_name: String,
        2 pin: String,
    }
}

message! {
    /// `POST /api/v1/auth/pin`: tap your name, enter your PIN.
    pub struct PinSignIn {
        1 person: u32,
        2 pin: String,
    }
}

message! {
    /// `PATCH /api/v1/me`.
    pub struct MeUpdate {
        1 display_name: Option<String>,
        2 email: Option<String>,
        /// "" = follow the household.
        3 calendar: Option<String>,
    }
}

message! {
    /// `POST /api/v1/me/pin`. `current` may be empty when there is no PIN.
    pub struct PinChange {
        1 current: String,
        2 new_pin: String,
    }
}

message! {
    /// One person, as the admin page sees them.
    pub struct AdminPersonView {
        1 id: u32,
        2 display_name: String,
        3 role: String,
        4 has_pin: bool,
        5 disabled: bool,
        6 email: String,
        7 created: u64,
        8 last_seen: u64,
        9 sessions: u32,
    }
}

message! {
    /// `GET /api/v1/admin/people`.
    pub struct AdminPeopleList {
        1 people: Vec<AdminPersonView>,
    }
}

message! {
    /// `POST /api/v1/admin/people`: a parent adds a kid, an admin adds a
    /// housemate. An empty `pin` makes someone others record for.
    pub struct NewPerson {
        1 display_name: String,
        2 role: String,
        3 pin: String,
        4 email: String,
    }
}

message! {
    /// `PATCH /api/v1/admin/people/{id}`. `pin: ""` removes the PIN.
    pub struct AdminPersonUpdate {
        1 display_name: Option<String>,
        2 role: Option<String>,
        3 pin: Option<String>,
        4 disabled: Option<bool>,
    }
}

message! {
    /// `GET`/`PATCH /api/v1/admin/settings`.
    pub struct Settings {
        1 signup_open: bool,
        /// The household's POSIX TZ (`EST5EDT,M3.2.0,M11.1.0`): decides what
        /// "today" is.
        2 timezone: String,
        /// The household's calendar: `chinese`, `theravada`, `japanese`
        /// or `none` (Sprint 7). People may choose their own.
        3 calendar: String,
        /// Move observances to the closest Sunday.
        4 sunday_shift: bool,
        /// Theravada: uposatha on quarter moons too.
        5 quarter_moons: bool,
        /// Chinese: ten fasting days a month instead of six.
        6 ten_fasting_days: bool,
        /// The zone as a person reads it ("New York (Eastern)"), or ""
        /// for a rule typed in by hand.
        7 timezone_label: String,
    }
}

message! {
    pub struct SettingsUpdate {
        1 signup_open: Option<bool>,
        /// A place's IANA name ("America/New_York", from the picker or the
        /// phone), or a POSIX rule ("Advanced").
        2 timezone: Option<String>,
        3 calendar: Option<String>,
        4 sunday_shift: Option<bool>,
        5 quarter_moons: Option<bool>,
        6 ten_fasting_days: Option<bool>,
    }
}

// ---- Stored (the snapshot) -----------------------------------------------

message! {
    pub struct StoredPerson {
        1 id: u32,
        2 display_name: String,
        /// `admin`, `member` or `guest`.
        3 role: String,
        /// PBKDF2 hash (`pin.rs`); empty = cannot sign in.
        4 pin: String,
        5 email: String,
        6 created: u64,
        7 last_seen: u64,
        8 disabled: bool,
        /// Practice kinds on their Today page, in order; empty = defaults.
        9 pinned: Vec<String>,
        /// Their own calendar system; empty = the household's.
        10 calendar: String,
    }
}

message! {
    pub struct StoredSession {
        /// SHA-256 of the cookie token, hex.
        1 hash: String,
        2 person: u32,
        3 created: u64,
        4 last_seen: u64,
    }
}

message! {
    /// The whole snapshot (`store.rs` writes it as CBOR with integer keys).
    pub struct StoredState {
        1 schema: u32,
        2 next_person: u32,
        3 people: Vec<StoredPerson>,
        4 sessions: Vec<StoredSession>,
        /// Default (false) = sign-up open.
        5 signup_closed: bool,
        6 commits: u64,
        /// Custom practice kinds (built-ins ship in the firmware).
        7 kinds: Vec<StoredKind>,
        8 next_kind: u32,
        /// Entries in the snapshot window (Sprint 5 archives older ones).
        9 entries: Vec<StoredEntry>,
        10 next_entry: u32,
        /// POSIX TZ; empty = the firmware default.
        11 timezone: String,
        /// Per-day totals of entries that left the window while there was
        /// no SD card: their detail is gone, their stats are not.
        12 archived_days: Vec<StoredDay>,
        /// Entries have lived on an SD card: if it is missing now, say so.
        13 used_card: bool,
        /// With a journal: ids below this may be in use on the card, so a
        /// boot without the card starts after it.
        14 entry_lease: u32,
        /// Custom timer presets (Sprint 6); built-ins ship in the firmware.
        15 timers: Vec<StoredTimer>,
        16 next_timer: u32,
        /// Calendar settings (Sprint 7); empty calendar = the default.
        17 calendar: String,
        18 sunday_shift: bool,
        19 quarter_moons: bool,
        20 ten_fasting_days: bool,
        /// Vows (Sprint 9, `vows.rs`).
        21 vows: Vec<StoredVow>,
        22 next_vow: u32,
        /// Vow posts: titles here, bodies on the SD card (or in `body`
        /// without one).
        23 posts: Vec<StoredPost>,
        24 next_post: u32,
        /// Precepts people have taken (Sprint 9).
        25 precepts: Vec<StoredPrecepts>,
        /// Household texts (Sprint 8): titles here, bodies on the SD card.
        26 texts: Vec<StoredText>,
        27 next_text: u32,
        /// Built practices (Sprint 8).
        28 practices: Vec<StoredPracticeDoc>,
        29 next_practice: u32,
    }
}

message! {
    /// One person's practice on one day, as totals (`stats::Tally`).
    pub struct StoredDay {
        1 person: u32,
        /// Days since 1970-01-01.
        2 day: i32,
        3 own_entries: u32,
        4 own_minutes: u32,
        5 open_entries: u32,
        6 open_minutes: u32,
        7 day_only: u32,
        8 demerits: u32,
    }
}

// ---- Practice (Sprint 3) ---------------------------------------------------

message! {
    /// A practice kind. Built-ins come from `catalog/practices.json`; custom
    /// ones live in the snapshot with keys `c<n>`.
    pub struct StoredKind {
        1 key: String,
        2 name: String,
        3 icon: String,
        /// `merit` or `demerit`.
        4 polarity: String,
        /// meditation, chanting, prostration, recitation, reading, copying,
        /// art, precepts, service, dedication, rejoicing, repentance, ...
        5 category: String,
        /// `minutes`, `count` or `once`.
        6 unit: String,
        /// What one tap records (minutes or count; 0 for `once`).
        7 default_amount: u32,
        /// `public`, `generic` or `private`: new entries start here.
        8 visibility: String,
        /// Entries can be published (a sit is content; a mood is noise).
        9 content: bool,
        /// 0 = the household's; otherwise one person's own kind.
        10 owner: u32,
        11 archived: bool,
    }
}

message! {
    /// `catalog/practices.json`.
    pub struct KindCatalog {
        1 kinds: Vec<StoredKind>,
    }
}

message! {
    pub struct KindView {
        1 key: String,
        2 name: String,
        3 icon: String,
        4 polarity: String,
        5 category: String,
        6 unit: String,
        7 default_amount: u32,
        8 visibility: String,
        9 content: bool,
        10 owner: u32,
        11 archived: bool,
        12 builtin: bool,
        /// On the viewer's Today page.
        13 pinned: bool,
    }
}

message! {
    /// `GET /api/v1/kinds`.
    pub struct KindList {
        1 kinds: Vec<KindView>,
    }
}

message! {
    /// `POST /api/v1/kinds`.
    pub struct KindInput {
        1 name: String,
        2 icon: String,
        3 polarity: String,
        4 category: String,
        5 unit: String,
        6 default_amount: u32,
        7 visibility: String,
        /// Everyone's (true) or just mine (false).
        8 household: bool,
    }
}

message! {
    /// `PATCH /api/v1/kinds/{key}` (custom kinds only).
    pub struct KindUpdate {
        1 name: Option<String>,
        2 icon: Option<String>,
        3 default_amount: Option<u32>,
        4 visibility: Option<String>,
        5 archived: Option<bool>,
    }
}

message! {
    /// `PUT /api/v1/me/pins`: the Today page's kinds, in order.
    pub struct PinsInput {
        1 kinds: Vec<String>,
    }
}

message! {
    pub struct StoredEntry {
        1 id: u32,
        2 person: u32,
        3 kind: String,
        /// Household-local `YYYY-MM-DD`.
        4 date: String,
        /// UTC seconds when it was recorded.
        5 created: u64,
        /// Minutes or count; 0 for `once` kinds.
        6 amount: u32,
        7 note: String,
        8 visibility: String,
        /// `tap`, `timer`, `board`, `manual` or `import`.
        9 source: String,
        /// Who recorded it; 0 = the board's touch menu.
        10 recorded_by: u32,
        11 rev: u32,
        /// People who rejoiced in it (Sprint 4), each once.
        12 rejoiced_by: Vec<u32>,
        /// Journal tombstone (Sprint 5): this id was deleted. Only `id` is
        /// meaningful; never set in the snapshot.
        13 deleted: bool,
        /// `EntryInput::client_id`, kept so a retried POST is recognized.
        14 client_id: String,
    }
}

message! {
    /// An entry as one viewer may see it. Someone else's `generic` entry
    /// shows only its category (`generic` is true, note and amount empty).
    pub struct EntryView {
        1 id: u32,
        2 person: u32,
        3 person_name: String,
        4 kind: String,
        5 kind_name: String,
        6 icon: String,
        7 category: String,
        8 polarity: String,
        9 unit: String,
        10 date: String,
        11 created: u64,
        12 amount: u32,
        13 note: String,
        14 visibility: String,
        15 source: String,
        16 recorded_by_name: String,
        17 rev: u32,
        18 can_edit: bool,
        19 generic: bool,
        /// How many rejoiced in it, and whether the viewer did.
        20 rejoices: u32,
        21 rejoiced_by_me: bool,
        /// The viewer may rejoice: someone else's, shown in full, merit.
        22 can_rejoice: bool,
    }
}

message! {
    pub struct EntryList {
        1 entries: Vec<EntryView>,
    }
}

message! {
    /// `POST /api/v1/entries`. `person: 0` = me. Unset fields take the
    /// kind's defaults and today's date.
    pub struct EntryInput {
        1 person: u32,
        2 kind: String,
        3 amount: Option<u32>,
        4 note: String,
        5 visibility: Option<String>,
        6 date: Option<String>,
        7 source: Option<String>,
        /// Made by the client once per finished session (the timer's offline
        /// queue): a retry with the same id records nothing new.
        8 client_id: String,
    }
}

message! {
    /// `PATCH /api/v1/entries/{id}`: `rev` must match (else 409 `stale`).
    pub struct EntryUpdate {
        1 rev: u32,
        2 amount: Option<u32>,
        3 note: Option<String>,
        4 visibility: Option<String>,
        5 date: Option<String>,
    }
}

message! {
    /// One button on the Today page.
    pub struct Tile {
        1 key: String,
        2 name: String,
        3 icon: String,
        4 unit: String,
        5 default_amount: u32,
        6 polarity: String,
        /// Entries of this kind today.
        7 count: u32,
        /// Minutes or count recorded today.
        8 total: u32,
    }
}

message! {
    /// `GET /api/v1/today?person=`.
    pub struct TodayView {
        1 date: String,
        2 person: u32,
        3 person_name: String,
        4 tiles: Vec<Tile>,
        5 entries: Vec<EntryView>,
        /// Today's observances on this person's calendar (Sprint 7).
        6 observances: Vec<ObservanceView>,
    }
}

// ---- Profiles, feed and stats (Sprint 4) -----------------------------------

message! {
    /// Minutes and entries of one category over a period.
    pub struct CategoryTotal {
        1 category: String,
        2 count: u32,
        3 minutes: u32,
    }
}

message! {
    /// `GET /api/v1/people/{id}/profile`: public, like everything that
    /// isn't private.
    pub struct ProfileView {
        1 id: u32,
        2 display_name: String,
        3 role: String,
        /// Today in the household zone.
        4 date: String,
        /// Days in a row with any merit, ending today (or yesterday, until
        /// today ends).
        5 current_streak: u32,
        6 longest_streak: u32,
        /// Merit minutes in the last 7 days, today included.
        7 week_minutes: u32,
        8 week: Vec<CategoryTotal>,
        /// Last 30 days.
        9 month: Vec<CategoryTotal>,
        /// One level (0-4) per day, oldest first, ending today.
        10 heatmap_start: String,
        11 heatmap: Vec<u32>,
        12 recent: Vec<EntryView>,
        /// The oldest day the board still has entries for: before it the
        /// heatmap is unknown, not empty.
        13 history_from: String,
        /// Demerits in the last 30 days: only for the person and admins.
        14 demerits_30: u32,
        15 own: bool,
    }
}

message! {
    /// `GET /api/v1/feed`: the household's recent entries anyone may see.
    pub struct FeedView {
        1 entries: Vec<EntryView>,
        2 more: bool,
    }
}

// ---- History, backups and export (Sprint 5) --------------------------------

message! {
    /// Someone in an export: no PIN, no email.
    pub struct ExportPerson {
        1 id: u32,
        2 display_name: String,
        3 role: String,
    }
}

message! {
    /// The export and backup format (`docs/EXPORT_FORMAT.md`), as JSON.
    pub struct ExportDoc {
        /// Always "meritledger-export".
        1 format: String,
        2 version: u32,
        /// UTC seconds.
        3 exported_at: u64,
        4 timezone: String,
        5 people: Vec<ExportPerson>,
        /// Custom kinds only; built-in keys are stable across boards.
        6 kinds: Vec<StoredKind>,
        7 entries: Vec<StoredEntry>,
        /// Day totals whose entries were not kept (no SD card at the time).
        8 archived_days: Vec<StoredDay>,
        9 vows: Vec<StoredVow>,
        /// With their bodies.
        10 posts: Vec<StoredPost>,
        11 precepts: Vec<StoredPrecepts>,
        /// Household texts, with their bodies.
        12 texts: Vec<StoredText>,
        13 practices: Vec<StoredPracticeDoc>,
    }
}

message! {
    /// `POST /api/v1/admin/import[?dry_run=1]`.
    pub struct ImportReport {
        1 dry_run: bool,
        2 people_matched: u32,
        3 people_created: u32,
        4 kinds_created: u32,
        5 entries_added: u32,
        /// Already on the board (same person, practice, day, time, amount, note).
        6 entries_skipped: u32,
        /// Older than the window with no SD card: kept as day totals only.
        7 entries_summarized: u32,
        8 notes: Vec<String>,
        9 vows_added: u32,
        10 posts_added: u32,
        11 precepts_added: u32,
        12 texts_added: u32,
        13 practices_added: u32,
    }
}

message! {
    /// `GET /api/v1/admin/backups`.
    pub struct BackupList {
        1 names: Vec<String>,
    }
}

// ---- Meditation timer (Sprint 6) -------------------------------------------

message! {
    /// One part of a timed session.
    pub struct StoredSegment {
        /// `sit` or `walk`.
        1 kind: String,
        2 minutes: u32,
    }
}

message! {
    /// A timer preset: timed segments with bells, or a count (mala).
    /// Built-ins come from `catalog/timers.json`; custom ones (`t<n>`)
    /// live in the snapshot.
    pub struct StoredTimer {
        1 key: String,
        2 name: String,
        /// Empty for a count preset.
        3 segments: Vec<StoredSegment>,
        /// Strikes at the start, between segments are always one.
        4 open_strikes: u32,
        /// A single strike every this many minutes inside a sitting
        /// segment; 0 for none.
        5 interval_minutes: u32,
        6 close_strikes: u32,
        /// A count preset's target (108); 0 for a timed preset.
        7 count_target: u32,
        /// The practice kind a sitting segment or count logs as; walking
        /// segments log as `walking`.
        8 logs_as: String,
        /// `bowl`, `bell` or `block`.
        9 timbre: String,
        /// 0 = the household's; otherwise one person's own.
        10 owner: u32,
        11 archived: bool,
        /// Shipped with the firmware (views only; never stored).
        12 builtin: bool,
    }
}

message! {
    /// `GET /api/v1/timers`.
    pub struct TimerList {
        1 timers: Vec<StoredTimer>,
    }
}

message! {
    /// `POST /api/v1/timers` and `PUT /api/v1/timers/{key}`.
    pub struct TimerInput {
        1 name: String,
        2 segments: Vec<StoredSegment>,
        3 open_strikes: u32,
        4 interval_minutes: u32,
        5 close_strikes: u32,
        6 count_target: u32,
        7 logs_as: String,
        8 timbre: String,
        /// Everyone's (true) or just mine (false). Ignored by PUT.
        9 household: bool,
        /// PUT only.
        10 archived: bool,
    }
}

// ---- Calendars (Sprint 7) --------------------------------------------------

message! {
    pub struct ObservanceView {
        1 date: String,
        /// `uposatha`, `fasting`, `festival` or `moon`.
        2 kind: String,
        3 name: String,
        4 feast: bool,
        /// The true date when Sunday shift moved it, else "".
        5 shifted_from: String,
    }
}

message! {
    /// One day of the calendar page.
    pub struct CalendarDay {
        1 date: String,
        /// Day of the lunar month (1 on the new moon's day).
        2 lunar_day: u32,
        /// The lunisolar month ("4", "leap 4") on its first day, else "".
        3 lunar_month: String,
        /// `new`, `first`, `full` or `last` on a phase's day, else "".
        4 phase: String,
    }
}

message! {
    /// `GET /api/v1/calendar?from&to[&system]`: public.
    pub struct CalendarView {
        1 system: String,
        2 from: String,
        3 to: String,
        4 days: Vec<CalendarDay>,
        5 observances: Vec<ObservanceView>,
        6 sunday_shift: bool,
    }
}

message! {
    /// A place in the time-zone picker.
    pub struct ZoneView {
        /// IANA name, what a phone reports (`America/New_York`).
        1 id: String,
        2 label: String,
    }
}

message! {
    /// `GET /api/v1/timezones`: public.
    pub struct ZoneList {
        1 zones: Vec<ZoneView>,
    }
}

message! {
    /// Where a precept set comes from: a CBETA work and fascicle.
    pub struct PreceptSource {
        /// CBETA work id (`T1484`).
        1 work: String,
        2 juan: u32,
        3 title_zh: String,
        4 title: String,
        /// Canon, volume, number, translator.
        5 canon: String,
    }
}

message! {
    /// One precept. `cite` and `label_cite` are CBETA line ids
    /// (`T24n1484_p1004b16`); `tools/precepts_check.py` checks them.
    pub struct Precept {
        /// Numbered as the source numbers it, within `kind`.
        1 n: u32,
        /// `major` or `minor`.
        2 kind: String,
        3 label_zh: String,
        4 title: String,
        /// Our rendering; see `docs/precepts/README.md`.
        5 text: String,
        /// Verbatim from the source at `cite[0]`.
        6 quote: String,
        7 cite: Vec<String>,
        /// Where `label_zh` comes from, when it's a commentary's title.
        8 label_cite: Option<String>,
    }
}

message! {
    /// A set of precepts: `catalog/precepts.json`, and `GET /api/v1/precepts` (Sprint 9).
    pub struct PreceptSet {
        /// Stable: people's records store it.
        1 key: String,
        2 title: String,
        3 zh: String,
        4 source: PreceptSource,
        /// `llm-draft` until the owner reviews it, then `llm-draft-reviewed`.
        5 provenance: String,
        6 items: Vec<Precept>,
    }
}

message! {
    /// `catalog/precepts.json`.
    pub struct PreceptCatalog {
        1 sets: Vec<PreceptSet>,
    }
}

message! {
    /// Something that happened to a vow: its timeline.
    pub struct StoredVowEvent {
        /// `took`, or a [`crate::vows::Action`] name.
        1 action: String,
        /// Household-local `YYYY-MM-DD`.
        2 date: String,
        /// UTC seconds.
        3 at: u64,
        4 note: String,
        /// Who else sees it: a breach defaults to `generic`.
        5 visibility: String,
        /// `repair`: the repentance entry it recorded.
        6 entry: u32,
    }
}

message! {
    pub struct StoredVow {
        1 id: u32,
        2 person: u32,
        3 text: String,
        /// `active`, `paused`, `breached`, `completed` or `retired`.
        4 state: String,
        5 started: String,
        /// Optional end date ("" = open-ended).
        6 ends: String,
        7 visibility: String,
        8 created: u64,
        9 rev: u32,
        /// Made from a taken precept: its set, kind and number.
        10 precept_set: String,
        11 precept_kind: String,
        12 precept_n: u32,
        13 events: Vec<StoredVowEvent>,
    }
}

message! {
    /// A dated post on a vow's blog.
    pub struct StoredPost {
        1 id: u32,
        2 vow: u32,
        3 person: u32,
        4 title: String,
        5 date: String,
        6 created: u64,
        7 rev: u32,
        /// The markdown, when it isn't on the card (no card, or an export).
        8 body: String,
        /// The body is `ml/posts/<id>.md` on the SD card.
        9 on_card: bool,
    }
}

message! {
    /// Which precepts of one set someone has taken.
    pub struct StoredPrecepts {
        1 person: u32,
        /// `catalog/precepts.json` set key.
        2 set: String,
        /// When they were taken ("" = not given).
        3 date: String,
        /// Where, with whom.
        4 note: String,
        /// Item numbers taken, ascending.
        5 majors: Vec<u32>,
        6 minors: Vec<u32>,
        7 visibility: String,
    }
}

message! {
    pub struct VowEventView {
        1 action: String,
        2 date: String,
        /// "Took this vow", "Renewed this vow", ...
        3 label: String,
        4 note: String,
        5 visibility: String,
    }
}

message! {
    pub struct VowView {
        1 id: u32,
        2 person: u32,
        3 person_name: String,
        /// Empty when `generic`.
        4 text: String,
        5 state: String,
        6 started: String,
        7 ends: String,
        8 visibility: String,
        9 rev: u32,
        10 precept_set: String,
        11 precept_kind: String,
        12 precept_n: u32,
        13 events: Vec<VowEventView>,
        14 posts: u32,
        15 can_edit: bool,
        /// Someone else's `generic` vow: shown as "A vow".
        16 generic: bool,
        /// Actions its person can take now.
        17 actions: Vec<String>,
    }
}

message! {
    pub struct PostView {
        1 id: u32,
        2 vow: u32,
        3 person: u32,
        4 person_name: String,
        5 title: String,
        6 date: String,
        7 created: u64,
        8 rev: u32,
        /// The markdown: only for whoever can edit it.
        9 body: String,
        /// Rendered by `markdown.rs` (safe).
        10 html: String,
        11 can_edit: bool,
        12 vow_text: String,
    }
}

message! {
    pub struct PreceptsView {
        1 person: u32,
        2 set: String,
        3 title: String,
        4 date: String,
        5 note: String,
        6 majors: Vec<u32>,
        7 minors: Vec<u32>,
        8 visibility: String,
        9 total_majors: u32,
        10 total_minors: u32,
        /// The set's renderings await the owner's review.
        11 draft: bool,
        /// Someone else's `generic` record: the set only, not which items.
        12 generic: bool,
    }
}

message! {
    /// `GET /api/v1/people/{id}/vows`: the profile's vows, precepts and blog.
    pub struct VowsPage {
        1 person: u32,
        2 person_name: String,
        3 own: bool,
        4 vows: Vec<VowView>,
        5 precepts: Vec<PreceptsView>,
        /// Newest first.
        6 posts: Vec<PostView>,
    }
}

message! {
    /// `GET /api/v1/vows/{id}`.
    pub struct VowDetail {
        1 vow: VowView,
        /// Newest first.
        2 posts: Vec<PostView>,
    }
}

message! {
    /// `POST /api/v1/vows`.
    pub struct VowInput {
        /// May be empty for a precept: its rendering is used.
        1 text: String,
        /// "" = today.
        2 started: String,
        3 ends: String,
        /// "" = public.
        4 visibility: String,
        5 precept_set: String,
        6 precept_kind: String,
        7 precept_n: u32,
    }
}

message! {
    /// `PATCH /api/v1/vows/{id}`.
    pub struct VowUpdate {
        1 rev: u32,
        2 text: Option<String>,
        3 ends: Option<String>,
        4 visibility: Option<String>,
    }
}

message! {
    /// `POST /api/v1/vows/{id}/{action}`.
    pub struct VowActionInput {
        1 note: String,
        /// For the event; "" = the action's default.
        2 visibility: String,
    }
}

message! {
    /// `POST /api/v1/vows/{id}/posts`, `PUT /api/v1/posts/{id}`.
    pub struct PostInput {
        1 title: String,
        2 body: String,
        /// "" = today.
        3 date: String,
        /// For `PUT`.
        4 rev: u32,
    }
}

message! {
    /// `PUT /api/v1/me/precepts/{set}`.
    pub struct PreceptsInput {
        1 date: String,
        2 note: String,
        3 majors: Vec<u32>,
        4 minors: Vec<u32>,
        5 visibility: String,
    }
}

message! {
    /// A household text: liturgy or reading someone added (`h<id>`).
    pub struct StoredText {
        1 id: u32,
        /// Who added it (they, or an admin, may change it).
        2 owner: u32,
        3 title: String,
        4 license: String,
        /// `nichiren`, `mahayana`, `theravada`, `zen`, ... or "".
        5 tradition: String,
        6 rev: u32,
        7 created: u64,
        /// The markdown, when it isn't on the card.
        8 body: String,
        /// The body is `ml/texts/<id>.md` on the SD card.
        9 on_card: bool,
    }
}

message! {
    pub struct TextView {
        /// `h<id>`.
        1 id: String,
        2 title: String,
        3 tradition: String,
        4 license: String,
        5 owner: u32,
        6 owner_name: String,
        7 rev: u32,
        /// Only from `GET /api/v1/texts/{id}`.
        8 body: String,
        9 can_edit: bool,
    }
}

message! {
    /// `GET /api/v1/texts`: household texts (built-ins ship with the UI).
    pub struct TextList {
        1 texts: Vec<TextView>,
    }
}

message! {
    /// `POST /api/v1/texts`, `PUT /api/v1/texts/{id}`.
    pub struct TextInput {
        1 title: String,
        2 body: String,
        3 tradition: String,
        4 license: String,
        /// For `PUT`.
        5 rev: u32,
    }
}

message! {
    /// One step of a practice (`spec/domain.md` § Texts and practices).
    pub struct StoredElement {
        /// `meditate`, `chant`, `prostrations`, `recite`, `read`, `copy`,
        /// `art` or `note`.
        1 kind: String,
        /// A heading ("" = the kind's or the text's name).
        2 title: String,
        /// A built-in text's id (`lotus-2`) or a household one (`h3`).
        3 text: String,
        4 minutes: u32,
        5 count: u32,
        /// A timer preset key (Sprint 6) for `meditate` and `chant`.
        6 timer: String,
        /// The practice kind "I did this" records ("" = none).
        7 logs_as: String,
        /// What to read, an art prompt, or free text.
        8 note: String,
    }
}

message! {
    pub struct StoredPracticeDoc {
        1 id: u32,
        2 owner: u32,
        3 title: String,
        /// `public` or `private`.
        4 visibility: String,
        5 elements: Vec<StoredElement>,
        6 rev: u32,
        7 created: u64,
    }
}

message! {
    pub struct PracticeView {
        1 id: u32,
        2 owner: u32,
        3 owner_name: String,
        4 title: String,
        5 visibility: String,
        6 elements: Vec<StoredElement>,
        7 rev: u32,
        8 can_edit: bool,
    }
}

message! {
    /// `GET /api/v1/practices`: the ones the viewer may see.
    pub struct PracticeList {
        1 practices: Vec<PracticeView>,
    }
}

message! {
    /// `POST /api/v1/practices`, `PUT /api/v1/practices/{id}`.
    pub struct PracticeInput {
        1 title: String,
        2 visibility: String,
        3 elements: Vec<StoredElement>,
        /// For `PUT`.
        4 rev: u32,
    }
}

message! {
    /// `POST /api/v1/practices/{id}/done`: "I did this".
    pub struct PracticeDone {
        /// "" = today.
        1 date: String,
    }
}
