//! Sprint 10 end to end: calls to service, the bell, events with guests
//! answering by link, reminders, and DMs and email reaching a fake
//! mastomini and Brevo through the outbox.
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
            r.url.to_string(),
            String::from_utf8_lossy(r.body).into_owned(),
        ));
        let body = if r.url.ends_with("/verify_credentials") {
            r#"{"id":"9","username":"meritledger"}"#
        } else if r.url.ends_with("/senders") {
            r#"{"senders":[{"name":"Matt","email":"matt@example.com","active":true}]}"#
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

#[test]
fn a_call_to_service_reaches_the_bell_and_mastodon() {
    let sink = Arc::new(Sink::default());
    let (app, outbox) = connected(sink.clone());
    let (dad, kid) = household(&app);
    ok(
        call(
            &app,
            "PATCH",
            "/api/v1/me",
            Some(&kid),
            r#"{"mastodon":"kid@mastomini.local"}"#,
        ),
        200,
    );
    let bad = call(
        &app,
        "PATCH",
        "/api/v1/me",
        Some(&kid),
        r#"{"mastodon":"not a handle"}"#,
    );
    assert_eq!(bad.status, 400);
    let calls = ok(
        call(
            &app,
            "POST",
            "/api/v1/call",
            Some(&dad),
            r#"{"message":"Come and chant","when":"at 7","to":[]}"#,
        ),
        201,
    );
    assert!(calls.contains("Come and chant"), "{calls}");
    let summary = ok(call(&app, "GET", "/api/v1/me/summary", Some(&kid), ""), 200);
    assert!(summary.contains(r#""unread":1"#), "{summary}");
    let bell = ok(
        call(&app, "GET", "/api/v1/me/notifications", Some(&kid), ""),
        200,
    );
    assert!(bell.contains("Dad: Come and chant (at 7)"), "{bell}");
    drain(&outbox);
    let sent = sink.sent.lock().unwrap();
    let (url, body) = sent.last().unwrap();
    assert_eq!(url, "https://mastomini.local/api/v1/statuses");
    assert!(
        body.contains("@kid@mastomini.local Dad: Come and chant (at 7)"),
        "{body}"
    );
    assert!(body.contains(r#""visibility":"direct""#), "{body}");
    drop(sent);
    let read = ok(
        call(
            &app,
            "POST",
            "/api/v1/me/notifications/read",
            Some(&kid),
            r#"{"up_to":0}"#,
        ),
        200,
    );
    assert!(read.contains(r#""unread":0"#), "{read}");
}

#[test]
fn a_full_outbox_answers_409() {
    let sink = Arc::new(Sink::default());
    let (app, outbox) = connected(sink);
    let (dad, kid) = household(&app);
    ok(
        call(
            &app,
            "PATCH",
            "/api/v1/me",
            Some(&kid),
            r#"{"mastodon":"@kid@mastomini.local"}"#,
        ),
        200,
    );
    let mut status = 0;
    for _ in 0..=meritledger::outbound::QUEUE {
        status = call(
            &app,
            "POST",
            "/api/v1/call",
            Some(&dad),
            r#"{"message":"","when":"","to":[]}"#,
        )
        .status;
        if status != 201 {
            break;
        }
    }
    assert_eq!(status, 409);
    assert_eq!(outbox.room(), 0);
}

#[test]
fn an_event_with_a_member_and_a_guest() {
    let app = app(None);
    let (dad, kid) = household(&app);
    let start = 4_000_000_000u64;
    let input = format!(
        r#"{{"title":"Ullambana feast","start":{start},"end":0,"place":"Home","notes":"Vegetarian potluck","invitees":[2],"rev":0}}"#
    );
    let body = ok(
        call(&app, "POST", "/api/v1/events", Some(&dad), &input),
        201,
    );
    assert!(body.contains(r#""id":1"#), "{body}");
    let bell = ok(
        call(&app, "GET", "/api/v1/me/notifications", Some(&kid), ""),
        200,
    );
    assert!(
        bell.contains("Dad invited you to Ullambana feast"),
        "{bell}"
    );
    let body = ok(
        call(
            &app,
            "POST",
            "/api/v1/events/1/rsvp",
            Some(&kid),
            r#"{"answer":"yes","bringing":"Mooncakes","token":""}"#,
        ),
        200,
    );
    assert!(body.contains("Mooncakes"), "{body}");
    // A guest by link: the host adds them and shares the link.
    let body = ok(
        call(
            &app,
            "POST",
            "/api/v1/events/1/guests",
            Some(&dad),
            r#"{"name":"Auntie Lin"}"#,
        ),
        201,
    );
    let at = body.find("/e/1?t=").expect("the host sees the link");
    let token: String = body[at + 7..].chars().take_while(|c| *c != '"').collect();
    let kid_view = ok(call(&app, "GET", "/api/v1/events/1", Some(&kid), ""), 200);
    assert!(!kid_view.contains("?t="), "only the host sees guest links");
    let guest = ok(
        call(
            &app,
            "GET",
            &format!("/api/v1/events/1/guest?t={token}"),
            None,
            "",
        ),
        200,
    );
    assert!(guest.contains("Ullambana feast"), "{guest}");
    assert_eq!(
        call(&app, "GET", "/api/v1/events/1/guest?t=wrong", None, "").status,
        404
    );
    let answer = format!(r#"{{"answer":"yes","bringing":"Tea","token":"{token}"}}"#);
    let body = ok(
        call(&app, "POST", "/api/v1/events/1/guest-rsvp", None, &answer),
        200,
    );
    assert!(body.contains(r#""my_answer":"yes""#), "{body}");
    // The calendar file: for its people, or with the guest's token.
    let ics = call(&app, "GET", "/cal/event/1.ics", Some(&kid), "");
    assert_eq!(ics.status, 200);
    assert!(ics.body.contains("SUMMARY:Ullambana feast"), "{}", ics.body);
    assert_eq!(call(&app, "GET", "/cal/event/1.ics", None, "").status, 404);
    assert_eq!(
        call(
            &app,
            "GET",
            &format!("/cal/event/1.ics?t={token}"),
            None,
            ""
        )
        .status,
        200
    );
    // Two answers coalesce in Dad's bell.
    let bell = ok(
        call(&app, "GET", "/api/v1/me/notifications", Some(&dad), ""),
        200,
    );
    assert!(bell.contains("2 answers to Ullambana feast"), "{bell}");
}

#[test]
fn reminders_are_emailed_at_their_time() {
    let sink = Arc::new(Sink::default());
    let (app, outbox) = connected(sink.clone());
    let (_, kid) = household(&app);
    ok(
        call(
            &app,
            "PATCH",
            "/api/v1/me",
            Some(&kid),
            r#"{"email":"kid@home.example"}"#,
        ),
        200,
    );
    let list = ok(
        call(
            &app,
            "POST",
            "/api/v1/me/reminders",
            Some(&kid),
            r#"{"kind":"daily","time":"07:00","days":0,"before":0,"observances":"","text":"Sit for twenty minutes","off":false}"#,
        ),
        201,
    );
    assert!(list.contains("kid@home.example"), "{list}");
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/me/reminders",
            Some(&kid),
            r#"{"kind":"daily","time":"7am"}"#
        )
        .status,
        400
    );
    // Tomorrow at 07:00 New York time, and a minute later (once only).
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let tz = meritledger::tz::Tz::parse(meritledger::state::DEFAULT_TZ).unwrap();
    let tomorrow = tz.local_date(now as i64).add_days(1);
    let seven = meritledger::notify::local_instant(&tz, tomorrow, 7 * 60);
    app.run_schedules_at(seven + 30);
    app.run_schedules_at(seven + 90);
    drain(&outbox);
    let sent = sink.sent.lock().unwrap();
    let emails: Vec<_> = sent
        .iter()
        .filter(|(u, _)| u.ends_with("/smtp/email"))
        .collect();
    assert_eq!(emails.len(), 1, "{sent:?}");
    assert!(
        emails[0]
            .1
            .contains(r#""subject":"Reminder: Sit for twenty minutes""#),
        "{}",
        emails[0].1
    );
    assert!(emails[0].1.contains("kid@home.example"));
}

#[test]
fn only_admins_see_delivery() {
    let sink = Arc::new(Sink::default());
    let (app, _) = connected(sink);
    let (dad, kid) = household(&app);
    assert_eq!(
        call(&app, "GET", "/api/v1/admin/delivery", Some(&kid), "").status,
        403
    );
    let view = ok(
        call(&app, "GET", "/api/v1/admin/delivery", Some(&dad), ""),
        200,
    );
    assert!(view.contains("@meritledger@mastomini.local"), "{view}");
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/admin/delivery/test",
            Some(&dad),
            r#"{"kind":"dm"}"#
        )
        .status,
        400,
        "no handle yet"
    );
}

/// Sprint 13: the board's touch menu records, undoes and calls, as "the board".
#[test]
fn the_touch_menu_records_undoes_and_calls() {
    let app = app(None);
    let (dad, kid) = household(&app);
    let people = app.board_people();
    assert_eq!(
        people.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        ["Dad", "Kid"]
    );

    let entry = app.record_from_board(2, "meditation").unwrap();
    let mine = ok(call(&app, "GET", "/api/v1/today", Some(&kid), ""), 200);
    assert!(!mine.contains("\"entries\":[]"), "{mine}");
    app.undo_from_board(entry).unwrap();
    let mine = ok(call(&app, "GET", "/api/v1/today", Some(&kid), ""), 200);
    assert!(mine.contains("\"entries\":[]"), "{mine}");
    // Only the board's own entries: not one Kid typed in.
    let typed = ok(
        call(
            &app,
            "POST",
            "/api/v1/entries",
            Some(&kid),
            r#"{"person":2,"kind":"chanting"}"#,
        ),
        201,
    );
    let id: u32 = typed
        .split("\"id\":")
        .nth(1)
        .unwrap()
        .split([',', '}'])
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(app.undo_from_board(id).unwrap_err().status, 409);

    app.call_from_board().unwrap();
    let bell = ok(
        call(&app, "GET", "/api/v1/me/notifications", Some(&dad), ""),
        200,
    );
    assert!(
        bell.contains("Someone at the board calls everyone to practice"),
        "{bell}"
    );

    // The admin page can't overwrite the board's last touch.
    app.note_touch((10, 20), (300, 10));
    let put = ok(
        call(
            &app,
            "PUT",
            "/api/v1/admin/screen",
            Some(&dad),
            r#"{"flip":false,"native":false,"brightness":30,"touch_flip_x":true,"touch_flip_y":false,"last_touch":"x"}"#,
        ),
        200,
    );
    assert!(put.contains("raw 10,20"), "{put}");
    assert!(put.contains("\"touch_flip_x\":true"), "{put}");
}
