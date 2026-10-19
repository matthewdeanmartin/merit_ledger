//! Merit Ledger on a PC: `make run` (API on :8089; `make web-dev` for the
//! Angular dev server), or `make run-bundle` to serve the built UI too.
//!
//! Settings:
//! - MERITLEDGER_ADDR (default 127.0.0.1:8089)
//! - MERITLEDGER_DATA (state directory, default .local/state)
//! - MERITLEDGER_SD (the SD card stand-in directory, default .local/sd;
//!   `none` for no card)
//! - MERITLEDGER_SETUP_CODE (optional code first-run setup needs)
use meritledger::sd::Sd;
use meritledger::{App, Limits};
use miniframework::desktop::{serve, DesktopPlatform};
use miniframework::kv::FileKv;
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
    // Calendar feeds are answered by the app, not static files.
    config.app_paths = &["/cal"];
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
    let mut app = App::open(kv, Arc::new(sd), Limits::board())?;
    app.dev_origins = config.origins.clone();
    app.setup_code = std::env::var("MERITLEDGER_SETUP_CODE")
        .ok()
        .map(|c| &*Box::leak(c.into_boxed_str()));
    log::info!("Merit Ledger on http://{address}/ (state in {data})");
    let site = Site::new(config, app, DesktopPlatform);
    Ok(serve(&site, &address, || site.service.tick())?)
}
