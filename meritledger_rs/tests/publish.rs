//! Sprint 11 end to end: sharing and auto-publishing to Mastodon (as the
//! person, or as the household bot), and statuses following edits and
//! deletes, against a fake instance.
use meritledger::outbound::{MailSetup, Outbox, Setup};
use meritledger::sd::Sd;
use meritledger::{App, Limits};
use miniframework::fetch::{Fetch, Fetched, Outgoing};
use miniframework::kv::MemKv;
use miniframework::{Reply, Request, Service};
use std::sync::{Arc, Mutex};

const ORIGIN: &str = "http://meritledger.local";

fn app(_: Option<()>) -> App {
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

/// mastomini and Brevo, faked: records what the board sends.
#[derive(Default)]
struct Sink {
    sent: Mutex<Vec<(String, String)>>,
}

impl Fetch for Sink {
    fn get(&self, _: &str, _: &str, _: usize) -> std::io::Result<Fetched> {
        Err(std::io::Error::other("unused"))
    }

    fn send(&self, r: &Outgoing<'_>) -> std::io::Result<Fetched> {
        self.sent.lock().unwrap().push((
            format!("{} {}", r.method, r.url),
            String::from_utf8_lossy(r.body).into_owned(),
        ));
        let body = if r.url.ends_with("/verify_credentials") {
            r#"{"id":"9","username":"meritledger"}"#
        } else if r.url.ends_with("/senders") {
            r#"{"senders":[{"name":"Matt","email":"matt@example.com","active":true}]}"#
        } else if r.url.ends_with("/api/v1/statuses") {
            r#"{"id":"777","url":"https://mastomini.local/@matt/777"}"#
        } else {
            r#"{"id":"1"}"#
        };
        Ok(Fetched {
            status: if r.url.ends_with("/smtp/email") {
                201
            } else {
                200
            },
            content_type: "application/json".into(),
            body: body.as_bytes().to_vec(),
        })
    }
}

/// An app whose outbox talks to `sink`, with the bot and email set up.
fn connected(sink: Arc<Sink>) -> (App, Arc<Outbox>) {
    let mut app = app(None);
    let outbox = Arc::new(Outbox::open(Arc::new(MemKv::default()), sink.clone(), sink));
    outbox
        .begin_setup(Setup::Token {
            instance: "https://mastomini.local".into(),
            token: "tok".into(),
            anchor: "household-ca".into(),
        })
        .unwrap();
    outbox
        .begin_mail_setup(MailSetup {
            api_key: "xkeysib-0123456789abcdef".into(),
            ..Default::default()
        })
        .unwrap();
    assert!(outbox.run_once(1) && outbox.run_once(1));
    app.outbox = Some(outbox.clone());
    (app, outbox)
}

fn drain(outbox: &Outbox) {
    while outbox.run_once(u64::MAX / 2) {}
}

use meritledger::outbound::StoredLink;

/// Dad (person 1) links his own account; its token is "dadtok".
fn link_dad(outbox: &Outbox) {
    outbox
        .save_link(
            1,
            &StoredLink {
                provider: "mastomini".into(),
                instance: "https://mastomini.local".into(),
                subject: "9".into(),
                handle: "@dad@mastomini.local".into(),
                token: "dadtok".into(),
                anchor: "household-ca".into(),
            },
        )
        .unwrap();
}

fn sent(sink: &Sink) -> Vec<(String, String)> {
    sink.sent.lock().unwrap().clone()
}

#[test]
fn sharing_posts_as_the_person_and_follows_edits_and_deletes() {
    let sink = Arc::new(Sink::default());
    let (app, outbox) = connected(sink.clone());
    let (dad, kid) = household(&app);
    link_dad(&outbox);
    ok(
        call(
            &app,
            "POST",
            "/api/v1/entries",
            Some(&dad),
            r#"{"person":0,"kind":"meditation","amount":30,"note":"Quiet morning"}"#,
        ),
        201,
    );
    // Not someone else's.
    assert_eq!(
        call(&app, "POST", "/api/v1/entries/1/publish", Some(&kid), "").status,
        403
    );
    let r = ok(
        call(&app, "POST", "/api/v1/entries/1/publish", Some(&dad), ""),
        202,
    );
    assert!(
        r.contains(r#""state":"queued""#) && r.contains("@dad@mastomini.local"),
        "{r}"
    );
    let again = ok(
        call(&app, "POST", "/api/v1/entries/1/publish", Some(&dad), ""),
        202,
    );
    assert!(again.contains(r#""state":"already""#), "{again}");
    drain(&outbox);
    let posts: Vec<_> = sent(&sink)
        .into_iter()
        .filter(|(u, _)| u == "POST https://mastomini.local/api/v1/statuses")
        .collect();
    assert_eq!(posts.len(), 1);
    assert!(posts[0].1.contains("Meditation · 30 min"), "{}", posts[0].1);
    assert!(posts[0].1.contains("Quiet morning"));
    assert!(
        posts[0].1.contains("https://meritledger.local/p/1"),
        "a household instance gets the link"
    );
    assert!(posts[0].1.contains(r#""visibility":"unlisted""#));
    assert_eq!(outbox.published("e1").unwrap().status, "777");
    // An edit follows; so does a delete.
    ok(
        call(
            &app,
            "PATCH",
            "/api/v1/entries/1",
            Some(&dad),
            r#"{"rev":1,"note":"Quiet, long morning"}"#,
        ),
        200,
    );
    drain(&outbox);
    assert!(sent(&sink).iter().any(|(u, b)| u
        == "PUT https://mastomini.local/api/v1/statuses/777"
        && b.contains("Quiet, long morning")));
    ok(
        call(&app, "DELETE", "/api/v1/entries/1", Some(&dad), ""),
        200,
    );
    drain(&outbox);
    assert!(sent(&sink)
        .iter()
        .any(|(u, _)| u == "DELETE https://mastomini.local/api/v1/statuses/777"));
    assert!(outbox.published("e1").is_none());
}

#[test]
fn nothing_private_or_sensitive_leaves() {
    let sink = Arc::new(Sink::default());
    let (app, outbox) = connected(sink.clone());
    let (dad, _) = household(&app);
    link_dad(&outbox);
    ok(
        call(
            &app,
            "POST",
            "/api/v1/entries",
            Some(&dad),
            r#"{"person":0,"kind":"meditation","visibility":"private"}"#,
        ),
        201,
    );
    ok(
        call(
            &app,
            "POST",
            "/api/v1/entries",
            Some(&dad),
            r#"{"person":0,"kind":"anger"}"#,
        ),
        201,
    );
    ok(
        call(
            &app,
            "POST",
            "/api/v1/entries",
            Some(&dad),
            r#"{"person":0,"kind":"repentance","visibility":"public"}"#,
        ),
        201,
    );
    for id in 1..=3 {
        let r = call(
            &app,
            "POST",
            &format!("/api/v1/entries/{id}/publish"),
            Some(&dad),
            "",
        );
        assert_eq!(r.status, 409, "{id}: {}", r.body);
    }
    // Making a published entry private takes it down.
    ok(
        call(
            &app,
            "POST",
            "/api/v1/entries",
            Some(&dad),
            r#"{"person":0,"kind":"chanting"}"#,
        ),
        201,
    );
    ok(
        call(&app, "POST", "/api/v1/entries/4/publish", Some(&dad), ""),
        202,
    );
    drain(&outbox);
    ok(
        call(
            &app,
            "PATCH",
            "/api/v1/entries/4",
            Some(&dad),
            r#"{"rev":1,"visibility":"private"}"#,
        ),
        200,
    );
    drain(&outbox);
    assert!(sent(&sink)
        .iter()
        .any(|(u, _)| u == "DELETE https://mastomini.local/api/v1/statuses/777"));
}

#[test]
fn auto_publish_and_the_bot_for_the_unlinked() {
    let sink = Arc::new(Sink::default());
    let (app, outbox) = connected(sink.clone());
    let (dad, kid) = household(&app);
    // Kid has no link: the household bot posts, saying whose it is.
    ok(
        call(
            &app,
            "PATCH",
            "/api/v1/me",
            Some(&kid),
            r#"{"auto_publish":true,"publish_visibility":"public"}"#,
        ),
        200,
    );
    ok(
        call(
            &app,
            "POST",
            "/api/v1/entries",
            Some(&kid),
            r#"{"person":0,"kind":"daimoku","amount":108}"#,
        ),
        201,
    );
    drain(&outbox);
    let posts: Vec<_> = sent(&sink)
        .into_iter()
        .filter(|(u, _)| u.ends_with("/api/v1/statuses"))
        .collect();
    assert_eq!(posts.len(), 1, "{posts:?}");
    assert!(
        posts[0].1.contains("Kid: 📿 Daimoku · 108×"),
        "{}",
        posts[0].1
    );
    assert!(posts[0].1.contains(r#""visibility":"public""#));
    // Dad didn't switch it on: nothing of his goes out by itself.
    ok(
        call(
            &app,
            "POST",
            "/api/v1/entries",
            Some(&dad),
            r#"{"person":0,"kind":"meditation"}"#,
        ),
        201,
    );
    drain(&outbox);
    assert_eq!(
        sent(&sink)
            .iter()
            .filter(|(u, _)| u.ends_with("/api/v1/statuses"))
            .count(),
        1
    );
    assert_eq!(
        call(
            &app,
            "PATCH",
            "/api/v1/me",
            Some(&dad),
            r#"{"publish_visibility":"direct"}"#
        )
        .status,
        400
    );
}

#[test]
fn unlinking_revokes_and_stops_posting_as_them() {
    let sink = Arc::new(Sink::default());
    let (app, outbox) = connected(sink);
    let (dad, _) = household(&app);
    link_dad(&outbox);
    // The board's state didn't record a link (the test linked the Kv only).
    assert_eq!(
        call(&app, "DELETE", "/api/v1/me/mastodon", Some(&dad), "").status,
        404
    );
    let providers = ok(call(&app, "GET", "/api/v1/auth/providers", None, ""), 200);
    assert!(
        providers.contains(r#""providers":[]"#),
        "no OAuth in this test app: {providers}"
    );
}
