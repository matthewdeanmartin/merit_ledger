//! Sprint 9 end to end: vows and their lifecycle, the vow blog (bodies on
//! the SD card, or in the snapshot without one), precepts on profiles, and
//! who sees what. The desktop clock is real, so "today" is the machine's.
use meritledger::sd::Sd;
use meritledger::{App, Limits};
use miniframework::kv::MemKv;
use miniframework::{Reply, Request, Service};
use std::path::PathBuf;
use std::sync::Arc;

const ORIGIN: &str = "http://meritledger.local";

fn card(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ml-vows-{name}-{}", std::process::id()));
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

#[test]
fn a_vow_through_breach_and_repair() {
    let app = app(None);
    let (dad, kid) = household(&app);
    let body = ok(
        call(
            &app,
            "POST",
            "/api/v1/vows",
            Some(&dad),
            r#"{"text":"Right speech at dinner","visibility":"public"}"#,
        ),
        201,
    );
    assert!(body.contains(r#""state":"active""#), "{body}");
    assert!(
        body.contains(r#""actions":["pause","breach","complete","retire"]"#),
        "{body}"
    );
    // An action with no body at all, as a bare POST.
    let body = ok(
        call(&app, "POST", "/api/v1/vows/1/breach", Some(&dad), ""),
        200,
    );
    assert!(body.contains(r#""state":"breached""#), "{body}");
    // Others don't see an unshown breach.
    let theirs = ok(call(&app, "GET", "/api/v1/vows/1", Some(&kid), ""), 200);
    assert!(theirs.contains(r#""state":"active""#), "{theirs}");
    assert!(!theirs.contains("Broke"), "{theirs}");
    // Illegal transitions are 409.
    let r = call(&app, "POST", "/api/v1/vows/1/resume", Some(&dad), "");
    assert_eq!(r.status, 409, "{}", r.body);
    // Only its person acts on it.
    assert_eq!(
        call(&app, "POST", "/api/v1/vows/1/repair", Some(&kid), "").status,
        403
    );
    let body = ok(
        call(
            &app,
            "POST",
            "/api/v1/vows/1/repair",
            Some(&dad),
            r#"{"note":"Said sorry","visibility":""}"#,
        ),
        200,
    );
    assert!(body.contains(r#""state":"active""#), "{body}");
    assert!(body.contains("Renewed this vow"), "{body}");
    // The repair is a repentance practice on Dad's day, generic to others.
    let today = ok(
        call(&app, "GET", "/api/v1/today?person=1", Some(&kid), ""),
        200,
    );
    assert!(today.contains("A repentance practice"), "{today}");
    assert!(!today.contains("Said sorry"), "{today}");
    // Unknown actions and vows.
    assert_eq!(
        call(&app, "POST", "/api/v1/vows/1/explode", Some(&dad), "").status,
        404
    );
    assert_eq!(call(&app, "GET", "/api/v1/vows/99", None, "").status, 404);
}

#[test]
fn vows_and_posts_follow_visibility() {
    let dir = card("visibility");
    let app = app(Some(&dir));
    let (dad, kid) = household(&app);
    for (text, vis) in [
        ("Open", "public"),
        ("Quiet", "generic"),
        ("Mine", "private"),
    ] {
        let input = format!(r#"{{"text":"{text}","visibility":"{vis}"}}"#);
        ok(call(&app, "POST", "/api/v1/vows", Some(&dad), &input), 201);
    }
    for vow in 1..=3 {
        let input = format!(
            r#"{{"title":"Post {vow}","body":"Day **{vow}** <script>x</script>","date":""}}"#
        );
        ok(
            call(
                &app,
                "POST",
                &format!("/api/v1/vows/{vow}/posts"),
                Some(&dad),
                &input,
            ),
            201,
        );
    }
    // The bodies are on the card.
    assert!(dir.join("ml/posts/1.md").exists());
    let page =
        |cookie: Option<&str>| ok(call(&app, "GET", "/api/v1/people/1/vows", cookie, ""), 200);
    for (theirs, member) in [(page(Some(&kid)), true), (page(None), false)] {
        assert!(theirs.contains(r#""text":"Open""#), "{theirs}");
        assert!(
            !theirs.contains("Quiet") && !theirs.contains("Mine"),
            "{theirs}"
        );
        // The household sees the generic vow as "a vow"; visitors (and
        // guests) see only public ones (owner, 2026-10-21).
        assert_eq!(theirs.contains(r#""generic":true"#), member, "{theirs}");
        assert!(
            theirs.contains("Post 1") && !theirs.contains("Post 2"),
            "{theirs}"
        );
        assert!(theirs.contains("&lt;script&gt;"), "escaped: {theirs}");
        assert!(!theirs.contains("<script>"), "{theirs}");
        assert!(
            theirs.contains(r#""body":"""#),
            "markdown only for its author: {theirs}"
        );
    }
    let mine = page(Some(&dad));
    assert!(
        mine.contains("Quiet") && mine.contains("Mine") && mine.contains("Post 3"),
        "{mine}"
    );
    assert_eq!(
        call(&app, "GET", "/api/v1/posts/3", Some(&kid), "").status,
        404
    );
    assert_eq!(call(&app, "GET", "/api/v1/posts/1", None, "").status, 200);
    // Deleting a vow deletes its posts and their files.
    ok(call(&app, "DELETE", "/api/v1/vows/1", Some(&dad), ""), 200);
    assert!(!dir.join("ml/posts/1.md").exists());
    assert_eq!(
        call(&app, "GET", "/api/v1/posts/1", Some(&dad), "").status,
        404
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn editing_a_post_keeps_its_place() {
    let dir = card("edit");
    let app = app(Some(&dir));
    let (dad, kid) = household(&app);
    ok(
        call(
            &app,
            "POST",
            "/api/v1/vows",
            Some(&dad),
            r#"{"text":"Sit"}"#,
        ),
        201,
    );
    ok(
        call(
            &app,
            "POST",
            "/api/v1/vows/1/posts",
            Some(&dad),
            r#"{"title":"One","body":"first"}"#,
        ),
        201,
    );
    let edit = r#"{"title":"One again","body":"second","rev":1}"#;
    assert_eq!(
        call(&app, "PUT", "/api/v1/posts/1", Some(&kid), edit).status,
        403
    );
    let body = ok(call(&app, "PUT", "/api/v1/posts/1", Some(&dad), edit), 200);
    assert!(
        body.contains("One again") && body.contains("<p>second</p>"),
        "{body}"
    );
    assert_eq!(
        call(&app, "PUT", "/api/v1/posts/1", Some(&dad), edit).status,
        409
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("ml/posts/1.md")).unwrap(),
        "second"
    );
    let body = ok(call(&app, "DELETE", "/api/v1/posts/1", Some(&dad), ""), 200);
    assert!(body.contains(r#""posts":[]"#), "{body}");
    assert!(!dir.join("ml/posts/1.md").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn precepts_on_a_profile_and_as_vows() {
    let app = app(None);
    let (dad, kid) = household(&app);
    let catalog = ok(call(&app, "GET", "/api/v1/precepts", None, ""), 200);
    assert!(
        catalog.contains("T24n1484_p1004b16"),
        "citations come with the sets"
    );
    // Some of the Upāsaka precepts: three majors and two minors.
    let take = r#"{"date":"2025-05-12","note":"Fo Guang Shan, with Venerable Miao","majors":[1,2,4],"minors":[21,22],"visibility":"public"}"#;
    let body = ok(
        call(&app, "PUT", "/api/v1/me/precepts/upasaka", Some(&dad), take),
        200,
    );
    assert!(
        body.contains(r#""majors":[1,2,4]"#) && body.contains(r#""minors":[21,22]"#),
        "{body}"
    );
    let bad = r#"{"majors":[7],"minors":[]}"#;
    assert_eq!(
        call(&app, "PUT", "/api/v1/me/precepts/upasaka", Some(&dad), bad).status,
        400
    );
    assert_eq!(
        call(&app, "PUT", "/api/v1/me/precepts/nope", Some(&dad), take).status,
        404
    );
    assert_eq!(
        call(&app, "PUT", "/api/v1/me/precepts/upasaka", None, take).status,
        401
    );
    // The profile shows exactly what was taken.
    let theirs = ok(
        call(&app, "GET", "/api/v1/people/1/vows", Some(&kid), ""),
        200,
    );
    assert!(
        theirs.contains(r#""majors":[1,2,4]"#) && theirs.contains("Venerable Miao"),
        "{theirs}"
    );
    // A taken precept becomes a vow; one not taken can't.
    let not_taken = r#"{"precept_set":"upasaka","precept_kind":"major","precept_n":3}"#;
    assert_eq!(
        call(&app, "POST", "/api/v1/vows", Some(&dad), not_taken).status,
        409
    );
    let taxes = r#"{"precept_set":"upasaka","precept_kind":"minor","precept_n":21}"#;
    let body = ok(call(&app, "POST", "/api/v1/vows", Some(&dad), taxes), 201);
    assert!(body.contains("pay the taxes you owe"), "{body}");
    assert!(body.contains(r#""precept_n":21"#), "{body}");
    // Generic: the set shows, the items don't.
    let quiet = r#"{"majors":[1],"minors":[],"visibility":"generic"}"#;
    ok(
        call(&app, "PUT", "/api/v1/me/precepts/five", Some(&dad), quiet),
        200,
    );
    let theirs = ok(
        call(&app, "GET", "/api/v1/people/1/vows", Some(&kid), ""),
        200,
    );
    assert!(
        theirs.contains(r#""set":"five""#) && theirs.contains(r#""generic":true"#),
        "{theirs}"
    );
    // No longer holding.
    ok(
        call(&app, "DELETE", "/api/v1/me/precepts/five", Some(&dad), ""),
        200,
    );
    let theirs = ok(
        call(&app, "GET", "/api/v1/people/1/vows", Some(&kid), ""),
        200,
    );
    assert!(!theirs.contains(r#""set":"five""#), "{theirs}");
}

#[test]
fn backups_carry_vows_posts_and_precepts() {
    let dir = card("export");
    let app1 = app(Some(&dir));
    let (dad, _) = household(&app1);
    ok(
        call(
            &app1,
            "POST",
            "/api/v1/vows",
            Some(&dad),
            r#"{"text":"Sit daily"}"#,
        ),
        201,
    );
    ok(
        call(
            &app1,
            "POST",
            "/api/v1/vows/1/posts",
            Some(&dad),
            r#"{"title":"Week one","body":"Still sitting."}"#,
        ),
        201,
    );
    ok(
        call(
            &app1,
            "PUT",
            "/api/v1/me/precepts/five",
            Some(&dad),
            r#"{"majors":[1,2,3,4,5],"minors":[]}"#,
        ),
        200,
    );
    let export = ok(
        call(&app1, "GET", "/api/v1/admin/export.json", Some(&dad), ""),
        200,
    );
    assert!(
        export.contains("Still sitting."),
        "the body travels: {export}"
    );

    // Into a fresh board with a card: the post body lands on the card.
    let dir2 = card("import");
    let app2 = app(Some(&dir2));
    let (dad2, _) = household(&app2);
    let report = ok(
        call(&app2, "POST", "/api/v1/admin/import", Some(&dad2), &export),
        200,
    );
    assert!(report.contains(r#""vows_added":1"#), "{report}");
    assert!(
        report.contains(r#""posts_added":1"#) && report.contains(r#""precepts_added":1"#),
        "{report}"
    );
    let page = ok(
        call(&app2, "GET", "/api/v1/people/1/vows", Some(&dad2), ""),
        200,
    );
    assert!(
        page.contains("Sit daily") && page.contains("<p>Still sitting.</p>"),
        "{page}"
    );
    assert_eq!(
        std::fs::read_to_string(dir2.join("ml/posts/1.md")).unwrap(),
        "Still sitting."
    );
    // Importing again adds nothing.
    let again = ok(
        call(&app2, "POST", "/api/v1/admin/import", Some(&dad2), &export),
        200,
    );
    assert!(
        again.contains(r#""vows_added":0"#) && again.contains(r#""posts_added":0"#),
        "{again}"
    );
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}
