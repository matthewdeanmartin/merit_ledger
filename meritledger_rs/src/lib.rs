//! Merit Ledger: a household Buddhist practice ledger served from an
//! ESP32-S3. See `../spec/` for the design and `../README.md` to run it.
//!
//! No `unsafe` here, enforced: the only `unsafe` in the project is the
//! board's FFI in `bin/s3.rs` (the screen driver and FAT free space).
#![forbid(unsafe_code)]
pub mod api;
pub mod astro;
pub mod calendar;
pub mod catalog;
pub mod events;
#[cfg(feature = "gate")]
pub mod gate;
pub mod ics;
pub mod journal;
pub mod ledger;
pub mod markdown;
pub mod messages;
pub mod notify;
pub mod outbound;
pub mod pin;
pub mod practice_docs;
pub mod precepts;
pub mod publish;
pub mod reading;
pub mod screen;
pub mod sd;
pub mod state;
pub mod stats;
pub mod store;
pub mod timers;
pub mod transfer;
pub mod tz;
pub mod vows;
pub mod zones;

pub use api::App;

/// A Mastodon instance to sign in with, trusted by its URL: `http://` or a
/// `.local` host is the household's, anything else public.
pub fn oauth_provider(id: &str, base: &str) -> miniframework::oauth::Provider {
    let base = base.trim_end_matches('/');
    let household = base.starts_with("http://")
        || base.split_once("://").is_some_and(|(_, h)| {
            h.split(['/', ':'])
                .next()
                .is_some_and(|h| h.ends_with(".local"))
        });
    miniframework::oauth::Provider {
        id: id.into(),
        base: base.into(),
        trust: if household {
            miniframework::oauth::Trust::Household
        } else {
            miniframework::oauth::Trust::Public
        },
    }
}
pub use state::Limits;
