//! Sprint 12 end to end: a book from an Open Library search on the reading
//! list, a reading session logged against it, and that session counted as
//! minutes of reading. The desktop clock is real.
use meritledger::sd::Sd;
use meritledger::{App, Limits};
use miniframework::kv::MemKv;
use miniframework::{Reply, Request, Service};
use std::sync::Arc;

const ORIGIN: &str = "http://meritledger.local";

fn app() -> App {
    let limits = Limits {
        pin_rounds: 2,
        ..Limits::board()
    };
    App::open(Arc::new(MemKv::default()), Arc::new(Sd::none()), limits).unwrap()
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

#[test]
fn a_reading_session_counts_as_reading() {
    let app = app();
    let (dad, kid) = household(&app);
    let book = r#"{"work":"/works/OL263645W","title":"The Heart of Understanding","author":"Thich Nhat Hanh","cover":715880,"pages":54,"year":1987,"status":"","visibility":""}"#;
    let list = ok(call(&app, "POST", "/api/v1/reading", Some(&dad), book), 201);
    assert!(
        list.contains(r#""work":"OL263645W""#) && list.contains(r#""status":"want""#),
        "{list}"
    );
    let list = ok(
        call(
            &app,
            "POST",
            "/api/v1/reading/1/log",
            Some(&dad),
            r#"{"minutes":25,"page":20,"note":""}"#,
        ),
        200,
    );
    assert!(
        list.contains(r#""status":"reading""#) && list.contains(r#""minutes":25"#),
        "{list}"
    );
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/reading/1/log",
            Some(&kid),
            r#"{"minutes":5,"page":0,"note":""}"#
        )
        .status,
        403
    );
    // The session is reading practice: in the profile's week, by category.
    let profile = ok(
        call(&app, "GET", "/api/v1/people/1/profile", Some(&kid), ""),
        200,
    );
    assert!(profile.contains(r#""week_minutes":25"#), "{profile}");
    assert!(profile.contains(r#""category":"reading""#), "{profile}");
    // The house's shelf and Dad's list, as Kid sees them.
    let shelf = ok(call(&app, "GET", "/api/v1/shelf", Some(&kid), ""), 200);
    assert!(
        shelf.contains("The Heart of Understanding") && shelf.contains(r#""person_name":"Dad""#),
        "{shelf}"
    );
    let theirs = ok(
        call(&app, "GET", "/api/v1/reading?person=1", Some(&kid), ""),
        200,
    );
    assert!(theirs.contains(r#""can_edit":false"#), "{theirs}");
    // A practice's "read" step can point at it.
    let practice = r#"{"title":"Evening reading","visibility":"public","elements":[{"kind":"read","book":"r1","minutes":20,"logs_as":"reading"}]}"#;
    ok(
        call(&app, "POST", "/api/v1/practices", Some(&dad), practice),
        201,
    );
    let missing =
        r#"{"title":"Broken","visibility":"public","elements":[{"kind":"read","book":"r9"}]}"#;
    assert_eq!(
        call(&app, "POST", "/api/v1/practices", Some(&dad), missing).status,
        400
    );
}
