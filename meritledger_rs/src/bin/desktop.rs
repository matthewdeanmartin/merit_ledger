//! Merit Ledger on a PC: `make run` (API on :8089; `make web-dev` for the
//! Angular dev server), or `make run-bundle` to serve the built UI too.
//!
//! Settings:
//! - MERITLEDGER_ADDR (default 127.0.0.1:8089)
//! - MERITLEDGER_DATA (state directory, default .local/state)
//! - MERITLEDGER_SD (the SD card stand-in directory, default .local/sd;
//!   `none` for no card)
//! - MERITLEDGER_SETUP_CODE (optional code first-run setup needs)
#![forbid(unsafe_code)]
use meritledger::outbound::{MailSetup, Outbox};
use meritledger::sd::Sd;
use meritledger::{App, Limits};
use miniframework::desktop::{serve, DesktopPlatform};
use miniframework::fetch::PlainFetch;
use miniframework::kv::FileKv;
use miniframework::oauth::{Config as OAuthConfig, OAuth};
use miniframework::{Config, Site};
use std::sync::Arc;

#[cfg(feature = "bundled-web")]
mod bundle {
    use miniframework::web::Asset;
    include!(concat!(env!("OUT_DIR"), "/assets.rs"));
    pub fn assets() -> &'static [Asset] {
        ASSETS
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    miniframework::desktop::init_logging();
    let address = std::env::var("MERITLEDGER_ADDR").unwrap_or_else(|_| "127.0.0.1:8089".into());
    let data = std::env::var("MERITLEDGER_DATA").unwrap_or_else(|_| ".local/state".into());
    let mut config = Config::new("meritledger", address.clone());
    config.proto_package = "meritledger";
    // Calendar feeds and the Mastodon sign-in callback are the app's.
    config.app_paths = &["/cal", "/auth"];
    config.about = miniframework::about::AboutConfig {
        title: "Merit Ledger",
        description: "Household Buddhist practice ledger (desktop development server)",
        ..Default::default()
    };
    #[cfg(feature = "bundled-web")]
    {
        config.assets = bundle::assets();
    }
    if let Ok(ca) = std::fs::read("certs/household-ca.der") {
        config.ca_der = Some(Box::leak(ca.into_boxed_slice()));
    }
    let sd = match std::env::var("MERITLEDGER_SD").as_deref() {
        Ok("none") => Sd::none(),
        Ok(dir) => Sd::at(dir, None),
        Err(_) => Sd::at(".local/sd", None),
    };
    let kv = Arc::new(FileKv::open(&data)?);
    let mut app = App::open(kv.clone(), Arc::new(sd), Limits::board())?;
    app.dev_origins = config.origins.clone();
    app.origin = format!("http://{address}");
    // DMs and email (Sprint 10): HTTPS with public roots, plus the household
    // CA when certs/household-ca.der is here; the network on its own thread.
    let fetch = Arc::new(PlainFetch {
        timeout: std::time::Duration::from_secs(10),
        roots: std::fs::read("certs/household-ca.der")
            .into_iter()
            .collect(),
    });
    let outbox = Arc::new(Outbox::open(kv.clone(), fetch.clone(), fetch.clone()));
    // Mastodon sign-in and linking (its steps run on the outbox's worker).
    // MERITLEDGER_MASTODON_MOCK=http://127.0.0.1:8099 adds a mock instance
    // (gamematcher's tools/mock_mastodon.py) for walking the flow.
    let oauth = Arc::new(OAuth::new(
        OAuthConfig {
            app_name: "Merit Ledger",
            website: format!("http://{address}"),
            extra_scopes: "write:statuses",
            ..OAuthConfig::default()
        },
        kv,
        fetch.clone(),
        fetch,
    ));
    if let Ok(mock) = std::env::var("MERITLEDGER_MASTODON_MOCK") {
        app.instances
            .push(meritledger::oauth_provider("mock", &mock));
    }
    oauth.set_providers(app.instances.clone());
    outbox.attach_oauth(oauth.clone());
    app.oauth = Some(oauth);
    let worker = outbox.clone();
    std::thread::spawn(move || loop {
        let now = miniframework::wall_ms().map_or(0, |ms| ms / 1000);
        if !worker.run_once(now) {
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    });
    // Email: a Brevo API key in the environment (or set on the Household page).
    if let Some(key) = ["MERITLEDGER_BREVO_API_KEY", "BREVO_API_KEY"]
        .iter()
        .find_map(|k| std::env::var(k).ok())
    {
        let setup = MailSetup {
            api_key: key,
            sender_email: std::env::var("MERITLEDGER_MAIL_FROM").unwrap_or_default(),
            built_in: true,
        };
        if let Err(e) = outbox.begin_mail_setup(setup) {
            log::warn!("email: {}", e.message);
        }
    }
    app.outbox = Some(outbox);
    app.setup_code = std::env::var("MERITLEDGER_SETUP_CODE")
        .ok()
        .map(|c| &*Box::leak(c.into_boxed_str()));
    log::info!("Merit Ledger on http://{address}/ (state in {data})");
    let site = Site::new(config, app, DesktopPlatform);
    Ok(serve(&site, &address, || site.service.tick())?)
}
