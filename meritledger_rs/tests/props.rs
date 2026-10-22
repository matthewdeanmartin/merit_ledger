//! Property tests (proptest, like Python's Hypothesis): instead of a few
//! hand-picked examples, each test states something that must hold for
//! *every* input and lets the generator hunt for a counterexample, shrinking
//! any it finds to the smallest one. They cover the code that takes input
//! from outside (requests, time zones, touches, any text on the screen) and
//! the arithmetic that's easy to get wrong at the edges (dates, the lunar
//! calendar, folding lines).
//!
//! `PROPTEST_CASES=10000 cargo test --test props` searches harder.
use meritledger::calendar::{chinese_day, chinese_months};
use meritledger::screen::canvas::{H, W};
use meritledger::screen::text::{squeeze, squeeze_line, BODY, BOLD, MID, SMALL, TITLE};
use meritledger::screen::ui::{self, Action, Gesture, Person, Touch, Ui};
use meritledger::screen::{Card, Context, Frame, STRIP};
use meritledger::sd::Sd;
use meritledger::tz::{Date, Tz};
use meritledger::{App, Limits};
use miniframework::kv::MemKv;
use miniframework::{Reply, Request, Service};
use proptest::prelude::*;
use proptest::test_runner::FileFailurePersistence;
use std::sync::Arc;

/// `PROPTEST_CASES` (when set) overrides each test's own count, so one
/// variable makes the whole suite search harder. Failures found are saved
/// beside this file (`props.proptest-regressions`, like Hypothesis's
/// example database) and tried first on every later run: commit it.
fn cases(default: u32) -> ProptestConfig {
    let n = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok());
    ProptestConfig {
        cases: n.unwrap_or(default),
        failure_persistence: Some(Box::new(FileFailurePersistence::WithSource(
            "proptest-regressions",
        ))),
        ..ProptestConfig::default()
    }
}

// ---------------------------------------------------------------- dates

fn date() -> impl Strategy<Value = Date> {
    // 1900..2200 by day number, so every date is valid by construction.
    let lo = Date::parse("1900-01-01").unwrap().days();
    let hi = Date::parse("2200-12-31").unwrap().days();
    (lo..=hi).prop_map(Date::from_days)
}

proptest! {
    #![proptest_config(cases(256))]

    #[test]
    fn dates_round_trip(d in date()) {
        prop_assert_eq!(Date::from_days(d.days()), d);
        prop_assert_eq!(Date::parse(&d.iso()), Some(d));
        prop_assert!(d.month >= 1 && d.month <= 12);
        prop_assert!(d.day >= 1 && d.day <= meritledger::tz::days_in_month(d.year, d.month));
    }

    #[test]
    fn adding_days_moves_the_weekday(d in date(), n in -5000i64..5000) {
        let later = d.add_days(n);
        prop_assert_eq!(later.days() - d.days(), n);
        prop_assert_eq!(i64::from(later.weekday()), (i64::from(d.weekday()) + n).rem_euclid(7));
    }

    #[test]
    fn date_parsing_never_panics(s in "\\PC{0,16}|[0-9]{1,5}-[0-9]{1,3}-[0-9]{1,3}") {
        if let Some(d) = Date::parse(&s) {
            prop_assert_eq!(Date::parse(&d.iso()), Some(d));
        }
    }

    // Anything typed into the time zone box: never a panic, and a zone that
    // parses keeps its offsets within a day.
    #[test]
    fn time_zones_parse_safely(
        spec in "\\PC{0,40}|[A-Z]{3,5}-?[0-9]{1,2}(:[0-9]{2})?([A-Z]{3,5}(,M[0-9]{1,2}\\.[0-9]\\.[0-9](/[0-9]{1,2})?){0,2})?",
        utc in -2_000_000_000i64..5_000_000_000,
    ) {
        if let Ok(tz) = Tz::parse(&spec) {
            prop_assert!(tz.offset_at(utc).abs() <= 25 * 3600, "{} at {}", spec, utc);
            let _ = tz.local_date(utc);
        }
    }
}

// ---------------------------------------------------------------- lunar calendar

proptest! {
    #![proptest_config(cases(48))]

    // Months tile the range: each starts the day after the last ended, has
    // 29 or 30 days, is numbered 1-12, and every day lands in exactly one.
    #[test]
    fn lunar_months_tile_the_days(start in date().prop_filter("2000-2099", |d| d.year >= 2000 && d.year < 2100), span in 0i64..120) {
        let tz = Tz::parse("CST-8").unwrap();
        let end = start.add_days(span);
        let months = chinese_months(&tz, start, end);
        prop_assert!(!months.is_empty());
        for pair in months.windows(2) {
            let length = pair[1].start.days() - pair[0].start.days();
            prop_assert!(length == 29 || length == 30, "{:?} -> {:?}", pair[0], pair[1]);
        }
        for m in &months {
            prop_assert!((1..=12).contains(&m.number), "{:?}", m);
        }
        for n in 0..=span {
            let d = start.add_days(n);
            let (_, _, day) = chinese_day(&months, d).expect("every day is in a month");
            prop_assert!((1..=30).contains(&day));
        }
    }
}

// ---------------------------------------------------------------- the screen

use meritledger::screen::text::Face;

/// Some of the faces, largest first (by index, so failures print).
fn faces() -> impl Strategy<Value = Vec<usize>> {
    proptest::sample::subsequence(vec![0, 1, 2, 3, 4], 1..=5)
}

fn pick(faces: &[usize]) -> Vec<&'static Face> {
    let all: [&'static Face; 5] = [&TITLE, &MID, &BOLD, &BODY, &SMALL];
    faces.iter().map(|&i| all[i]).collect()
}

/// Words people actually type, plus anything at all.
fn words() -> impl Strategy<Value = String> {
    prop_oneof![
        "[A-Za-z ,.'!?]{0,120}",
        "\\PC{0,80}",
        "[A-Za-z]{20,60}( [A-Za-z]{1,8}){0,4}",
        Just("南無妙法蓮華經 Namu-myōhō-renge-kyō 🙏".to_string()),
    ]
}

proptest! {
    // Fitted text never runs past its box.
    #[test]
    fn squeezed_text_fits(text in words(), max_w in 40i32..=320, max_h in 0i32..=172, faces in faces()) {
        let fitted = squeeze(&text, max_w, max_h, &pick(&faces));
        let lines = (max_h / fitted.face.line as i32).max(1) as usize;
        prop_assert!(fitted.lines.len() <= lines, "{} lines in {} px", fitted.lines.len(), max_h);
        for l in &fitted.lines {
            prop_assert!(fitted.face.width(l) <= max_w, "{:?} is {} px > {}", l, fitted.face.width(l), max_w);
        }
    }

    #[test]
    fn squeezed_lines_fit(text in words(), max_w in 40i32..=320, max_lines in 0usize..4, faces in faces()) {
        let fitted = squeeze_line(&text, max_w, max_lines, &pick(&faces));
        for l in &fitted.lines {
            prop_assert!(fitted.face.width(l) <= max_w, "{:?}", l);
        }
    }

    #[test]
    fn wrapping_takes_any_line_count(text in words(), max in 1i32..=320, lines in 0usize..5) {
        let out = BODY.wrap(&text, max, lines);
        prop_assert!(out.len() <= lines.max(1));
    }
}

fn card() -> impl Strategy<Value = Card> {
    let s = words;
    prop_oneof![
        (
            s(),
            s(),
            s(),
            s(),
            s(),
            any::<bool>(),
            any::<u32>(),
            any::<u32>()
        )
            .prop_map(
                |(time, weekday, date, lunar, observance, today, people, minutes)| Card::Today {
                    time,
                    weekday,
                    date,
                    lunar,
                    observance,
                    today,
                    people,
                    minutes
                }
            ),
        (s(), s(), s(), any::<u64>()).prop_map(|(by, message, when, minutes_ago)| Card::Call {
            by,
            message,
            when,
            minutes_ago
        }),
        (s(), s()).prop_map(|(text, source)| Card::Verse { text, source }),
        (s(), proptest::collection::vec(s(), 0..12), 0usize..6).prop_map(
            |(title, labels, cols)| Card::Buttons {
                title,
                labels,
                cols
            }
        ),
        (s(), s(), s()).prop_map(|(title, body, button)| Card::Message {
            title,
            body,
            button
        }),
    ]
}

proptest! {
    #![proptest_config(cases(64))]

    // Any card with any text draws, strip by strip, without a panic, and
    // covers the whole screen exactly once.
    #[test]
    fn any_card_draws(card in card(), page in 0usize..9, pages in 0usize..9) {
        let frame = Frame::new(card, Context { name: "meritledger.local".into(), page, pages });
        let mut buf = vec![0u16; (W * STRIP) as usize];
        let mut next = 0;
        frame.draw(&mut buf, |top, rows, _| {
            assert_eq!(top, next);
            next += rows;
            Ok::<(), ()>(())
        }).unwrap();
        prop_assert_eq!(next, H);
    }
}

// ---------------------------------------------------------------- the touch menu

proptest! {
    #![proptest_config(cases(256))]

    #[test]
    fn buttons_stay_on_screen_and_apart(n in 0usize..12, cols in 0usize..6) {
        let cells = ui::grid(n, cols);
        prop_assert!(cells.len() <= n);
        for (i, r) in cells.iter().enumerate() {
            prop_assert!(r.x >= 0 && r.y >= ui::HEADER && r.x + r.w <= W && r.y + r.h <= H, "{:?}", r);
            prop_assert!(r.h >= 44, "too short to tap: {:?}", r);
            for other in &cells[i + 1..] {
                let apart = r.x + r.w <= other.x || other.x + other.w <= r.x
                    || r.y + r.h <= other.y || other.y + other.h <= r.y;
                prop_assert!(apart, "{:?} overlaps {:?}", r, other);
            }
        }
    }

    #[test]
    fn touches_map_onto_the_screen(x in -50i32..500, y in -50i32..500, fx in any::<bool>(), fy in any::<bool>()) {
        let (sx, sy) = ui::landscape(x, y, fx, fy);
        prop_assert!((0..W).contains(&sx) && (0..H).contains(&sy));
    }
}

#[derive(Clone, Debug)]
enum Event {
    /// A finger at a point (or lifted), `dt` ms after the last event.
    Sample(Option<(i32, i32)>, u64),
    Tick(u64),
}

fn events() -> impl Strategy<Value = Vec<Event>> {
    let point = (0i32..W, 0i32..H);
    proptest::collection::vec(
        prop_oneof![
            (proptest::option::of(point), 0u64..1500).prop_map(|(p, dt)| Event::Sample(p, dt)),
            (0u64..40_000).prop_map(Event::Tick),
        ],
        0..120,
    )
}

proptest! {
    // Whatever anyone pokes at the board: no panic, the view always has a
    // card, and every action names someone on the list, a kind the menu
    // offers, or an entry the board just recorded.
    #[test]
    fn the_touch_menu_survives_anything(events in events(), n_people in 0usize..14, cards in 1usize..4) {
        let people: Vec<Person> = (0..n_people).map(|i| Person { id: i as u32 + 1, name: format!("P{i}") }).collect();
        let shown: Vec<Card> = (0..cards).map(|i| Card::Verse { text: format!("v{i}"), source: String::new() }).collect();
        let (mut ui, mut touch, mut now) = (Ui::default(), Touch::default(), 0u64);
        let mut recorded = Vec::new();
        let mut next_entry = 100;
        for e in events {
            match e {
                Event::Tick(dt) => {
                    now += dt;
                    ui.tick(now, cards);
                }
                Event::Sample(at, dt) => {
                    now += dt;
                    let Some(g) = touch.sample(at, now) else { continue };
                    if let Gesture::Tap(x, y) = g {
                        prop_assert!((0..W).contains(&x) && (0..H).contains(&y));
                    }
                    match ui.gesture(g, &people, cards, now) {
                        Some(Action::Record { person, kind }) => {
                            prop_assert!(people.iter().any(|p| p.id == person));
                            prop_assert!(ui::KINDS.iter().any(|(k, _)| *k == kind));
                            next_entry += 1;
                            recorded.push(next_entry);
                            ui.recorded(next_entry, "done".into(), now);
                        }
                        Some(Action::Undo { entry }) => prop_assert!(recorded.contains(&entry)),
                        Some(Action::Call) | None => {}
                    }
                }
            }
            let (_, page, pages) = ui.view(&shown, &people);
            prop_assert!(pages == 0 || page < pages, "page {} of {}", page, pages);
        }
    }
}

// ---------------------------------------------------------------- the API

fn open(kv: Arc<MemKv>) -> App {
    let limits = Limits {
        pin_rounds: 2,
        ..Limits::board()
    };
    App::open(kv, Arc::new(Sd::none()), limits).unwrap()
}

fn call(
    app: &App,
    method: &str,
    path: &str,
    cookie: Option<&str>,
    body: &[u8],
) -> (u16, String, Option<String>) {
    let mut headers = vec![
        ("Host".to_string(), "meritledger.local".to_string()),
        ("Content-Type".to_string(), "application/json".to_string()),
        ("Origin".to_string(), "http://meritledger.local".to_string()),
    ];
    if let Some(c) = cookie {
        headers.push(("Cookie".into(), c.into()));
    }
    let req = Request::new(method, path, &headers, body, false);
    let (mut out, mut spare) = (Vec::new(), Vec::new());
    let mut reply = Reply::new(&mut out, &mut spare, 256 * 1024);
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
    (reply.status(), body, cookie)
}

/// An admin, signed in, so the fuzzing reaches past the sign-in checks.
fn signed_in(app: &App) -> String {
    call(
        app,
        "POST",
        "/api/v1/setup?fmt=json",
        None,
        br#"{"display_name":"Dad","pin":"1234","setup_code":""}"#,
    )
    .2
    .unwrap()
}

const PATHS: &[&str] = &[
    "/api/v1/entries",
    "/api/v1/entries/1",
    "/api/v1/entries/1/rejoice",
    "/api/v1/entries/1/publish",
    "/api/v1/today",
    "/api/v1/feed",
    "/api/v1/kinds",
    "/api/v1/calendar",
    "/api/v1/timers",
    "/api/v1/texts",
    "/api/v1/practices",
    "/api/v1/practices/1",
    "/api/v1/precepts",
    "/api/v1/vows",
    "/api/v1/vows/1",
    "/api/v1/calls",
    "/api/v1/events",
    "/api/v1/events/1",
    "/api/v1/events/1/rsvp",
    "/api/v1/reading",
    "/api/v1/reading/1",
    "/api/v1/reading/1/log",
    "/api/v1/shelf",
    "/api/v1/people",
    "/api/v1/me",
    "/api/v1/me/summary",
    "/api/v1/me/notifications",
    "/api/v1/me/reminders",
    "/api/v1/admin/people",
    "/api/v1/admin/people/2",
    "/api/v1/admin/settings",
    "/api/v1/admin/screen",
    "/api/v1/admin/import",
    "/api/v1/display/frame.bmp",
    "/api/v1/status",
    "/cal/chinese.ics",
    "/cal/theravada.ics",
    "/api/v1/call",
    "/api/v1/kinds",
    "/api/v1/me/pins",
    "/api/v1/me/pin",
    "/api/v1/auth/pin",
    "/api/v1/auth/signup",
    "/api/v1/me/notifications/read",
    "/api/v1/admin/backups",
    "/api/v1/auth/guest",
    "/api/v1/auth/logout",
];

/// JSON that's nearly right: the shapes the API expects, with wrong types,
/// huge numbers, odd strings and missing fields mixed in.
fn json() -> impl Strategy<Value = String> {
    let leaf = prop_oneof![
        Just("null".to_string()),
        any::<bool>().prop_map(|b| b.to_string()),
        any::<i64>().prop_map(|n| n.to_string()),
        Just("1e400".to_string()),
        Just("-0".to_string()),
        "\\PC{0,30}".prop_map(|s| serde_json_escape(&s)),
    ];
    let keys = prop_oneof![
        Just("person"),
        Just("kind"),
        Just("amount"),
        Just("minutes"),
        Just("date"),
        Just("when"),
        Just("title"),
        Just("display_name"),
        Just("pin"),
        Just("role"),
        Just("visibility"),
        Just("brightness"),
        Just("page"),
        Just("pages"),
        Just("status"),
        Just("work"),
        Just("to"),
        Just("message"),
        Just("steps"),
        Just("elements"),
        Just("tz"),
        Just("answer"),
        Just("id"),
    ];
    let value = leaf.prop_recursive(3, 24, 6, |inner| {
        prop_oneof![
            proptest::collection::vec(inner.clone(), 0..5)
                .prop_map(|v| format!("[{}]", v.join(","))),
            proptest::collection::vec(("[a-z_]{1,10}", inner), 0..5).prop_map(|kv| {
                format!(
                    "{{{}}}",
                    kv.iter()
                        .map(|(k, v)| format!("\"{k}\":{v}"))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            }),
        ]
    });
    prop_oneof![
        proptest::collection::vec((keys, value.clone()), 0..8).prop_map(|kv| {
            format!(
                "{{{}}}",
                kv.iter()
                    .map(|(k, v)| format!("\"{k}\":{v}"))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }),
        value,
        "\\PC{0,64}",
    ]
}

fn serde_json_escape(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn request() -> impl Strategy<Value = (String, String, String)> {
    (
        prop_oneof![
            Just("GET"),
            Just("POST"),
            Just("PUT"),
            Just("PATCH"),
            Just("DELETE")
        ]
        .prop_map(String::from),
        prop_oneof![
            proptest::sample::select(PATHS).prop_map(String::from),
            (
                proptest::sample::select(PATHS),
                "[0-9]{1,12}|-1|%00|\\.\\.|[a-z]{1,8}"
            )
                .prop_map(|(p, tail)| format!("{p}/{tail}")),
            (proptest::sample::select(PATHS), "[a-z]{1,6}=[^&# ]{0,12}")
                .prop_map(|(p, q)| format!("{p}?{q}")),
        ],
        json(),
    )
}

proptest! {
    #![proptest_config(cases(512))]

    // Whatever arrives (any route, any method, any body, signed in as an
    // admin or not): an answer, never a panic or a 500, and the state
    // stays loadable: the board reopens it from flash.
    #[test]
    fn the_api_answers_anything(requests in proptest::collection::vec((request(), any::<bool>()), 1..12)) {
        let kv = Arc::new(MemKv::default());
        let app = open(kv.clone());
        let cookie = signed_in(&app);
        for ((method, path, body), admin) in requests {
            let sep = if path.contains('?') { '&' } else { '?' };
            let path = format!("{path}{sep}fmt=json");
            let (status, text, _) = call(&app, &method, &path, admin.then_some(cookie.as_str()), body.as_bytes());
            // 0 is "not an API route": the framework answers 404 from the
            // bundled files.
            prop_assert!(answered(status, &text), "{} {} {} -> {} {}", method, path, body, status, text);
        }
        same_after_reboot(app, kv)?;
    }
}

/// Signs in again as Dad (sessions don't survive a reboot; the ledger must).
fn pin(app: &App) -> String {
    call(
        app,
        "POST",
        "/api/v1/auth/pin?fmt=json",
        None,
        br#"{"person":1,"pin":"1234"}"#,
    )
    .2
    .unwrap()
}

/// What the household would see, before and after the board restarts.
fn same_after_reboot(app: App, kv: Arc<MemKv>) -> Result<(), TestCaseError> {
    const SEEN: &[&str] = &[
        "/api/v1/people",
        "/api/v1/feed",
        "/api/v1/reading",
        "/api/v1/events",
        "/api/v1/vows",
        "/api/v1/me/summary",
    ];
    let look = |app: &App| {
        let cookie = pin(app);
        SEEN.iter()
            .map(|p| call(app, "GET", &format!("{p}?fmt=json"), Some(&cookie), b"").1)
            .collect::<Vec<_>>()
    };
    let before = look(&app);
    drop(app);
    let after = look(&open(kv));
    for ((p, b), a) in SEEN.iter().zip(before).zip(after) {
        prop_assert_eq!(b, a, "{} changed across a reboot", p);
    }
    Ok(())
}

/// A string to put in a field: ordinary, empty, huge, or strange.
fn field() -> impl Strategy<Value = String> {
    prop_oneof![
        3 => "[A-Za-z ]{0,20}",
        1 => Just(String::new()),
        1 => "\\PC{0,40}",
        1 => Just("x".repeat(5000)),
        1 => Just("<script>alert(1)</script>".to_string()),
        1 => "(19|20|21)[0-9]{2}-[01][0-9]-[0-3][0-9]",
    ]
    .prop_map(|s| serde_json_escape(&s))
}

fn num() -> impl Strategy<Value = u64> {
    prop_oneof![
        0u64..10,
        0u64..100_000,
        Just(u64::from(u32::MAX)),
        Just(u64::MAX),
        1_700_000_000u64..1_900_000_000,
    ]
}

fn opt(n: Option<u64>) -> String {
    n.map_or("null".into(), |a| a.to_string())
}

/// Well-formed requests with hostile values: these get past decoding and
/// into the ledger's own checks.
fn operation() -> impl Strategy<Value = (&'static str, String, String)> {
    let kind = prop_oneof![
        Just("\"meditation\"".to_string()),
        Just("\"chanting\"".to_string()),
        Just("\"reading\"".to_string()),
        Just("\"precepts\"".to_string()),
        field(),
    ];
    let vis = prop_oneof![
        Just("\"public\"".to_string()),
        Just("\"household\"".to_string()),
        Just("\"private\"".to_string()),
        field(),
    ];
    let id = 0u32..6;
    prop_oneof![
        (0u32..4, kind, proptest::option::of(num()), field(), vis.clone(), proptest::option::of(field())).prop_map(
            |(person, kind, amount, note, vis, date)| (
                "POST",
                "/api/v1/entries".to_string(),
                format!(
                    r#"{{"person":{person},"kind":{kind},"amount":{},"note":{note},"visibility":{vis},"date":{},"client_id":""}}"#,
                    opt(amount),
                    date.unwrap_or("null".into())
                )
            )
        ),
        (id.clone(), proptest::option::of(num()), field()).prop_map(|(id, amount, note)| (
            "PATCH",
            format!("/api/v1/entries/{id}"),
            format!(r#"{{"rev":0,"amount":{},"note":{note}}}"#, opt(amount))
        )),
        id.clone().prop_map(|id| ("DELETE", format!("/api/v1/entries/{id}"), String::new())),
        id.clone().prop_map(|id| ("POST", format!("/api/v1/entries/{id}/rejoice"), String::new())),
        (field(), num(), num(), field(), proptest::collection::vec(0u32..5, 0..4)).prop_map(
            |(title, start, end, place, inv)| (
                "POST",
                "/api/v1/events".to_string(),
                format!(
                    r#"{{"title":{title},"start":{start},"end":{end},"place":{place},"notes":"","invitees":{inv:?},"rev":0}}"#
                )
            )
        ),
        (id.clone(), prop_oneof![Just("yes"), Just("no"), Just("maybe"), Just("")], field()).prop_map(
            |(id, a, b)| (
                "POST",
                format!("/api/v1/events/{id}/rsvp"),
                format!(r#"{{"answer":"{a}","bringing":{b},"token":""}}"#)
            )
        ),
        (
            field(),
            field(),
            num(),
            prop_oneof![Just("want"), Just("reading"), Just("done"), Just("lost")],
            vis.clone()
        )
            .prop_map(|(work, title, pages, st, vis)| (
                "POST",
                "/api/v1/reading".to_string(),
                format!(
                    r#"{{"work":{work},"title":{title},"author":"","cover":0,"pages":{pages},"year":0,"status":"{st}","visibility":{vis}}}"#
                )
            )),
        (id.clone(), num(), num()).prop_map(|(id, minutes, page)| (
            "POST",
            format!("/api/v1/reading/{id}/log"),
            format!(r#"{{"minutes":{minutes},"page":{page},"note":""}}"#)
        )),
        (id.clone(), proptest::option::of(num()), proptest::option::of(num())).prop_map(|(id, page, pages)| (
            "PATCH",
            format!("/api/v1/reading/{id}"),
            format!(r#"{{"rev":0,"page":{},"pages":{}}}"#, opt(page), opt(pages))
        )),
        id.clone().prop_map(|id| ("DELETE", format!("/api/v1/reading/{id}"), String::new())),
        (field(), field(), field(), vis).prop_map(|(text, started, ends, vis)| (
            "POST",
            "/api/v1/vows".to_string(),
            format!(
                r#"{{"text":{text},"started":{started},"ends":{ends},"visibility":{vis},"precept_set":"","precept_kind":"","precept_n":0}}"#
            )
        )),
        (field(), prop_oneof![Just("\"now\"".to_string()), field()]).prop_map(|(m, w)| (
            "POST",
            "/api/v1/call".to_string(),
            format!(r#"{{"message":{m},"when":{w},"to":[]}}"#)
        )),
        (field(), 0usize..4).prop_map(|(name, role)| (
            "POST",
            "/api/v1/admin/people".to_string(),
            format!(
                r#"{{"display_name":{name},"role":"{}","pin":"1111","email":""}}"#,
                ["member", "admin", "guest", "x"][role]
            )
        )),
        (num(), any::<bool>()).prop_map(|(b, f)| (
            "PUT",
            "/api/v1/admin/screen".to_string(),
            format!(
                r#"{{"flip":{f},"native":false,"brightness":{b},"touch_flip_x":{f},"touch_flip_y":false,"last_touch":""}}"#
            )
        )),
    ]
}

proptest! {
    #![proptest_config(cases(256))]

    // A household's worth of plausible-but-hostile use: never a 5xx, a
    // full table says 409 "full", and what everyone sees survives a reboot.
    #[test]
    fn the_ledger_holds_up_to_use(ops in proptest::collection::vec(operation(), 1..40)) {
        let kv = Arc::new(MemKv::default());
        let app = open(kv.clone());
        let cookie = signed_in(&app);
        for (method, path, body) in ops {
            let (status, text, _) = call(&app, method, &format!("{path}?fmt=json"), Some(&cookie), body.as_bytes());
            prop_assert!(status != 0 && answered(status, &text), "{} {} {} -> {} {}", method, path, body, status, text);
        }
        same_after_reboot(app, kv)?;
    }
}

// ---------------------------------------------------------------- household texts

/// Markdown-ish text with the characters and tricks that cause trouble.
fn markdownish() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        prop_oneof![
            "\\PC{0,12}",
            proptest::sample::select(vec![
                "# ",
                "- ",
                "* ",
                "**",
                "*",
                "_",
                "[",
                "](",
                ")",
                "\n",
                "\n\n",
                "\r\n",
                "<",
                ">",
                "\"",
                "'",
                "&",
                "<script>alert(1)</script>",
                "[x](javascript:alert(1))",
                "[x](https://e.com\" onmouseover=\"alert(1))",
                "<img src=x onerror=alert(1)>",
                "[a](https://ok.example/path)",
            ])
            .prop_map(String::from),
        ],
        0..40,
    )
    .prop_map(|parts| parts.concat())
}

/// Whether one tag is something `markdown::to_html` may write.
fn allowed_tag(tag: &str) -> bool {
    const PLAIN: &[&str] = &[
        "<p>",
        "</p>",
        "<br>",
        "<ul>",
        "</ul>",
        "<li>",
        "</li>",
        "<strong>",
        "</strong>",
        "<em>",
        "</em>",
        "</a>",
    ];
    if PLAIN.contains(&tag) {
        return true;
    }
    let Some(rest) = tag.strip_prefix("<a href=\"") else {
        return false;
    };
    let Some(url) = rest.strip_suffix("\" rel=\"nofollow noopener noreferrer\" target=\"_blank\">")
    else {
        return false;
    };
    (url.starts_with("https://") || url.starts_with("http://"))
        && !url
            .chars()
            .any(|c| matches!(c, '"' | '<' | '>' | '\'') || c.is_whitespace())
}

proptest! {
    #![proptest_config(cases(1024))]

    // Household texts are uploaded by people and shown to everyone: only
    // the tags the subset allows, and only http(s) links, ever.
    #[test]
    fn texts_render_safely(md in markdownish()) {
        let html = meritledger::markdown::to_html(&md);
        let mut rest = html.as_str();
        while let Some(start) = rest.find('<') {
            let end = rest[start..].find('>').map(|e| start + e + 1);
            prop_assert!(end.is_some(), "an unclosed '<' in {:?}", html);
            let tag = &rest[start..end.unwrap()];
            prop_assert!(allowed_tag(tag), "{:?} in {:?}", tag, html);
            rest = &rest[end.unwrap()..];
        }
    }
}

// ---------------------------------------------------------------- flash

/// A real snapshot: a household with people, entries, a book, an event.
fn lived_in() -> Vec<u8> {
    let kv = Arc::new(MemKv::default());
    let app = open(kv.clone());
    let cookie = signed_in(&app);
    for (method, path, body) in [
        (
            "POST",
            "/api/v1/admin/people",
            r#"{"display_name":"Kid","role":"member","pin":"1111","email":""}"#,
        ),
        (
            "POST",
            "/api/v1/entries",
            r#"{"person":1,"kind":"meditation","amount":20,"note":"","client_id":""}"#,
        ),
        (
            "POST",
            "/api/v1/entries",
            r#"{"person":2,"kind":"chanting","amount":15,"note":"hi","client_id":""}"#,
        ),
        (
            "POST",
            "/api/v1/reading",
            r#"{"work":"OL1W","title":"Lotus","author":"","cover":0,"pages":300,"year":0,"status":"reading","visibility":"public"}"#,
        ),
        (
            "POST",
            "/api/v1/events",
            r#"{"title":"Uposatha","start":1800000000,"end":0,"place":"","notes":"","invitees":[2],"rev":0}"#,
        ),
        (
            "POST",
            "/api/v1/vows",
            r#"{"text":"Kindness","started":"","ends":"","visibility":"household","precept_set":"","precept_kind":"","precept_n":0}"#,
        ),
    ] {
        call(
            &app,
            method,
            &format!("{path}?fmt=json"),
            Some(&cookie),
            body.as_bytes(),
        );
    }
    drop(app);
    let (_, loaded) = meritledger::store::SlotStore::open(kv, u16::MAX).unwrap();
    loaded.unwrap().payload
}

proptest! {
    #![proptest_config(cases(256))]

    // Flash goes bad: a snapshot whose checksum is fine but whose contents
    // are damaged (a bug, a half-migrated schema). The board refuses it or
    // boots, and if it boots, every page still answers.
    #[test]
    fn damaged_snapshots_never_crash_the_board(
        edits in proptest::collection::vec((any::<proptest::sample::Index>(), any::<u8>()), 1..4),
        cut in proptest::option::weighted(0.1, any::<proptest::sample::Index>()),
    ) {
        let mut payload = lived_in();
        for (at, byte) in edits {
            let i = at.index(payload.len());
            payload[i] = byte;
        }
        if let Some(at) = cut {
            payload.truncate(at.index(payload.len()));
        }
        let kv = Arc::new(MemKv::default());
        let (mut store, _) = meritledger::store::SlotStore::open(kv.clone(), u16::MAX).unwrap();
        store.commit(meritledger::state::SCHEMA, &payload).unwrap();
        let Ok(app) = App::open(kv, Arc::new(Sd::none()), Limits { pin_rounds: 2, ..Limits::board() }) else {
            return Ok(());
        };
        let signed = pin_any(&app);
        for path in PATHS {
            for cookie in [None, signed.as_deref()] {
                let (status, text, _) = call(&app, "GET", &format!("{path}?fmt=json"), cookie, b"");
                prop_assert!(answered(status, &text), "GET {} -> {} {}", path, status, text);
            }
        }
    }
}

/// Signs in as whoever the damaged snapshot still has, if anyone.
fn pin_any(app: &App) -> Option<String> {
    (1..4).find_map(|id| {
        let body = format!(r#"{{"person":{id},"pin":"1234"}}"#);
        call(
            app,
            "POST",
            "/api/v1/auth/pin?fmt=json",
            None,
            body.as_bytes(),
        )
        .2
    })
}

/// Not a server error; a 503 is fine only as the rate limiter's "busy".
fn answered(status: u16, text: &str) -> bool {
    status < 500 || (status == 503 && text.contains("\"busy\""))
}

// ---------------------------------------------------------------- guests

proptest! {
    #![proptest_config(cases(256))]

    // The 90-day clean-up only ever takes guests, and only idle ones.
    #[test]
    fn only_idle_guests_are_cleared(
        people in proptest::collection::vec((any::<bool>(), 0u64..200), 1..12),
        later in 0u64..200,
    ) {
        use meritledger::state::{State, GUEST, GUEST_IDLE_SECS, MAX_GUESTS};
        const DAY: u64 = 86_400;
        const START: u64 = 1_800_000_000;
        let mut s = State::new(Limits { pin_rounds: 2, ..Limits::board() });
        s.setup("Dad", "1234", START).unwrap();
        let mut guests = 0;
        for (i, (guest, seen)) in people.iter().enumerate() {
            let id = if *guest && guests < MAX_GUESTS {
                guests += 1;
                s.add_visitor(&format!("G{i}"), START).unwrap()
            } else {
                s.signup(&format!("M{i}"), "1234", START).unwrap()
            };
            s.person_mut(id).unwrap().last_seen = START + seen * DAY;
        }
        let before: Vec<_> = s.people.clone();
        let now = START + later * DAY + 100 * DAY;
        s.purge_idle_guests(now);
        for p in &before {
            let idle = now.saturating_sub(p.last_seen.max(p.created)) > GUEST_IDLE_SECS;
            let gone = s.person(p.id).is_none();
            prop_assert_eq!(gone, p.role == GUEST && idle, "{} ({})", p.display_name, p.role);
        }
    }
}
