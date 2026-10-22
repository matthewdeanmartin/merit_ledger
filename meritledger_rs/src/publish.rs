//! What may go to Mastodon (`sprints/sprint11.md`, `spec/v2_decisions.md`
//! § Content vs noise): Mastodon only mirrors the board, so nothing goes
//! there that isn't visible on the board to everyone, and nothing that is
//! noise.
//!
//! - **Entries:** `public` entries of `content` kinds post in full;
//!   `generic` ones post only their category ("A meditation practice").
//!   `private` entries, demerits, repentance and noise kinds never post.
//! - **Vow posts:** only posts on a `public` vow, as their title and text.
//!
//! [`publishable`] is the whole rule, as a pure function; the outbox and
//! the routes only ever ask it.
use crate::catalog;
use crate::ledger::sensitive;
use crate::messages::{StoredEntry, StoredKind, StoredPost, StoredVow};
use crate::state::State;
use miniframework::ApiError;

/// "30 min", "2 h", "1,080×", as the UI says it (`practice.ts`).
pub fn amount_label(unit: &str, amount: u32) -> String {
    match unit {
        "minutes" if amount >= 60 && amount.is_multiple_of(60) => format!("{} h", amount / 60),
        "minutes" => format!("{amount} min"),
        _ => {
            let digits = amount.to_string();
            let mut out = String::new();
            for (i, c) in digits.chars().enumerate() {
                if i > 0 && (digits.len() - i).is_multiple_of(3) {
                    out.push(',');
                }
                out.push(c);
            }
            format!("{out}×")
        }
    }
}

/// Mastodon's default status limit, with room to spare.
pub const STATUS_MAX: usize = 480;

/// Something on the board that might be published.
pub enum Item<'a> {
    Entry(&'a StoredEntry, &'a StoredKind),
    /// A vow post, its vow, and its markdown body.
    Post(&'a StoredPost, &'a StoredVow, &'a str),
}

/// The key the outbox remembers a published item by.
pub fn key(item: &Item) -> String {
    match item {
        Item::Entry(e, _) => format!("e{}", e.id),
        Item::Post(p, _, _) => format!("p{}", p.id),
    }
}

/// Plain text from the posts' markdown subset (`markdown.rs`): emphasis
/// marks dropped, links as "text (url)", list items on their own lines.
pub fn plain(md: &str) -> String {
    let mut out = String::new();
    for block in md.replace("\r\n", "\n").split("\n\n") {
        let lines: Vec<&str> = block
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        if lines.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        let text: Vec<String> = lines
            .iter()
            .map(|l| {
                let l = l
                    .strip_prefix("- ")
                    .or_else(|| l.strip_prefix("* "))
                    .map_or(l.to_string(), |r| format!("• {r}"));
                let mut s = String::new();
                let mut rest = l.as_str();
                while let Some(open) = rest.find('[') {
                    let tail = &rest[open..];
                    match (tail.find("]("), tail.find(')')) {
                        (Some(mid), Some(end)) if mid < end => {
                            s.push_str(&rest[..open]);
                            s.push_str(&tail[1..mid]);
                            s.push_str(" (");
                            s.push_str(&tail[mid + 2..end]);
                            s.push(')');
                            rest = &tail[end + 1..];
                        }
                        _ => {
                            s.push_str(&rest[..open + 1]);
                            rest = &rest[open + 1..];
                        }
                    }
                }
                s.push_str(rest);
                s.replace("**", "")
                    .replace('*', "")
                    .replace('_', " ")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        out.push_str(&text.join("\n"));
    }
    out
}

fn cut(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let kept: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", kept.trim_end())
}

/// The status text for `item`, or `None` when it must not leave the board.
/// `link` is appended when given (the board's address works only at home,
/// so callers pass one only for household instances).
pub fn publishable(item: &Item, link: Option<&str>) -> Option<String> {
    let mut text = match item {
        Item::Entry(e, kind) => {
            if e.deleted
                || e.visibility == "private"
                || !kind.content
                || sensitive(kind)
                || kind.polarity != "merit"
            {
                return None;
            }
            if e.visibility == "generic" {
                format!("🌿 {}", catalog::generic_label(kind))
            } else {
                let mut s = format!("{} {}", kind.icon, kind.name);
                if kind.unit != "once" && e.amount > 0 {
                    s.push_str(&format!(" · {}", amount_label(&kind.unit, e.amount)));
                }
                if !e.note.is_empty() {
                    s.push_str(&format!("\n\n{}", e.note));
                }
                s
            }
        }
        Item::Post(p, vow, body) => {
            if vow.visibility != "public" {
                return None;
            }
            let mut s = format!("🪷 {}", p.title);
            let body = plain(body);
            if !body.is_empty() {
                s.push_str("\n\n");
                s.push_str(&body);
            }
            s.push_str(&format!("\n\n(On my vow: {})", vow.text));
            s
        }
    };
    let tail = link.map(|l| format!("\n\n{l}")).unwrap_or_default();
    text = cut(&text, STATUS_MAX - tail.chars().count());
    text.push_str(&tail);
    Some(text)
}

/// Mastodon visibility for someone's posts ("" = unlisted).
pub fn check_visibility(v: &str) -> Result<String, ApiError> {
    match v {
        "" | "unlisted" => Ok("unlisted".into()),
        "public" | "private" => Ok(v.into()),
        _ => Err(ApiError::bad_request(
            "Mastodon visibility: public, unlisted or private (followers only)",
        )),
    }
}

impl State {
    /// Links a Mastodon account to `person` (one person per account).
    pub fn link_identity(
        &mut self,
        person: u32,
        provider: &str,
        subject: &str,
        handle: &str,
    ) -> Result<(), ApiError> {
        if let Some(other) = self
            .people
            .iter()
            .find(|p| p.id != person && p.link_provider == provider && p.link_subject == subject)
        {
            return Err(ApiError::new(
                409,
                "linked_elsewhere",
                format!("{handle} is already linked to {}", other.display_name),
            ));
        }
        let p = self.person_mut(person)?;
        p.link_provider = provider.into();
        p.link_subject = subject.into();
        p.link_handle = handle.into();
        // DMs go to the same account unless they chose another.
        if p.mastodon.is_empty() {
            p.mastodon = handle.to_lowercase();
        }
        Ok(())
    }

    pub fn unlink(&mut self, person: u32) -> Result<(), ApiError> {
        let p = self.person_mut(person)?;
        if p.link_provider.is_empty() {
            return Err(ApiError::not_found("No Mastodon account is linked"));
        }
        p.link_provider.clear();
        p.link_subject.clear();
        p.link_handle.clear();
        p.auto_publish = false;
        Ok(())
    }

    /// Who signs in with this Mastodon account: only someone who linked it
    /// (from their settings, signed in with their PIN) first.
    pub fn person_for_identity(&self, provider: &str, subject: &str) -> Result<u32, ApiError> {
        self.people
            .iter()
            .find(|p| !p.disabled && p.link_provider == provider && p.link_subject == subject)
            .map(|p| p.id)
            .ok_or_else(|| {
                ApiError::new(
                    403,
                    "not_linked",
                    "That Mastodon account isn't linked to anyone here. Sign in with your PIN, then link it in Settings",
                )
            })
    }

    pub fn set_publishing(
        &mut self,
        person: u32,
        auto: Option<bool>,
        visibility: Option<&str>,
    ) -> Result<(), ApiError> {
        let visibility = visibility.map(check_visibility).transpose()?;
        let p = self.person_mut(person)?;
        if let Some(a) = auto {
            p.auto_publish = a;
        }
        if let Some(v) = visibility {
            p.publish_visibility = v;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(key: &str) -> StoredKind {
        catalog::builtin(key).unwrap().clone()
    }

    fn entry(kind: &str, visibility: &str, amount: u32, note: &str) -> StoredEntry {
        StoredEntry {
            id: 7,
            kind: kind.into(),
            visibility: visibility.into(),
            amount,
            note: note.into(),
            ..Default::default()
        }
    }

    #[test]
    fn the_truth_table_for_entries() {
        // Every built-in kind, every visibility: text only for merit,
        // content, non-sensitive kinds that aren't private.
        for k in catalog::builtins() {
            for v in ["public", "generic", "private"] {
                let e = entry(&k.key, v, k.default_amount, "");
                let text = publishable(&Item::Entry(&e, k), None);
                let allowed = v != "private" && k.content && k.polarity == "merit" && !sensitive(k);
                assert_eq!(text.is_some(), allowed, "{} {v}", k.key);
                if v == "generic" {
                    if let Some(t) = text {
                        assert!(
                            !t.contains(&k.name) || k.name == catalog::generic_label(k),
                            "{} leaks its name: {t}",
                            k.key
                        );
                    }
                }
            }
        }
        // A deleted entry never publishes.
        let mut e = entry("meditation", "public", 30, "");
        e.deleted = true;
        assert!(publishable(&Item::Entry(&e, &kind("meditation")), None).is_none());
    }

    #[test]
    fn what_an_entry_says() {
        let e = entry("meditation", "public", 30, "Quiet morning");
        assert_eq!(
            publishable(&Item::Entry(&e, &kind("meditation")), None).unwrap(),
            "🧘 Meditation · 30 min\n\nQuiet morning"
        );
        let e = entry("meditation", "generic", 30, "Quiet morning");
        assert_eq!(
            publishable(&Item::Entry(&e, &kind("meditation")), None).unwrap(),
            "🌿 A meditation practice"
        );
        let e = entry("daimoku", "public", 108, "");
        let t = publishable(
            &Item::Entry(&e, &kind("daimoku")),
            Some("https://meritledger.local/p/1"),
        )
        .unwrap();
        assert_eq!(t, "📿 Daimoku · 108×\n\nhttps://meritledger.local/p/1");
        // Never on: noise, demerits, repentance.
        for k in ["anger", "repentance", "broke_precept", "harsh_speech"] {
            let e = entry(k, "public", 0, "");
            assert!(
                publishable(&Item::Entry(&e, &kind(k)), None).is_none(),
                "{k}"
            );
        }
    }

    #[test]
    fn vow_posts_only_from_public_vows() {
        let post = StoredPost {
            id: 3,
            title: "Week one".into(),
            ..Default::default()
        };
        let mut vow = StoredVow {
            text: "Sit every morning".into(),
            visibility: "public".into(),
            ..Default::default()
        };
        let t = publishable(
            &Item::Post(
                &post,
                &vow,
                "**Kept** it, see [notes](https://x.example)\n\n- one\n- two",
            ),
            None,
        )
        .unwrap();
        assert_eq!(t, "🪷 Week one\n\nKept it, see notes (https://x.example)\n\n• one\n• two\n\n(On my vow: Sit every morning)");
        for v in ["generic", "private"] {
            vow.visibility = v.into();
            assert!(
                publishable(&Item::Post(&post, &vow, "x"), None).is_none(),
                "{v}"
            );
        }
    }

    #[test]
    fn one_person_per_mastodon_account() {
        let mut s = State::new(crate::state::Limits {
            pin_rounds: 2,
            ..crate::state::Limits::board()
        });
        let dad = s.setup("Dad", "1234", 0).unwrap();
        let kid = s
            .add_person("Kid", crate::state::MEMBER, "1111", "", 0)
            .unwrap();
        assert_eq!(
            s.person_for_identity("mastomini", "9").unwrap_err().code,
            "not_linked"
        );
        s.link_identity(dad, "mastomini", "9", "@Dad@mastomini.local")
            .unwrap();
        assert_eq!(s.person(dad).unwrap().mastodon, "@dad@mastomini.local");
        assert_eq!(s.person_for_identity("mastomini", "9").unwrap(), dad);
        assert_eq!(
            s.link_identity(kid, "mastomini", "9", "@Dad@mastomini.local")
                .unwrap_err()
                .code,
            "linked_elsewhere"
        );
        s.set_publishing(dad, Some(true), Some("public")).unwrap();
        assert!(s.set_publishing(dad, None, Some("direct")).is_err());
        s.unlink(dad).unwrap();
        assert!(!s.person(dad).unwrap().auto_publish);
        assert!(s.person_for_identity("mastomini", "9").is_err());
    }

    #[test]
    fn amounts() {
        assert_eq!(amount_label("minutes", 30), "30 min");
        assert_eq!(amount_label("minutes", 120), "2 h");
        assert_eq!(amount_label("count", 1080), "1,080×");
        assert_eq!(amount_label("count", 108), "108×");
    }

    #[test]
    fn long_text_is_cut_to_fit() {
        let e = entry("meditation", "public", 30, &"x".repeat(1_000));
        let t = publishable(
            &Item::Entry(&e, &kind("meditation")),
            Some("https://meritledger.local/p/1"),
        )
        .unwrap();
        assert!(t.chars().count() <= STATUS_MAX);
        assert!(t.ends_with("…\n\nhttps://meritledger.local/p/1"));
    }
}
