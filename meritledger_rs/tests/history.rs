//! Sprint 5 end to end: entries in the SD journal, history beyond the
//! window, migration from the snapshot, export and import. The SD card is a
//! temporary directory; the clock is the machine's.
use meritledger::journal::Journal;
use meritledger::messages::StoredEntry;
use meritledger::sd::Sd;
use meritledger::state::DEFAULT_TZ;
use meritledger::tz::{Date, Tz};
use meritledger::{App, Limits};
use miniframework::kv::{Kv, MemKv};
use miniframework::{Reply, Request, Service};
use std::path::PathBuf;
use std::sync::Arc;

const ORIGIN: &str = "http://meritledger.local";

fn limits() -> Limits {
    Limits {
        pin_rounds: 2,
        ..Limits::board()
    }
}

fn card(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ml-history-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn open(kv: &Arc<dyn Kv>, sd: Option<&PathBuf>) -> App {
    let sd = sd.map_or_else(Sd::none, |d| Sd::at(d, None));
    App::open(kv.clone(), Arc::new(sd), limits()).unwrap()
}

fn today() -> Date {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    Tz::parse(DEFAULT_TZ).unwrap().local_date(now as i64)
}

struct Res {
    status: u16,
    body: String,
    cookie: Option<String>,
}

impl Res {
    fn num(&self, field: &str) -> i64 {
        let key = format!("\"{field}\":");
        let at = self
            .body
            .find(&key)
            .unwrap_or_else(|| panic!("{field} in {}", self.body))
            + key.len();
        match self.body[at..].split([',', '}']).next().unwrap() {
            "true" => 1,
            "false" => 0,
            n => n.parse().unwrap(),
        }
    }
}

fn call(app: &App, method: &str, path: &str, cookie: Option<&str>, body: &str) -> Res {
    let mut headers = vec![
        ("Host".to_string(), "meritledger.local".to_string()),
        ("Content-Type".to_string(), "application/json".to_string()),
        ("Origin".to_string(), ORIGIN.to_string()),
    ];
    if let Some(c) = cookie {
        headers.push(("Cookie".into(), c.into()));
    }
    let sep = if path.contains('?') { '&' } else { '?' };
    let uri = if path.ends_with(".csv") || path.ends_with(".json") {
        path.to_string()
    } else {
        format!("{path}{sep}fmt=json")
    };
    let req = Request::new(method, &uri, &headers, body.as_bytes(), false);
    let (mut out, mut spare) = (Vec::new(), Vec::new());
    let mut reply = Reply::new(&mut out, &mut spare, 1024 * 1024);
    app.handle(&req, &mut reply);
    let cookie = reply
        .headers()
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Set-Cookie"))
        .map(|(_, v)| v.split(';').next().unwrap().to_string());
    let body = match reply.owned_body() {
        Some(b) => String::from_utf8_lossy(b).into_owned(),
        None => String::from_utf8_lossy(reply.body()).into_owned(),
    };
    Res {
        status: reply.status(),
        body,
        cookie,
    }
}

/// Dad (admin, id 1) and Kid (member, id 2); returns Dad's cookie.
fn household(app: &App) -> String {
    let dad = call(
        app,
        "POST",
        "/api/v1/setup",
        None,
        r#"{"display_name":"Dad","pin":"1234","setup_code":""}"#,
    )
    .cookie
    .unwrap();
    let r = call(
        app,
        "POST",
        "/api/v1/admin/people",
        Some(&dad),
        r#"{"display_name":"Kid","role":"member","pin":"1111","email":""}"#,
    );
    assert_eq!(r.status, 201, "{}", r.body);
    dad
}

fn sign_in(app: &App, person: u32, pin: &str) -> String {
    call(
        app,
        "POST",
        "/api/v1/auth/pin",
        None,
        &format!(r#"{{"person":{person},"pin":"{pin}"}}"#),
    )
    .cookie
    .unwrap()
}

fn tap(app: &App, cookie: &str, person: u32, kind: &str) {
    let r = call(
        app,
        "POST",
        "/api/v1/entries",
        Some(cookie),
        &format!(r#"{{"person":{person},"kind":"{kind}","note":""}}"#),
    );
    assert_eq!(r.status, 201, "{}", r.body);
}

fn storage(app: &App, cookie: &str) -> Res {
    let r = call(app, "GET", "/api/v1/admin/storage", Some(cookie), "");
    assert_eq!(r.status, 200, "{}", r.body);
    r
}

#[test]
fn taps_append_to_the_journal_not_the_snapshot() {
    let dir = card("append");
    let kv: Arc<dyn Kv> = Arc::new(MemKv::default());
    let app = open(&kv, Some(&dir));
    let dad = household(&app);
    let before = storage(&app, &dad);
    assert_eq!(before.num("journaled"), 1);
    let commits = before.num("commits");
    for _ in 0..5 {
        tap(&app, &dad, 2, "meditation");
    }
    let after = storage(&app, &dad);
    assert_eq!(after.num("commits"), commits, "no snapshot commit per tap");
    assert_eq!(after.num("journal_records"), 5);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn journaled_entries_edits_and_deletes_survive_a_restart() {
    let dir = card("restart");
    let kv: Arc<dyn Kv> = Arc::new(MemKv::default());
    let dad = {
        let app = open(&kv, Some(&dir));
        let dad = household(&app);
        tap(&app, &dad, 0, "meditation");
        tap(&app, &dad, 0, "chanting");
        tap(&app, &dad, 0, "reading");
        call(
            &app,
            "PATCH",
            "/api/v1/entries/1",
            Some(&dad),
            r#"{"rev":1,"amount":45}"#,
        );
        call(&app, "DELETE", "/api/v1/entries/3", Some(&dad), "");
        dad
    };
    let app = open(&kv, Some(&dir));
    let today = call(&app, "GET", "/api/v1/today", Some(&dad), "");
    assert!(today.body.contains(r#""amount":45"#), "{}", today.body);
    assert!(
        today.body.contains("Chanting") && !today.body.contains("Reading Dharma ·"),
        "{}",
        today.body
    );
    let r = call(&app, "GET", "/api/v1/people/1/entries", Some(&dad), "");
    assert_eq!(r.body.matches(r#""id":"#).count(), 2, "{}", r.body);
    // New ids continue after the journal's.
    tap(&app, &dad, 0, "vegetarian");
    assert!(
        call(&app, "GET", "/api/v1/people/1/entries", Some(&dad), "")
            .body
            .contains(r#""id":4"#)
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn entries_move_from_the_snapshot_to_a_new_card_once() {
    let dir = card("migrate");
    let kv: Arc<dyn Kv> = Arc::new(MemKv::default());
    let dad = {
        let app = open(&kv, None);
        let dad = household(&app);
        tap(&app, &dad, 0, "meditation");
        tap(&app, &dad, 2, "prostrations");
        assert_eq!(storage(&app, &dad).num("journaled"), 0);
        dad
    };
    for _ in 0..2 {
        let app = open(&kv, Some(&dir));
        let s = storage(&app, &dad);
        assert_eq!(
            (s.num("journaled"), s.num("journal_records")),
            (1, 2),
            "{}",
            s.body
        );
        assert_eq!(s.num("window_entries"), 2);
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_crash_between_journal_and_snapshot_loses_and_doubles_nothing() {
    let dir = card("crash");
    let kv: Arc<dyn Kv> = Arc::new(MemKv::default());
    let dad = {
        let app = open(&kv, None);
        let dad = household(&app);
        tap(&app, &dad, 0, "meditation");
        tap(&app, &dad, 0, "chanting");
        dad
    };
    // Simulate a migration that wrote the journal but died before the
    // snapshot commit: the entries are in both places.
    let app = open(&kv, None);
    let exported = call(&app, "GET", "/api/v1/admin/export.json", Some(&dad), "");
    assert_eq!(exported.status, 200, "{}", exported.body);
    drop(app);
    let mut j = Journal::open(Arc::new(Sd::at(&dir, None))).unwrap();
    let doc: meritledger::messages::ExportDoc =
        miniframework::wire::decode(miniframework::wire::Format::Json, exported.body.as_bytes())
            .unwrap();
    j.append(&doc.entries).unwrap();
    let app = open(&kv, Some(&dir));
    let r = call(&app, "GET", "/api/v1/people/1/entries", Some(&dad), "");
    assert_eq!(
        r.body.matches(r#""id":"#).count(),
        2,
        "no duplicates: {}",
        r.body
    );
    std::fs::remove_dir_all(dir).unwrap();
}

/// Writes entries straight into the journal, as older days would be.
fn old_entries(dir: &PathBuf, list: &[(u32, u32, &str, i64)]) {
    let mut j = Journal::open(Arc::new(Sd::at(dir, None))).unwrap();
    let records: Vec<StoredEntry> = list
        .iter()
        .map(|&(id, person, kind, days_ago)| StoredEntry {
            id,
            person,
            kind: kind.into(),
            date: today().add_days(-days_ago).iso(),
            amount: if kind == "meditation" { 30 } else { 0 },
            visibility: "public".into(),
            source: "tap".into(),
            recorded_by: person,
            rev: 1,
            ..Default::default()
        })
        .collect();
    j.append(&records).unwrap();
}

#[test]
fn history_beyond_the_window_feeds_stats_and_reads_back() {
    let dir = card("old");
    let kv: Arc<dyn Kv> = Arc::new(MemKv::default());
    let dad = {
        let app = open(&kv, Some(&dir));
        household(&app)
    };
    // 120 days of daily meditation for the kid, ending yesterday.
    let list: Vec<(u32, u32, &str, i64)> = (1..=120)
        .map(|d| (1000 + d as u32, 2, "meditation", d))
        .collect();
    old_entries(&dir, &list);
    let app = open(&kv, Some(&dir));
    let s = storage(&app, &dad);
    assert_eq!(s.num("window_entries"), 59, "{}", s.body);
    assert_eq!(s.num("history_days"), 61);
    let p = call(&app, "GET", "/api/v1/people/2/profile", None, "");
    assert_eq!(p.num("current_streak"), 120, "{}", p.body);
    assert_eq!(p.num("longest_streak"), 120);
    let from = today().add_days(-100).iso();
    assert!(
        p.body.contains(&format!(
            "\"history_from\":\"{}\"",
            today().add_days(-120).iso()
        )),
        "{}",
        p.body
    );
    // Entries from before the window come back from the journal.
    let old = call(
        &app,
        "GET",
        &format!(
            "/api/v1/people/2/entries?from={from}&to={}",
            today().add_days(-90).iso()
        ),
        None,
        "",
    );
    assert_eq!(old.body.matches(r#""id":"#).count(), 11, "{}", old.body);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn export_then_import_into_an_empty_board_gives_the_same_profile() {
    let src = card("export");
    let kv: Arc<dyn Kv> = Arc::new(MemKv::default());
    let dad = {
        let app = open(&kv, Some(&src));
        household(&app)
    };
    let list: Vec<(u32, u32, &str, i64)> = (1..=90)
        .map(|d| (2000 + d as u32, 2, "meditation", d))
        .collect();
    old_entries(&src, &list);
    let app = open(&kv, Some(&src));
    let kid = sign_in(&app, 2, "1111");
    tap(&app, &kid, 0, "chanting");
    let before = call(&app, "GET", "/api/v1/people/2/profile", None, "");
    let doc = call(&app, "GET", "/api/v1/admin/export.json", Some(&dad), "");
    assert_eq!(doc.status, 200);
    assert!(
        doc.body.contains("meritledger-export") && !doc.body.contains("pbkdf2"),
        "no PIN hashes in exports"
    );

    // A fresh board with a card: set up, dry run, then import twice.
    let dst = card("import");
    let kv2: Arc<dyn Kv> = Arc::new(MemKv::default());
    let app2 = open(&kv2, Some(&dst));
    let dad2 = call(
        &app2,
        "POST",
        "/api/v1/setup",
        None,
        r#"{"display_name":"Dad","pin":"9999","setup_code":""}"#,
    )
    .cookie
    .unwrap();
    let dry = call(
        &app2,
        "POST",
        "/api/v1/admin/import?dry_run=1",
        Some(&dad2),
        &doc.body,
    );
    assert_eq!(dry.status, 200, "{}", dry.body);
    assert_eq!(
        (
            dry.num("people_matched"),
            dry.num("people_created"),
            dry.num("entries_added")
        ),
        (1, 1, 91)
    );
    assert_eq!(
        storage(&app2, &dad2).num("people"),
        1,
        "a dry run changes nothing"
    );
    let done = call(
        &app2,
        "POST",
        "/api/v1/admin/import",
        Some(&dad2),
        &doc.body,
    );
    assert_eq!(done.num("entries_added"), 91, "{}", done.body);
    let again = call(
        &app2,
        "POST",
        "/api/v1/admin/import",
        Some(&dad2),
        &doc.body,
    );
    assert_eq!(
        (again.num("entries_added"), again.num("entries_skipped")),
        (0, 91),
        "{}",
        again.body
    );
    let after = call(&app2, "GET", "/api/v1/people/2/profile", None, "");
    for field in ["current_streak", "longest_streak", "week_minutes"] {
        assert_eq!(
            after.num(field),
            before.num(field),
            "{field}: {}",
            after.body
        );
    }
    // And after a restart, from the new board's own journal.
    drop(app2);
    let app2 = open(&kv2, Some(&dst));
    assert_eq!(
        call(&app2, "GET", "/api/v1/people/2/profile", None, "").num("longest_streak"),
        before.num("longest_streak")
    );
    std::fs::remove_dir_all(src).unwrap();
    std::fs::remove_dir_all(dst).unwrap();
}

#[test]
fn csv_is_your_own_or_an_admins() {
    let kv: Arc<dyn Kv> = Arc::new(MemKv::default());
    let app = open(&kv, None);
    let dad = household(&app);
    let kid = sign_in(&app, 2, "1111");
    call(
        &app,
        "POST",
        "/api/v1/entries",
        Some(&kid),
        r#"{"person":0,"kind":"anger","note":"at \"dinner\", sorry"}"#,
    );
    let own = call(&app, "GET", "/api/v1/people/2/entries.csv", Some(&kid), "");
    assert_eq!(own.status, 200, "{}", own.body);
    assert!(own.body.starts_with("date,practice,"), "{}", own.body);
    assert!(
        own.body.contains("\"at \"\"dinner\"\", sorry\""),
        "{}",
        own.body
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/people/1/entries.csv", Some(&kid), "").status,
        403
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/people/2/entries.csv", Some(&dad), "").status,
        200
    );
}

#[test]
fn a_missing_card_is_reported() {
    let dir = card("missing");
    let kv: Arc<dyn Kv> = Arc::new(MemKv::default());
    let dad = {
        let app = open(&kv, Some(&dir));
        household(&app)
    };
    let app = open(&kv, None);
    assert_eq!(storage(&app, &dad).num("card_missing"), 1);
    // Nothing was written, so the folder may not exist.
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn backing_up_now_writes_and_rotates() {
    let dir = card("backup");
    let kv: Arc<dyn Kv> = Arc::new(MemKv::default());
    let app = open(&kv, Some(&dir));
    let dad = household(&app);
    tap(&app, &dad, 0, "meditation");
    // Old backups to rotate away.
    std::fs::create_dir_all(dir.join("ml/backup")).unwrap();
    for d in 20..60 {
        std::fs::write(
            dir.join(format!("ml/backup/{}.json", today().add_days(-d).iso())),
            b"{}",
        )
        .unwrap();
    }
    let r = call(&app, "POST", "/api/v1/admin/backups", Some(&dad), "");
    assert_eq!(r.status, 201, "{}", r.body);
    let name = format!("{}.json", today().iso());
    assert!(r.body.contains(&name), "{}", r.body);
    let kept = std::fs::read_dir(dir.join("ml/backup")).unwrap().count();
    assert!(kept <= 14 + 3, "{kept} kept");
    let written = std::fs::read_to_string(dir.join("ml/backup").join(&name)).unwrap();
    assert!(written.contains("meritledger-export") && written.contains("meditation"));
    let s = storage(&app, &dad);
    assert!(
        s.body.contains(&format!("\"last_backup\":\"{name}\"")),
        "{}",
        s.body
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[cfg(feature = "gate")]
#[test]
fn the_gate_journal_bench_cleans_up() {
    let dir = card("bench");
    let sd = Arc::new(Sd::at(&dir, None));
    let scratch = Arc::new(sd.sub("ml/gate/bench").unwrap());
    let b = meritledger::gate::journal_bench(scratch, 1_200).unwrap();
    assert_eq!(b.records, 1_220);
    assert!(sd.list("ml/gate/bench/ml/journal").unwrap().is_empty());
    std::fs::remove_dir_all(dir).unwrap();
}
