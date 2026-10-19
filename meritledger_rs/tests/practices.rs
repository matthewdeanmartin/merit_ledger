//! Sprint 8 end to end: household texts (bodies on the SD card), built
//! practices, following one without signing in, "I did this", and backups.
//! The desktop clock is real, so "today" is the machine's.
use meritledger::sd::Sd;
use meritledger::{App, Limits};
use miniframework::kv::MemKv;
use miniframework::{Reply, Request, Service};
use std::path::PathBuf;
use std::sync::Arc;

const ORIGIN: &str = "http://meritledger.local";

fn card(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ml-practices-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn app(sd: Option<&PathBuf>) -> App {
    let limits = Limits {
        pin_rounds: 2,
        ..Limits::board()
    };
    let sd = sd.map_or_else(Sd::none, |d| Sd::at(d, None));
    App::open(Arc::new(MemKv::default()), Arc::new(sd), limits).unwrap()
}

struct Res {
    status: u16,
    body: String,
    cookie: Option<String>,
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
    let uri = format!("{path}{sep}fmt=json");
    let req = Request::new(method, &uri, &headers, body.as_bytes(), false);
    let (mut out, mut spare) = (Vec::new(), Vec::new());
    let mut reply = Reply::new(&mut out, &mut spare, 256 * 1024);
    app.handle(&req, &mut reply);
    let cookie = reply
        .headers()
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Set-Cookie"))
        .map(|(_, v)| v.split(';').next().unwrap().to_string());
    // Exports come back as an owned body.
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

fn ok(r: Res, status: u16) -> String {
    assert_eq!(r.status, status, "{}", r.body);
    r.body
}

/// Dad (admin, id 1) and Kid (member, id 2), both signed in.
fn household(app: &App) -> (String, String) {
    let dad = call(
        app,
        "POST",
        "/api/v1/setup",
        None,
        r#"{"display_name":"Dad","pin":"1234","setup_code":""}"#,
    )
    .cookie
    .unwrap();
    ok(
        call(
            app,
            "POST",
            "/api/v1/admin/people",
            Some(&dad),
            r#"{"display_name":"Kid","role":"member","pin":"1111","email":""}"#,
        ),
        201,
    );
    let kid = call(
        app,
        "POST",
        "/api/v1/auth/pin",
        None,
        r#"{"person":2,"pin":"1111"}"#,
    )
    .cookie
    .unwrap();
    (dad, kid)
}

const GONGYO: &str = r#"{"title":"Morning gongyo","visibility":"public","elements":[
  {"kind":"chant","title":"Three daimoku","text":"daimoku","count":3},
  {"kind":"recite","text":"lotus-2","logs_as":"recite"},
  {"kind":"recite","text":"lotus-16-verses"},
  {"kind":"chant","text":"daimoku","count":108,"timer":"mala108","logs_as":"daimoku"},
  {"kind":"recite","text":"silent-prayers","logs_as":"dedication"}
]}"#;

#[test]
fn a_practice_anyone_can_follow_and_its_builder_can_log() {
    let app = app(None);
    let (dad, kid) = household(&app);
    let body = ok(
        call(&app, "POST", "/api/v1/practices", Some(&dad), GONGYO),
        201,
    );
    assert!(body.contains(r#""title":"Morning gongyo""#), "{body}");
    // Readers at the shrine needn't sign in.
    let read = ok(call(&app, "GET", "/api/v1/practices/1", None, ""), 200);
    assert!(
        read.contains("lotus-16-verses") && read.contains(r#""can_edit":false"#),
        "{read}"
    );
    // "I did this" logs the steps that log: recite, 108 daimoku, dedication.
    let today = ok(
        call(&app, "POST", "/api/v1/practices/1/done", Some(&kid), ""),
        201,
    );
    assert!(
        today.contains(r#""kind":"daimoku""#) && today.contains(r#""amount":108"#),
        "{today}"
    );
    assert_eq!(
        today.matches(r#""note":"Morning gongyo""#).count(),
        3,
        "{today}"
    );
    assert_eq!(
        call(&app, "POST", "/api/v1/practices/1/done", None, "").status,
        401
    );
    // Only its builder changes it.
    let edit = GONGYO.replacen('{', r#"{"rev":1,"#, 1);
    assert_eq!(
        call(&app, "PUT", "/api/v1/practices/1", Some(&kid), &edit).status,
        403
    );
    ok(
        call(&app, "PUT", "/api/v1/practices/1", Some(&dad), &edit),
        200,
    );
    assert_eq!(
        call(&app, "PUT", "/api/v1/practices/1", Some(&dad), &edit).status,
        409
    );
    // Private ones are their builder's.
    let private = GONGYO.replace(r#""visibility":"public""#, r#""visibility":"private""#);
    ok(
        call(&app, "POST", "/api/v1/practices", Some(&kid), &private),
        201,
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/practices/2", Some(&dad), "").status,
        200,
        "admins see all"
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/practices/2", None, "").status,
        404
    );
    let list = ok(call(&app, "GET", "/api/v1/practices", None, ""), 200);
    assert_eq!(list.matches(r#""id":"#).count(), 1, "{list}");
    ok(
        call(&app, "DELETE", "/api/v1/practices/1", Some(&dad), ""),
        200,
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/practices/1", None, "").status,
        404
    );
}

#[test]
fn household_texts_live_on_the_card_and_travel_in_backups() {
    let dir = card("texts");
    let app1 = app(Some(&dir));
    let (dad, _) = household(&app1);
    let text = r#"{"title":"Our evening verse","body":"May this house be at peace.","tradition":"mahayana","license":"ours"}"#;
    let list = ok(call(&app1, "POST", "/api/v1/texts", Some(&dad), text), 201);
    assert!(list.contains(r#""id":"h1""#), "{list}");
    assert_eq!(
        std::fs::read_to_string(dir.join("ml/texts/1.md")).unwrap(),
        "May this house be at peace."
    );
    let one = ok(call(&app1, "GET", "/api/v1/texts/h1", None, ""), 200);
    assert!(one.contains("May this house be at peace."), "{one}");
    let uses =
        r#"{"title":"Evening","elements":[{"kind":"recite","text":"h1","logs_as":"recite"}]}"#;
    ok(
        call(&app1, "POST", "/api/v1/practices", Some(&dad), uses),
        201,
    );
    let missing = r#"{"title":"Broken","elements":[{"kind":"recite","text":"h9"}]}"#;
    assert_eq!(
        call(&app1, "POST", "/api/v1/practices", Some(&dad), missing).status,
        400
    );

    let export = ok(
        call(&app1, "GET", "/api/v1/admin/export.json", Some(&dad), ""),
        200,
    );
    assert!(
        export.contains("May this house be at peace."),
        "the body travels"
    );
    let dir2 = card("texts-import");
    let app2 = app(Some(&dir2));
    let (dad2, _) = household(&app2);
    // A text of the board's own comes first, so the imported one is renumbered.
    let other = r#"{"title":"Already here","body":"x","tradition":"","license":""}"#;
    ok(
        call(&app2, "POST", "/api/v1/texts", Some(&dad2), other),
        201,
    );
    let report = ok(
        call(&app2, "POST", "/api/v1/admin/import", Some(&dad2), &export),
        200,
    );
    assert!(
        report.contains(r#""texts_added":1"#) && report.contains(r#""practices_added":1"#),
        "{report}"
    );
    let practice = ok(
        call(&app2, "GET", "/api/v1/practices/1", Some(&dad2), ""),
        200,
    );
    assert!(
        practice.contains(r#""text":"h2""#),
        "the step follows its text: {practice}"
    );
    assert_eq!(
        std::fs::read_to_string(dir2.join("ml/texts/2.md")).unwrap(),
        "May this house be at peace."
    );
    ok(
        call(&app2, "DELETE", "/api/v1/texts/h2", Some(&dad2), ""),
        200,
    );
    assert!(!dir2.join("ml/texts/2.md").exists());
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}
