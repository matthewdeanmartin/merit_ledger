//! Firmware settings come from the environment or, failing that, from a
//! gitignored `.env` (this directory, then the repo root, then the file
//! named by `MINIFRAMEWORK_ENV`, then the household one in `C:\github`).
//! Values are never printed.
use std::path::{Path, PathBuf};

#[path = "fonts.rs"]
mod fonts;

/// Reads `KEY=value` / `KEY = "value"` lines (comments, optional `export`,
/// quotes).
fn parse(text: &str, want: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != want {
            continue;
        }
        let value = value.trim();
        let unquoted = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')));
        let value = match unquoted {
            Some(v) => v.to_string(),
            None => value.split('#').next().unwrap_or("").trim().to_string(),
        };
        if !value.is_empty() {
            return Some(value);
        }
    }
    None
}

/// Names accepted for each setting, so one household `.env` can serve
/// every board.
fn aliases(key: &str) -> &'static [&'static str] {
    match key {
        "MERITLEDGER_WIFI_SSID" => &[
            "MERITLEDGER_WIFI_SSID",
            "MINIFRAMEWORK_WIFI_SSID",
            "WIFI_SSID",
        ],
        "MERITLEDGER_WIFI_PASSWORD" => &[
            "MERITLEDGER_WIFI_PASSWORD",
            "MINIFRAMEWORK_WIFI_PASSWORD",
            "WIFI_PASSWORD",
        ],
        "MERITLEDGER_HOSTNAME" => &["MERITLEDGER_HOSTNAME"],
        "MERITLEDGER_BUILD" => &["MERITLEDGER_BUILD"],
        "MERITLEDGER_SETUP_CODE" => &["MERITLEDGER_SETUP_CODE"],
        // A Brevo *API* key (xkeysib-...), not an SMTP key.
        "MERITLEDGER_BREVO_API_KEY" => &[
            "MERITLEDGER_BREVO_API_KEY",
            "BREVO_API_KEY",
            "BREVO_OTHER_KEY",
        ],
        "MERITLEDGER_MAIL_FROM" => &["MERITLEDGER_MAIL_FROM", "BREVO_SENDER"],
        _ => &[],
    }
}

const KEYS: [&str; 7] = [
    "MERITLEDGER_BREVO_API_KEY",
    "MERITLEDGER_MAIL_FROM",
    "MERITLEDGER_SETUP_CODE",
    "MERITLEDGER_WIFI_SSID",
    "MERITLEDGER_WIFI_PASSWORD",
    "MERITLEDGER_HOSTNAME",
    "MERITLEDGER_BUILD",
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    // Glyph atlases for the board's screen (also used by the desktop's
    // preview of it).
    println!("cargo:rerun-if-changed=fonts.rs");
    fonts::generate(&root, &out);

    // The TLS certificate follows the hostname: certs/<host>.crt/.key
    // (`make certs`). Plain-HTTP builds still compile the paths, so copy
    // empty placeholders when HTTPS is off and the files are absent.
    if std::env::var_os("CARGO_FEATURE_ESP32").is_some() {
        println!("cargo:rerun-if-env-changed=MERITLEDGER_HOSTNAME");
        let host = std::env::var("MERITLEDGER_HOSTNAME").unwrap_or_else(|_| "meritledger".into());
        let https = std::env::var_os("CARGO_FEATURE_HTTPS").is_some();
        for ext in ["crt", "key"] {
            let source = root.join(format!("certs/{host}.{ext}"));
            println!("cargo:rerun-if-changed={}", source.display());
            if source.exists() {
                std::fs::copy(&source, out.join(format!("server.{ext}"))).unwrap();
            } else if https {
                panic!("no {}: run `make certs`", source.display());
            } else {
                std::fs::write(out.join(format!("server.{ext}")), "").unwrap();
            }
        }
    }

    if std::env::var_os("CARGO_FEATURE_BUNDLED_WEB").is_some() {
        let assets = root.join(".embuild/web/assets.rs");
        assert!(
            assets.exists(),
            "bundled-web needs the Angular bundle: run `make web` first"
        );
        println!("cargo:rerun-if-changed={}", assets.display());
        std::fs::copy(&assets, out.join("assets.rs")).unwrap();
    }
    for cert in ["certs/household-ca.crt", "certs/household-ca.der"] {
        println!("cargo:rerun-if-changed={}", root.join(cert).display());
    }

    #[cfg(feature = "esp32")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("espidf") {
        embuild::espidf::sysenv::output();
    }

    // Wi-Fi and friends are compiled into firmware only; the desktop server
    // reads its own settings at run time.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("espidf") {
        return;
    }
    println!("cargo:rerun-if-env-changed=MINIFRAMEWORK_ENV");
    let mut files: Vec<PathBuf> = vec![root.join(".env"), root.join("../.env")];
    if let Some(extra) = std::env::var_os("MINIFRAMEWORK_ENV") {
        files.push(PathBuf::from(extra));
    }
    // The household-wide one (`C:\github\.env`).
    files.push(root.join("../../.env"));
    let sources: Vec<(PathBuf, String)> = files
        .into_iter()
        .filter_map(|path| {
            let text = std::fs::read_to_string(&path).ok()?;
            println!("cargo:rerun-if-changed={}", path.display());
            Some((path, text))
        })
        .collect();
    for key in KEYS {
        println!("cargo:rerun-if-env-changed={key}");
        if let Some(value) = aliases(key).iter().find_map(|k| std::env::var(k).ok()) {
            println!("cargo:rustc-env={key}={value}");
            continue;
        }
        let found = sources.iter().find_map(|(path, text)| {
            aliases(key)
                .iter()
                .find_map(|name| parse(text, name))
                .map(|value| (path.as_path(), value))
        });
        if let Some((path, value)) = found {
            let shown: &Path = path.strip_prefix(&root).unwrap_or(path);
            println!("cargo:warning={key} taken from {}", shown.display());
            println!("cargo:rustc-env={key}={value}");
        }
    }
}
