//! The API end to end, through `Service::handle` with real requests:
//! status, setup, PIN sign-in, sessions, people, admin, persistence.
use meritledger::pin::GuardLimits;
use meritledger::sd::Sd;
use meritledger::{App, Limits};
use miniframework::kv::{Kv, MemKv};
use miniframework::{Reply, Request, Service};
use std::sync::Arc;

const ORIGIN: &str = "http://meritledger.local";

fn limits() -> Limits {
    Limits {
        pin_rounds: 2,
        ..Limits::board()
    }
}

fn open(kv: Arc<dyn Kv>) -> App {
    App::open(kv, Arc::new(Sd::none()), limits()).unwrap()
}

fn app() -> App {
    open(Arc::new(MemKv::default()))
}

struct Res {
    status: u16,
    body: String,
    cookie: Option<String>,
    retry_after: Option<String>,
}

impl Res {
    fn json(&self, field: &str) -> String {
        // Tiny extractor: "field":value or "field":"value".
        let key = format!("\"{field}\":");
        let at = self
            .body
            .find(&key)
            .unwrap_or_else(|| panic!("{field} in {}", self.body))
            + key.len();
        let rest = &self.body[at..];
        if let Some(stripped) = rest.strip_prefix('"') {
            stripped[..stripped.find('"').unwrap()].to_string()
        } else {
            rest[..rest.find([',', '}']).unwrap()].to_string()
        }
    }
}

fn call_from(
    app: &App,
    method: &str,
    path: &str,
    cookie: Option<&str>,
    body: &str,
    origin: Option<&str>,
    secure: bool,
) -> Res {
    let mut headers = vec![
        ("Host".to_string(), "meritledger.local".to_string()),
        ("Content-Type".to_string(), "application/json".to_string()),
    ];
    if let Some(o) = origin {
        headers.push(("Origin".into(), o.into()));
    }
    if let Some(c) = cookie {
        headers.push(("Cookie".into(), c.into()));
    }
    let sep = if path.contains('?') { '&' } else { '?' };
    let uri = format!("{path}{sep}fmt=json");
    let req = Request::new(method, &uri, &headers, body.as_bytes(), secure);
    let (mut out, mut spare) = (Vec::new(), Vec::new());
    let mut reply = Reply::new(&mut out, &mut spare, 64 * 1024);
    app.handle(&req, &mut reply);
    let find = |name: &str| {
        reply
            .headers()
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.clone())
    };
    Res {
        status: reply.status(),
        body: String::from_utf8_lossy(reply.body()).into_owned(),
        cookie: find("Set-Cookie").map(|c| c.split(';').next().unwrap().to_string()),
        retry_after: find("Retry-After"),
    }
}

fn call(app: &App, method: &str, path: &str, cookie: Option<&str>, body: &str) -> Res {
    call_from(app, method, path, cookie, body, Some(ORIGIN), false)
}

/// Sets the board up as "Dad" (PIN 1234); returns the session cookie.
fn setup(app: &App) -> String {
    let r = call(
        app,
        "POST",
        "/api/v1/setup",
        None,
        r#"{"display_name":"Dad","pin":"1234","setup_code":""}"#,
    );
    assert_eq!(r.status, 201, "{}", r.body);
    r.cookie.expect("setup signs in")
}

fn add(app: &App, admin: &str, name: &str, role: &str, pin: &str) -> u32 {
    let body = format!(r#"{{"display_name":"{name}","role":"{role}","pin":"{pin}","email":""}}"#);
    let r = call(app, "POST", "/api/v1/admin/people", Some(admin), &body);
    assert_eq!(r.status, 201, "{}", r.body);
    let people = call(app, "GET", "/api/v1/people", None, "");
    let at = people
        .body
        .find(&format!("\"display_name\":\"{name}\""))
        .unwrap();
    let before = &people.body[..at];
    let id_at = before.rfind("\"id\":").unwrap() + 5;
    before[id_at..].split(',').next().unwrap().parse().unwrap()
}

fn sign_in(app: &App, person: u32, pin: &str) -> Res {
    call(
        app,
        "POST",
        "/api/v1/auth/pin",
        None,
        &format!(r#"{{"person":{person},"pin":"{pin}"}}"#),
    )
}

#[test]
fn status_before_and_after_setup() {
    let app = app();
    let r = call(&app, "GET", "/api/v1/status", None, "");
    assert_eq!(r.status, 200);
    assert_eq!(r.json("app"), "meritledger");
    assert_eq!(r.json("version"), env!("CARGO_PKG_VERSION"));
    assert_eq!(r.json("setup_needed"), "true");
    setup(&app);
    assert_eq!(
        call(&app, "GET", "/api/v1/status", None, "").json("setup_needed"),
        "false"
    );
}

#[test]
fn status_reports_tls() {
    let app = app();
    let secure = call_from(&app, "GET", "/api/v1/status", None, "", None, true);
    assert_eq!(secure.json("secure"), "true");
    assert_eq!(
        call(&app, "GET", "/api/v1/status", None, "").json("secure"),
        "false"
    );
}

#[test]
fn unknown_route_is_left_to_the_site() {
    // The site turns an untouched reply into a 404.
    assert_eq!(call(&app(), "GET", "/api/v1/nope", None, "").status, 0);
}

#[test]
fn setup_happens_once() {
    let app = app();
    setup(&app);
    let again = call(
        &app,
        "POST",
        "/api/v1/setup",
        None,
        r#"{"display_name":"Eve","pin":"1234","setup_code":""}"#,
    );
    assert_eq!(again.status, 409, "{}", again.body);
}

#[test]
fn concurrent_setups_make_exactly_one_admin() {
    let app = Arc::new(app());
    let handles: Vec<_> = (0..8)
        .map(|i| {
            let app = app.clone();
            std::thread::spawn(move || {
                let body = format!(r#"{{"display_name":"P{i}","pin":"1234","setup_code":""}}"#);
                call(&app, "POST", "/api/v1/setup", None, &body).status
            })
        })
        .collect();
    let mut statuses: Vec<u16> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    statuses.sort();
    assert_eq!(statuses, vec![201, 409, 409, 409, 409, 409, 409, 409]);
}

#[test]
fn setup_code_is_checked() {
    let mut app = app();
    app.setup_code = Some("lotus");
    let wrong = call(
        &app,
        "POST",
        "/api/v1/setup",
        None,
        r#"{"display_name":"Dad","pin":"1234","setup_code":"nope"}"#,
    );
    assert_eq!(wrong.status, 403);
    let right = call(
        &app,
        "POST",
        "/api/v1/setup",
        None,
        r#"{"display_name":"Dad","pin":"1234","setup_code":"lotus"}"#,
    );
    assert_eq!(right.status, 201, "{}", right.body);
}

#[test]
fn changes_need_the_boards_own_origin() {
    let app = app();
    let r = call_from(
        &app,
        "POST",
        "/api/v1/setup",
        None,
        r#"{"display_name":"Dad","pin":"1234","setup_code":""}"#,
        Some("http://evil.example"),
        false,
    );
    assert_eq!(r.status, 403, "{}", r.body);
    assert_eq!(
        call(&app, "GET", "/api/v1/status", None, "").json("setup_needed"),
        "true"
    );
}

#[test]
fn kids_sign_in_by_name_and_pin() {
    let app = app();
    let dad = setup(&app);
    let kid = add(&app, &dad, "Ada", "member", "2468");
    let r = sign_in(&app, kid, "2468");
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(r.json("display_name"), "Ada");
    let cookie = r.cookie.unwrap();
    assert_eq!(
        call(&app, "GET", "/api/v1/me", Some(&cookie), "").json("role"),
        "member"
    );
    // A member isn't an admin.
    assert_eq!(
        call(&app, "GET", "/api/v1/admin/people", Some(&cookie), "").status,
        403
    );
}

#[test]
fn people_without_a_pin_are_listed_but_cannot_sign_in() {
    let app = app();
    let dad = setup(&app);
    let baby = add(&app, &dad, "Baby", "member", "");
    let people = call(&app, "GET", "/api/v1/people", None, "");
    assert!(
        people
            .body
            .contains(r#""display_name":"Baby","role":"member","can_sign_in":false"#),
        "{}",
        people.body
    );
    assert_eq!(sign_in(&app, baby, "0000").status, 403);
}

#[test]
fn guests_sign_up_while_open() {
    let app = app();
    let dad = setup(&app);
    let r = call(
        &app,
        "POST",
        "/api/v1/auth/signup",
        None,
        r#"{"display_name":"Visitor","pin":"1357"}"#,
    );
    assert_eq!(r.status, 201, "{}", r.body);
    assert_eq!(r.json("role"), "guest");
    let closed = call(
        &app,
        "PATCH",
        "/api/v1/admin/settings",
        Some(&dad),
        r#"{"signup_open":false}"#,
    );
    assert_eq!(closed.json("signup_open"), "false");
    let late = call(
        &app,
        "POST",
        "/api/v1/auth/signup",
        None,
        r#"{"display_name":"Late","pin":"1357"}"#,
    );
    assert_eq!(late.status, 403);
}

#[test]
fn wrong_pins_lock_with_retry_after() {
    let app = App::open_with(
        Arc::new(MemKv::default()),
        Arc::new(Sd::none()),
        limits(),
        GuardLimits {
            free_failures: 3,
            ..GuardLimits::default()
        },
    )
    .unwrap();
    let dad = setup(&app);
    let kid = add(&app, &dad, "Kid", "member", "1111");
    for _ in 0..3 {
        assert_eq!(sign_in(&app, kid, "9999").status, 401);
    }
    let locked = sign_in(&app, kid, "1111");
    assert_eq!(locked.status, 429, "{}", locked.body);
    assert_eq!(locked.retry_after.as_deref(), Some("60"));
    // An admin resetting the PIN clears the lock.
    let r = call(
        &app,
        "PATCH",
        &format!("/api/v1/admin/people/{kid}"),
        Some(&dad),
        r#"{"pin":"2222"}"#,
    );
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(sign_in(&app, kid, "2222").status, 200);
}

#[test]
fn sign_out_here_and_everywhere() {
    let app = app();
    let first = setup(&app);
    let second = sign_in(&app, 1, "1234").cookie.unwrap();
    let third = sign_in(&app, 1, "1234").cookie.unwrap();
    assert_eq!(
        call(&app, "GET", "/api/v1/me", Some(&first), "").json("sessions"),
        "3"
    );
    call(&app, "POST", "/api/v1/auth/logout", Some(&first), "");
    assert_eq!(
        call(&app, "GET", "/api/v1/me", Some(&first), "").status,
        401
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/me", Some(&second), "").status,
        200
    );
    call(&app, "POST", "/api/v1/auth/logout?all=1", Some(&second), "");
    assert_eq!(
        call(&app, "GET", "/api/v1/me", Some(&third), "").status,
        401
    );
}

#[test]
fn sessions_per_person_are_bounded() {
    let app = app();
    let oldest = setup(&app);
    for _ in 0..4 {
        sign_in(&app, 1, "1234");
    }
    // Four per person: the setup session was evicted.
    assert_eq!(
        call(&app, "GET", "/api/v1/me", Some(&oldest), "").status,
        401
    );
}

#[test]
fn removing_a_pin_signs_them_out() {
    let app = app();
    let dad = setup(&app);
    let kid = add(&app, &dad, "Kid", "member", "1111");
    let kid_cookie = sign_in(&app, kid, "1111").cookie.unwrap();
    call(
        &app,
        "PATCH",
        &format!("/api/v1/admin/people/{kid}"),
        Some(&dad),
        r#"{"pin":""}"#,
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/me", Some(&kid_cookie), "").status,
        401
    );
}

#[test]
fn the_seventeenth_person_is_refused() {
    let app = app();
    let dad = setup(&app);
    for i in 2..=16 {
        add(&app, &dad, &format!("P{i}"), "member", "");
    }
    let r = call(
        &app,
        "POST",
        "/api/v1/admin/people",
        Some(&dad),
        r#"{"display_name":"P17","role":"member","pin":"","email":""}"#,
    );
    assert_eq!(r.status, 409);
    assert!(r.body.contains("full"), "{}", r.body);
}

#[test]
fn state_and_sessions_survive_a_restart() {
    let kv: Arc<dyn Kv> = Arc::new(MemKv::default());
    let (dad, kid) = {
        let app = open(kv.clone());
        let dad = setup(&app);
        let kid = add(&app, &dad, "Kid", "member", "1111");
        (dad, kid)
    };
    let app = open(kv);
    assert_eq!(
        call(&app, "GET", "/api/v1/me", Some(&dad), "").json("display_name"),
        "Dad"
    );
    assert_eq!(sign_in(&app, kid, "1111").status, 200);
    let storage = call(&app, "GET", "/api/v1/admin/storage", Some(&dad), "");
    assert_eq!(storage.json("people"), "2");
    assert_eq!(storage.json("sd_present"), "false");
}

#[test]
fn a_torn_newest_slot_loads_the_previous_generation() {
    let kv: Arc<dyn Kv> = Arc::new(MemKv::default());
    let dad = {
        let app = open(kv.clone());
        let dad = setup(&app);
        add(&app, &dad, "Kid", "member", "");
        dad
    };
    // Tear whichever slot holds the newest generation.
    let newest = ["snap0", "snap1"]
        .into_iter()
        .max_by_key(|k| {
            kv.get(k)
                .map(|b| u64::from_le_bytes(b[8..16].try_into().unwrap()))
        })
        .unwrap();
    let mut bytes = kv.get(newest).unwrap();
    bytes.truncate(bytes.len() - 3);
    kv.set(newest, &bytes).unwrap();
    let app = open(kv);
    let storage = call(&app, "GET", "/api/v1/admin/storage", Some(&dad), "");
    assert_eq!(storage.json("fallback_used"), "true");
    assert_eq!(
        storage.json("people"),
        "1",
        "the previous generation had only Dad"
    );
}

#[test]
fn both_slots_corrupt_refuses_to_start() {
    let kv: Arc<dyn Kv> = Arc::new(MemKv::default());
    kv.set("snap0", b"junk").unwrap();
    kv.set("snap1", b"MLS1 junk").unwrap();
    assert!(App::open(kv, Arc::new(Sd::none()), limits()).is_err());
}

#[test]
fn storage_is_admin_only() {
    let app = app();
    assert_eq!(
        call(&app, "GET", "/api/v1/admin/storage", None, "").status,
        401
    );
}
