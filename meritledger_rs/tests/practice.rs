//! Sprint 3 end to end: the Today page, one-tap logging, recording for
//! someone else, visibility, custom kinds, pins and the household zone.
//! The desktop clock is real, so "today" here is the machine's today.
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
    Res {
        status: reply.status(),
        body: String::from_utf8_lossy(reply.body()).into_owned(),
        cookie,
    }
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
    let r = call(
        app,
        "POST",
        "/api/v1/admin/people",
        Some(&dad),
        r#"{"display_name":"Kid","role":"member","pin":"1111","email":""}"#,
    );
    assert_eq!(r.status, 201, "{}", r.body);
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

/// The `"count":N` of a tile in a TodayView.
fn tile_count(body: &str, key: &str) -> u32 {
    let at = body
        .find(&format!("\"key\":\"{key}\""))
        .unwrap_or_else(|| panic!("{key} in {body}"));
    let rest = &body[at..];
    let c = rest.find("\"count\":").unwrap() + 8;
    rest[c..].split([',', '}']).next().unwrap().parse().unwrap()
}

#[test]
fn one_tap_logs_today() {
    let app = app();
    let (dad, _) = household(&app);
    let today = call(&app, "GET", "/api/v1/today", Some(&dad), "");
    assert_eq!(today.status, 200, "{}", today.body);
    assert_eq!(tile_count(&today.body, "meditation"), 0);
    let r = call(
        &app,
        "POST",
        "/api/v1/entries",
        Some(&dad),
        r#"{"person":0,"kind":"meditation","note":""}"#,
    );
    assert_eq!(r.status, 201, "{}", r.body);
    assert_eq!(tile_count(&r.body, "meditation"), 1);
    assert!(
        r.body.contains(r#""amount":20"#),
        "the default amount: {}",
        r.body
    );
}

#[test]
fn recording_for_the_kid_and_privacy() {
    let app = app();
    let (dad, kid) = household(&app);
    // Dad records prostrations for the kid; the kid logs a private demerit.
    let r = call(
        &app,
        "POST",
        "/api/v1/entries",
        Some(&dad),
        r#"{"person":2,"kind":"prostrations","note":""}"#,
    );
    assert_eq!(r.status, 201, "{}", r.body);
    assert!(
        r.body.contains(r#""person_name":"Kid""#) && r.body.contains(r#""recorded_by_name":"Dad""#),
        "{}",
        r.body
    );
    call(
        &app,
        "POST",
        "/api/v1/entries",
        Some(&kid),
        r#"{"person":0,"kind":"anger","note":"at my sister"}"#,
    );
    // Anyone, signed in or not, sees the public one only.
    let public = call(&app, "GET", "/api/v1/people/2/entries", None, "");
    assert_eq!(public.status, 200, "{}", public.body);
    assert!(
        public.body.contains("Prostrations") && !public.body.contains("sister"),
        "{}",
        public.body
    );
    // The kid sees both.
    let own = call(&app, "GET", "/api/v1/people/2/entries", Some(&kid), "");
    assert!(own.body.contains("sister"), "{}", own.body);
}

#[test]
fn edits_need_the_current_rev() {
    let app = app();
    let (dad, _) = household(&app);
    call(
        &app,
        "POST",
        "/api/v1/entries",
        Some(&dad),
        r#"{"person":0,"kind":"chanting","note":""}"#,
    );
    let ok = call(
        &app,
        "PATCH",
        "/api/v1/entries/1",
        Some(&dad),
        r#"{"rev":1,"amount":45}"#,
    );
    assert_eq!(ok.status, 200, "{}", ok.body);
    assert!(ok.body.contains(r#""amount":45"#));
    let stale = call(
        &app,
        "PATCH",
        "/api/v1/entries/1",
        Some(&dad),
        r#"{"rev":1,"amount":50}"#,
    );
    assert_eq!(stale.status, 409, "{}", stale.body);
    let gone = call(&app, "DELETE", "/api/v1/entries/1", Some(&dad), "");
    assert_eq!(gone.status, 200);
    assert_eq!(tile_count(&gone.body, "chanting"), 0);
}

#[test]
fn custom_kinds_and_pins() {
    let app = app();
    let (_, kid) = household(&app);
    let body = r#"{"name":"Tea ceremony","icon":"🍵","polarity":"merit","category":"ritual","unit":"minutes","default_amount":15,"visibility":"","household":false}"#;
    let r = call(&app, "POST", "/api/v1/kinds", Some(&kid), body);
    assert_eq!(r.status, 201, "{}", r.body);
    assert!(r.body.contains(r#""key":"c1""#), "{}", r.body);
    let pins = call(
        &app,
        "PUT",
        "/api/v1/me/pins",
        Some(&kid),
        r#"{"kinds":["c1","daimoku"]}"#,
    );
    assert_eq!(pins.status, 200, "{}", pins.body);
    let today = call(&app, "GET", "/api/v1/today", Some(&kid), "");
    let c1 = today.body.find(r#""key":"c1""#).unwrap();
    let daimoku = today.body.find(r#""key":"daimoku""#).unwrap();
    assert!(c1 < daimoku, "pinned order is kept");
    assert!(
        !today.body.contains(r#""key":"meditation""#),
        "defaults replaced"
    );
}

#[test]
fn removing_someone_asks_about_their_entries() {
    let app = app();
    let (dad, kid) = household(&app);
    call(
        &app,
        "POST",
        "/api/v1/entries",
        Some(&kid),
        r#"{"person":0,"kind":"meditation","note":""}"#,
    );
    let unsaid = call(&app, "DELETE", "/api/v1/admin/people/2", Some(&dad), "");
    assert_eq!(unsaid.status, 400, "{}", unsaid.body);
    let kept = call(
        &app,
        "DELETE",
        "/api/v1/admin/people/2?entries=keep",
        Some(&dad),
        "",
    );
    assert_eq!(kept.status, 200, "{}", kept.body);
    let entries = call(&app, "GET", "/api/v1/people/2/entries", Some(&dad), "");
    assert!(entries.body.contains("a former member"), "{}", entries.body);
}

#[test]
fn the_household_zone_is_a_setting() {
    let app = app();
    let (dad, kid) = household(&app);
    let settings = call(&app, "GET", "/api/v1/admin/settings", Some(&dad), "");
    assert!(settings.body.contains("EST5EDT"), "{}", settings.body);
    let bad = call(
        &app,
        "PATCH",
        "/api/v1/admin/settings",
        Some(&dad),
        r#"{"timezone":"Mars/Olympus"}"#,
    );
    assert_eq!(bad.status, 400);
    let ok = call(
        &app,
        "PATCH",
        "/api/v1/admin/settings",
        Some(&dad),
        r#"{"timezone":"CET-1CEST,M3.5.0,M10.5.0/3"}"#,
    );
    assert_eq!(ok.status, 200, "{}", ok.body);
    assert_eq!(
        call(
            &app,
            "PATCH",
            "/api/v1/admin/settings",
            Some(&kid),
            r#"{"timezone":"UTC0"}"#
        )
        .status,
        403
    );
}

#[test]
fn practice_needs_sign_in() {
    let app = app();
    household(&app);
    assert_eq!(call(&app, "GET", "/api/v1/today", None, "").status, 401);
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/v1/entries",
            None,
            r#"{"person":1,"kind":"meditation","note":""}"#
        )
        .status,
        401
    );
}

#[test]
fn the_board_records_without_a_session() {
    let app = app();
    let (dad, _) = household(&app);
    app.record_from_board(2, "vegetarian").unwrap();
    let entries = call(&app, "GET", "/api/v1/people/2/entries", Some(&dad), "");
    assert!(
        entries.body.contains(r#""source":"board""#) && entries.body.contains("the board"),
        "{}",
        entries.body
    );
}

#[test]
fn profiles_and_the_feed_are_public_without_details() {
    let app = app();
    let (_, kid) = household(&app);
    call(
        &app,
        "POST",
        "/api/v1/entries",
        Some(&kid),
        r#"{"person":0,"kind":"meditation","note":"sat by the window"}"#,
    );
    call(
        &app,
        "POST",
        "/api/v1/entries",
        Some(&kid),
        r#"{"person":0,"kind":"harsh_speech","note":"snapped","visibility":"public"}"#,
    );
    call(
        &app,
        "POST",
        "/api/v1/entries",
        Some(&kid),
        r#"{"person":0,"kind":"repentance","note":"for snapping"}"#,
    );
    // Signed out: the profile and feed answer, without demerit or repentance details.
    let profile = call(&app, "GET", "/api/v1/people/2/profile", None, "");
    assert_eq!(profile.status, 200, "{}", profile.body);
    assert!(
        profile.body.contains(r#""current_streak":1"#),
        "{}",
        profile.body
    );
    assert!(
        profile.body.contains("sat by the window"),
        "{}",
        profile.body
    );
    assert!(
        !profile.body.contains("snapped") && !profile.body.contains("for snapping"),
        "{}",
        profile.body
    );
    assert!(
        profile.body.contains("A moment of reflection"),
        "{}",
        profile.body
    );
    let feed = call(&app, "GET", "/api/v1/feed", None, "");
    assert_eq!(feed.status, 200, "{}", feed.body);
    assert!(
        !feed.body.contains("snapped") && !feed.body.contains("Harsh speech"),
        "{}",
        feed.body
    );
    // The kid sees their own details.
    let own = call(&app, "GET", "/api/v1/people/2/profile", Some(&kid), "");
    assert!(
        own.body.contains("snapped") && own.body.contains(r#""demerits_30":1"#),
        "{}",
        own.body
    );
}

#[test]
fn rejoicing_from_the_feed() {
    let app = app();
    let (dad, kid) = household(&app);
    call(
        &app,
        "POST",
        "/api/v1/entries",
        Some(&kid),
        r#"{"person":0,"kind":"prostrations","note":""}"#,
    );
    let r = call(&app, "POST", "/api/v1/entries/1/rejoice", Some(&dad), "");
    assert_eq!(r.status, 200, "{}", r.body);
    assert!(
        r.body.contains(r#""rejoices":1"#) && r.body.contains(r#""rejoiced_by_me":true"#),
        "{}",
        r.body
    );
    assert_eq!(
        call(&app, "POST", "/api/v1/entries/1/rejoice", Some(&dad), "").status,
        409
    );
    assert_eq!(
        call(&app, "POST", "/api/v1/entries/1/rejoice", None, "").status,
        401
    );
    // Dad now has a rejoicing entry of his own.
    let today = call(&app, "GET", "/api/v1/people/1/entries", Some(&dad), "");
    assert!(
        today.body.contains("In Kid") && today.body.contains("prostrations"),
        "{}",
        today.body
    );
}

#[test]
fn timer_presets_and_a_retried_session() {
    let app = app();
    let (dad, _) = household(&app);
    let list = call(&app, "GET", "/api/v1/timers", Some(&dad), "");
    assert_eq!(list.status, 200, "{}", list.body);
    assert!(
        list.body.contains(r#""key":"sesshin""#) && list.body.contains(r#""builtin":true"#),
        "{}",
        list.body
    );
    let body = r#"{"name":"Lunch sit","segments":[{"kind":"sit","minutes":12}],"open_strikes":1,"interval_minutes":0,"close_strikes":1,"count_target":0,"logs_as":"meditation","timbre":"bell","household":false,"archived":false}"#;
    let r = call(&app, "POST", "/api/v1/timers", Some(&dad), body);
    assert_eq!(r.status, 201, "{}", r.body);
    assert!(r.body.contains(r#""key":"t1""#), "{}", r.body);
    // A finished session, sent twice (the reply to the first was lost).
    let session = r#"{"person":0,"kind":"meditation","amount":12,"note":"","source":"timer","client_id":"sess-1"}"#;
    call(&app, "POST", "/api/v1/entries", Some(&dad), session);
    let again = call(&app, "POST", "/api/v1/entries", Some(&dad), session);
    assert_eq!(again.status, 201, "{}", again.body);
    assert_eq!(
        tile_count(&again.body, "meditation"),
        1,
        "the retry recorded nothing new"
    );
}

#[test]
fn calendars_today_and_feeds() {
    let app = app();
    let (dad, kid) = household(&app);
    // Public, with the household's calendar (Chinese-inspired by default).
    let cal = call(
        &app,
        "GET",
        "/api/v1/calendar?from=2026-05-01&to=2026-05-31",
        None,
        "",
    );
    assert_eq!(cal.status, 200, "{}", cal.body);
    assert!(
        cal.body.contains(r#""system":"chinese""#) && cal.body.contains("Buddha's Birthday"),
        "{}",
        cal.body
    );
    assert!(cal.body.contains(r#""phase":"full""#), "{}", cal.body);
    // The kid follows the Theravada calendar; the household moves to Sundays.
    let r = call(
        &app,
        "PATCH",
        "/api/v1/me",
        Some(&kid),
        r#"{"calendar":"theravada"}"#,
    );
    assert!(r.body.contains(r#""calendar":"theravada""#), "{}", r.body);
    let kid_cal = call(
        &app,
        "GET",
        "/api/v1/calendar?from=2026-05-01&to=2026-05-31",
        Some(&kid),
        "",
    );
    assert!(kid_cal.body.contains("Vesak"), "{}", kid_cal.body);
    call(
        &app,
        "PATCH",
        "/api/v1/admin/settings",
        Some(&dad),
        r#"{"sunday_shift":true}"#,
    );
    let shifted = call(
        &app,
        "GET",
        "/api/v1/calendar?from=2026-05-01&to=2026-06-30&system=theravada",
        None,
        "",
    );
    // Vesak 2026 is a Sunday and stays; Monday 29 June's full moon moves back a day.
    assert!(
        !shifted.body.contains(r#""shifted_from":"2026-05-31""#),
        "{}",
        shifted.body
    );
    assert!(
        shifted
            .body
            .contains(r#""date":"2026-06-28","kind":"uposatha","name":"Uposatha (full moon)","feast":false,"shifted_from":"2026-06-29""#),
        "{}",
        shifted.body
    );
    assert!(shifted.body.contains(r#""sunday_shift":true"#));
    assert_eq!(
        call(
            &app,
            "PATCH",
            "/api/v1/me",
            Some(&kid),
            r#"{"calendar":"aztec"}"#
        )
        .status,
        400
    );
    // Today carries today's observances (maybe none).
    assert!(call(&app, "GET", "/api/v1/today", Some(&dad), "")
        .body
        .contains(r#""observances":["#));
    // The feed: no session, plain iCalendar.
    let mut headers = vec![("Host".to_string(), "meritledger.local".to_string())];
    headers.push(("Origin".into(), ORIGIN.into()));
    let req = miniframework::Request::new("GET", "/cal/chinese.ics", &headers, b"", false);
    let (mut out, mut spare) = (Vec::new(), Vec::new());
    let mut reply = miniframework::Reply::new(&mut out, &mut spare, 256 * 1024);
    app.handle(&req, &mut reply);
    assert_eq!(reply.status(), 200);
    let ics = String::from_utf8_lossy(reply.owned_body().unwrap()).into_owned();
    assert!(
        ics.starts_with("BEGIN:VCALENDAR") && ics.contains("Lunar New Year"),
        "{}",
        &ics[..200]
    );
    let req = miniframework::Request::new("GET", "/cal/aztec.ics", &headers, b"", false);
    let (mut out, mut spare) = (Vec::new(), Vec::new());
    let mut reply = miniframework::Reply::new(&mut out, &mut spare, 256 * 1024);
    app.handle(&req, &mut reply);
    assert_eq!(reply.status(), 404);
}

#[test]
fn time_zones_by_place_name() {
    let app = app();
    let (dad, _) = household(&app);
    let zones = call(&app, "GET", "/api/v1/timezones", None, "");
    assert!(
        zones
            .body
            .contains(r#""id":"America/Chicago","label":"Chicago (Central)""#),
        "{}",
        zones.body
    );
    let before = call(&app, "GET", "/api/v1/admin/settings", Some(&dad), "");
    assert!(
        before
            .body
            .contains(r#""timezone_label":"New York (Eastern)""#),
        "{}",
        before.body
    );
    // What the picker or "use this phone's zone" sends.
    let r = call(
        &app,
        "PATCH",
        "/api/v1/admin/settings",
        Some(&dad),
        r#"{"timezone":"America/Chicago"}"#,
    );
    assert_eq!(r.status, 200, "{}", r.body);
    assert!(
        r.body.contains(r#""timezone":"CST6CDT,M3.2.0,M11.1.0""#)
            && r.body.contains(r#""timezone_label":"Chicago (Central)""#),
        "{}",
        r.body
    );
    assert_eq!(
        call(
            &app,
            "PATCH",
            "/api/v1/admin/settings",
            Some(&dad),
            r#"{"timezone":"Mars/Olympus_Mons"}"#
        )
        .status,
        400
    );
}
