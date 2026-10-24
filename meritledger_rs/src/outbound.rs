//! Outbound DMs and email (`sprints/sprint10.md`), after gamematcher's
//! `outbound.rs`: the household Mastodon bot and Brevo transactional mail.
//!
//! - Everything that talks to the network runs on a worker thread
//!   ([`Outbox::run_once`]), never in a request handler.
//! - Jobs wait in a bounded queue persisted in the Kv, so a reboot or an
//!   internet outage loses nothing; a full queue refuses new work (the
//!   caller answers 409) rather than dropping old work.
//! - A DM carries an `Idempotency-Key`, so a retry never posts twice.
//!   Failures back off from 2 minutes to about an hour; after 12 tries or
//!   a day a job is marked failed.
//! - The bot's token is in the Kv. Creating a bot on mastomini uses the
//!   mastomini admin's login once, in RAM only. The Brevo key is in the Kv
//!   or comes from the firmware's build settings.
//! - **Publishing** (Sprint 11): statuses go out as the person, with the
//!   token their Mastodon link kept (`tk<person>` in the Kv, apart from the
//!   snapshot), or else as the household bot. Each published item's status
//!   id is remembered, so an edit or a delete on the board follows it there
//!   (best effort), and publishing the same item twice does nothing.
use crate::messages::{DeliveryView, OutboxEntry};
use crate::notify::Delivery;
use miniframework::fetch::{Fetch, Outgoing};
use miniframework::kv::Kv;
use miniframework::wire::{self, Format};
use miniframework::{message, ApiError};
use std::sync::{Arc, Mutex};

pub const QUEUE: usize = 32;
/// Published items remembered (for edits and deletes).
pub const PUBLISHED_KEPT: usize = 512;
pub const DAILY_LIMIT: u32 = 100;
const KEY_NAME: &str = "mmb-meritledger";
const BREVO: &str = "https://api.brevo.com/v3";
const NETWORK: &str = "can't reach Brevo: ";
const LIMIT: usize = 32 * 1024;
const GIVE_UP: u64 = 24 * 3600;

message! {
    /// The household bot, persisted in the Kv (`ob_bot`).
    pub struct StoredBot {
        /// `https://mastomini.local`
        1 instance: String,
        2 token: String,
        /// `@meritledger@mastomini.local`
        3 handle: String,
        /// `household-ca` (mastomini) or `public`.
        4 anchor: String,
        5 posts: u32,
        6 last_error: String,
    }
}

message! {
    pub struct StoredJob {
        1 id: u32,
        /// `dm` or `email`.
        2 kind: String,
        /// The DM's text, or the email's body.
        3 text: String,
        4 created: u64,
        5 attempts: u32,
        6 next_at: u64,
        /// `queued`, `sent` or `failed`.
        7 state: String,
        8 error: String,
        /// Email: address, name and subject.
        9 to: String,
        10 to_name: String,
        11 subject: String,
        /// `status` jobs: the board's item (`e12`, `p3`), and as whom
        /// (a person, or 0 for the bot).
        12 item: String,
        13 account: u32,
        14 visibility: String,
        /// `post`, `edit`, `delete`, or `revoke` (a link's token, in `text`).
        15 op: String,
        /// `revoke`: the sign-in provider.
        16 provider: String,
    }
}

message! {
    /// A person's Mastodon link with its kept token (`tk<person>`).
    pub struct StoredLink {
        /// The sign-in provider's id (`mastomini`).
        1 provider: String,
        /// `https://mastomini.local`
        2 instance: String,
        3 subject: String,
        4 handle: String,
        5 token: String,
        /// `household-ca` or `public`.
        6 anchor: String,
    }
}

message! {
    /// An item that went out: where its status lives.
    pub struct Published {
        1 item: String,
        2 account: u32,
        /// The status id on the instance.
        3 status: String,
        4 url: String,
    }
}

message! {
    /// Brevo settings, persisted in the Kv (`ob_mail`).
    pub struct StoredMail {
        1 api_key: String,
        2 sender_email: String,
        3 sender_name: String,
        4 sent: u32,
        5 last_error: String,
        /// UTC day of `sent_today`.
        6 day: u64,
        7 sent_today: u32,
    }
}

message! {
    pub struct StoredOutbox {
        1 jobs: Vec<StoredJob>,
        2 next_id: u32,
        /// Newest last; the oldest are forgotten past [`PUBLISHED_KEPT`].
        3 published: Vec<Published>,
    }
}

message! {
    pub struct StatusPost {
        1 status: String,
        /// `direct` for a DM.
        2 visibility: String,
    }
}

message! {
    /// A posted status: only what we keep.
    pub struct Status {
        1 id: String,
        2 url: String,
    }
}

message! {
    /// `verify_credentials`, and mastomini's admin replies: only what we read.
    pub struct Account {
        1 id: String,
        2 username: String,
        3 access_token: String,
        4 key: String,
    }
}

message! {
    pub struct BrevoSender {
        1 name: String,
        2 email: String,
        3 active: bool,
    }
}

message! {
    pub struct BrevoSenders {
        1 senders: Vec<BrevoSender>,
    }
}

message! {
    pub struct AdminLogin {
        1 username: String,
        2 password: String,
    }
}

message! {
    pub struct NewBot {
        1 username: String,
        2 display_name: String,
        3 password: String,
    }
}

message! {
    pub struct KeyRequest {
        1 name: String,
        2 password: String,
    }
}

/// A bot setup waiting for the worker (credentials in RAM only).
#[derive(Clone)]
pub enum Setup {
    /// Create the bot on a mastomini with its admin API.
    Mastomini {
        instance: String,
        admin: String,
        password: String,
        username: String,
        display_name: String,
    },
    /// An existing account's access token (any Mastodon).
    Token {
        instance: String,
        token: String,
        anchor: String,
    },
}

/// A Brevo setup waiting for the worker.
#[derive(Clone, Default)]
pub struct MailSetup {
    pub api_key: String,
    pub sender_email: String,
    pub built_in: bool,
}

#[derive(Default)]
struct Inner {
    bot: Option<StoredBot>,
    outbox: StoredOutbox,
    setup: Option<Setup>,
    setup_state: String,
    mail: Option<StoredMail>,
    mail_setup: Option<MailSetup>,
    mail_state: String,
    mail_built_in: bool,
    mail_retry_at: u64,
    generation: u64,
    /// The job the worker is sending right now (still `queued` until it's done).
    active: Option<u32>,
}

pub struct Outbox {
    kv: Arc<dyn Kv>,
    /// Trusts the household CA (mastomini).
    household: Arc<dyn Fetch>,
    /// Trusts public CAs (Brevo, mastodon.social).
    public: Arc<dyn Fetch>,
    inner: Mutex<Inner>,
    /// Mastodon sign-in, when the build has it: its network steps run on
    /// this worker too, and it revokes the tokens of unlinked accounts.
    oauth: Mutex<Option<Arc<miniframework::oauth::OAuth>>>,
}

fn host_of(base: &str) -> &str {
    base.split_once("://")
        .map_or(base, |(_, r)| r)
        .split(['/', ':'])
        .next()
        .unwrap_or("")
}

fn clean_instance(raw: &str) -> Result<String, ApiError> {
    let base = raw.trim().trim_end_matches('/').to_string();
    if !(base.starts_with("https://") || base.starts_with("http://")) || host_of(&base).is_empty() {
        return Err(ApiError::bad_request(
            "The instance address starts with https://",
        ));
    }
    Ok(base)
}

fn json<M: miniframework::Message>(m: &M) -> Vec<u8> {
    wire::to_vec(Format::Json, m).unwrap_or_default()
}

fn json_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Brevo's `POST /v3/smtp/email` body (its `textContent` key isn't a
/// Rust-shaped field name, so this one is written by hand).
pub fn brevo_body(
    sender_name: &str,
    sender_email: &str,
    to: &str,
    to_name: &str,
    subject: &str,
    text: &str,
) -> Vec<u8> {
    let mut out = String::from("{\"sender\":{\"name\":");
    json_string(&mut out, sender_name);
    out.push_str(",\"email\":");
    json_string(&mut out, sender_email);
    out.push_str("},\"to\":[{\"email\":");
    json_string(&mut out, to);
    if !to_name.is_empty() {
        out.push_str(",\"name\":");
        json_string(&mut out, to_name);
    }
    out.push_str("}],\"subject\":");
    json_string(&mut out, subject);
    out.push_str(",\"textContent\":");
    json_string(&mut out, text);
    out.push('}');
    out.into_bytes()
}

/// Builds a bot setup from the admin form, validating first.
pub fn setup_from(input: &crate::messages::BotSetup) -> Result<Setup, ApiError> {
    let instance = clean_instance(&input.instance)?;
    if !input.token.trim().is_empty() {
        let anchor = if host_of(&instance).ends_with(".local") {
            "household-ca"
        } else {
            "public"
        };
        return Ok(Setup::Token {
            instance,
            token: input.token.trim().into(),
            anchor: anchor.into(),
        });
    }
    let username = input.username.trim().trim_start_matches('@').to_lowercase();
    if username.is_empty()
        || username.len() > 30
        || !username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(ApiError::bad_request("Bot username: letters, digits and _"));
    }
    if input.admin.trim().is_empty() || input.password.is_empty() {
        return Err(ApiError::bad_request(
            "To create the bot, give mastomini's admin username and password (used once, never stored), or paste an access token",
        ));
    }
    Ok(Setup::Mastomini {
        instance,
        admin: input.admin.trim().into(),
        password: input.password.clone(),
        username,
        display_name: if input.display_name.trim().is_empty() {
            "Merit Ledger".into()
        } else {
            input.display_name.trim().into()
        },
    })
}

impl Outbox {
    pub fn open(kv: Arc<dyn Kv>, household: Arc<dyn Fetch>, public: Arc<dyn Fetch>) -> Self {
        let get = |key: &str| kv.get(key);
        let bot = get("ob_bot")
            .and_then(|b| wire::decode::<StoredBot>(Format::CborInt, &b).ok())
            .filter(|b| !b.token.is_empty());
        let outbox = get("ob_queue")
            .and_then(|b| wire::decode(Format::CborInt, &b).ok())
            .unwrap_or_default();
        let mail = get("ob_mail")
            .and_then(|b| wire::decode::<StoredMail>(Format::CborInt, &b).ok())
            .filter(|m| !m.api_key.is_empty());
        Self {
            kv,
            household,
            public,
            inner: Mutex::new(Inner {
                bot,
                outbox,
                mail,
                ..Default::default()
            }),
            oauth: Mutex::new(None),
        }
    }

    /// Runs `oauth`'s network steps on this worker, and lets unlinking
    /// revoke tokens.
    pub fn attach_oauth(&self, oauth: Arc<miniframework::oauth::OAuth>) {
        *self.oauth.lock().unwrap_or_else(|e| e.into_inner()) = Some(oauth);
    }

    fn link_key(person: u32) -> String {
        format!("tk{person}")
    }

    /// `person`'s Mastodon link (with its token), if any.
    pub fn link_of(&self, person: u32) -> Option<StoredLink> {
        self.kv
            .get(&Self::link_key(person))
            .and_then(|b| wire::decode::<StoredLink>(Format::CborInt, &b).ok())
            .filter(|l| !l.token.is_empty())
    }

    pub fn save_link(&self, person: u32, link: &StoredLink) -> Result<(), ApiError> {
        let bytes = wire::to_vec(Format::CborInt, link).map_err(ApiError::from)?;
        self.kv.set(&Self::link_key(person), &bytes).map_err(|e| {
            log::error!("saving a Mastodon link: {e}");
            ApiError::new(500, "storage_failed", "Could not save the link")
        })
    }

    /// Forgets `person`'s link and queues revoking its token.
    pub fn remove_link(&self, person: u32, now: u64) {
        let Some(link) = self.link_of(person) else {
            return;
        };
        let _ = self.kv.remove(&Self::link_key(person));
        let mut inner = self.lock();
        // Their queued posts can't go out as them any more.
        inner
            .outbox
            .jobs
            .retain(|j| !(j.kind == "status" && j.account == person && j.state == "queued"));
        let job = StoredJob {
            kind: "status".into(),
            op: "revoke".into(),
            provider: link.provider,
            text: link.token,
            account: person,
            ..Default::default()
        };
        self.push(&mut inner, job, now);
    }

    /// Where `item` was published, if it was.
    pub fn published(&self, item: &str) -> Option<Published> {
        self.lock()
            .outbox
            .published
            .iter()
            .find(|p| p.item == item)
            .cloned()
    }

    fn queued(inner: &Inner, item: &str, op: &str) -> bool {
        inner
            .outbox
            .jobs
            .iter()
            .any(|j| j.kind == "status" && j.item == item && j.op == op && j.state == "queued")
    }

    /// Queues publishing `item` as `account` (0 = the bot). False if it's
    /// already out (or on its way), nobody can post it, or the queue is full.
    pub fn publish(
        &self,
        item: &str,
        account: u32,
        text: String,
        visibility: &str,
        now: u64,
    ) -> bool {
        let can_post = if account == 0 {
            self.bot_ready()
        } else {
            self.link_of(account).is_some()
        };
        let mut inner = self.lock();
        if !can_post
            || inner.outbox.published.iter().any(|p| p.item == item)
            || Self::queued(&inner, item, "post")
        {
            return false;
        }
        let job = StoredJob {
            kind: "status".into(),
            op: "post".into(),
            item: item.into(),
            account,
            text,
            visibility: visibility.into(),
            ..Default::default()
        };
        self.push(&mut inner, job, now)
    }

    /// The account of `item`'s post if the worker is sending it right now.
    fn posting(inner: &Inner, item: &str) -> Option<u32> {
        let id = inner.active?;
        inner
            .outbox
            .jobs
            .iter()
            .find(|j| j.id == id && j.kind == "status" && j.item == item && j.op == "post")
            .map(|j| j.account)
    }

    /// The item changed on the board: change its status too (best effort).
    pub fn edit(&self, item: &str, text: String, now: u64) {
        let mut inner = self.lock();
        let published = inner
            .outbox
            .published
            .iter()
            .find(|p| p.item == item)
            .map(|p| p.account)
            .or_else(|| Self::posting(&inner, item));
        let Some(account) = published else {
            // Not out yet: the queued post takes the new text.
            if let Some(j) = inner.outbox.jobs.iter_mut().find(|j| {
                j.kind == "status" && j.item == item && j.op == "post" && j.state == "queued"
            }) {
                j.text = text;
                self.save(&inner);
            }
            return;
        };
        inner.outbox.jobs.retain(|j| {
            !(j.kind == "status" && j.item == item && j.op == "edit" && j.state == "queued")
        });
        // An edit runs after the post it follows (the queue is in order), and
        // finds the status id then.
        let job = StoredJob {
            kind: "status".into(),
            op: "edit".into(),
            item: item.into(),
            account,
            text,
            ..Default::default()
        };
        self.push(&mut inner, job, now);
    }

    /// The item is gone from the board (or no longer public): delete its
    /// status, or drop it from the queue if it hasn't gone out yet.
    pub fn unpublish(&self, item: &str, now: u64) {
        let mut inner = self.lock();
        let active = inner.active;
        inner.outbox.jobs.retain(|j| {
            Some(j.id) == active
                || !(j.kind == "status"
                    && j.item == item
                    && j.op != "delete"
                    && j.state == "queued")
        });
        let published = inner
            .outbox
            .published
            .iter()
            .find(|p| p.item == item)
            .map(|p| p.account)
            .or_else(|| Self::posting(&inner, item));
        let Some(account) = published else {
            self.save(&inner);
            return;
        };
        if Self::queued(&inner, item, "delete") {
            return;
        }
        let job = StoredJob {
            kind: "status".into(),
            op: "delete".into(),
            item: item.into(),
            account,
            ..Default::default()
        };
        self.push(&mut inner, job, now);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn save(&self, inner: &Inner) {
        let result = match &inner.bot {
            Some(bot) => self.kv.set(
                "ob_bot",
                &wire::to_vec(Format::CborInt, bot).unwrap_or_default(),
            ),
            None => self.kv.remove("ob_bot"),
        }
        .and_then(|_| {
            self.kv.set(
                "ob_queue",
                &wire::to_vec(Format::CborInt, &inner.outbox).unwrap_or_default(),
            )
        })
        .and_then(|_| match &inner.mail {
            Some(m) => self.kv.set(
                "ob_mail",
                &wire::to_vec(Format::CborInt, m).unwrap_or_default(),
            ),
            None => self.kv.remove("ob_mail"),
        });
        if let Err(e) = result {
            log::warn!("saving the outbox: {e}");
        }
    }

    /// Jobs that can still be queued before the queue is full of unsent work.
    pub fn room(&self) -> usize {
        let inner = self.lock();
        QUEUE
            - inner
                .outbox
                .jobs
                .iter()
                .filter(|j| j.state == "queued")
                .count()
    }

    pub fn bot_ready(&self) -> bool {
        self.lock().bot.is_some()
    }

    pub fn mail_ready(&self) -> bool {
        self.lock().mail.is_some()
    }

    /// Queues a delivery; false (and logged) when its channel isn't set up,
    /// today's email allowance is spent, or the queue is full of unsent work.
    pub fn send(&self, d: &Delivery, origin: &str, now: u64) -> bool {
        let link = if d.link.is_empty() {
            String::new()
        } else {
            format!("{origin}{}", d.link)
        };
        let mut inner = self.lock();
        let job = match d.channel {
            "dm" => {
                if inner.bot.is_none() {
                    return false;
                }
                let mut text = format!("{} {}", d.to, d.text);
                if !link.is_empty() {
                    text.push_str("\n\n");
                    text.push_str(&link);
                }
                StoredJob {
                    kind: "dm".into(),
                    text,
                    ..Default::default()
                }
            }
            _ => {
                let Some(m) = inner.mail.as_mut() else {
                    return false;
                };
                let day = now / 86_400;
                if m.day != day {
                    m.day = day;
                    m.sent_today = 0;
                }
                if m.sent_today >= DAILY_LIMIT {
                    log::warn!(
                        "email allowance for today is spent; not sending \"{}\"",
                        d.subject
                    );
                    return false;
                }
                m.sent_today += 1;
                let mut text = d.text.clone();
                if !link.is_empty() {
                    text.push_str("\n\n");
                    text.push_str(&link);
                }
                text.push_str("\n\n-- \nMerit Ledger, at home. Turn reminder emails off on your settings page.");
                StoredJob {
                    kind: "email".into(),
                    text,
                    to: d.to.clone(),
                    to_name: d.name.clone(),
                    subject: d.subject.clone(),
                    ..Default::default()
                }
            }
        };
        let kind = job.kind.clone();
        if self.push(&mut inner, job, now) {
            true
        } else {
            if kind == "email" {
                if let Some(m) = inner.mail.as_mut() {
                    m.sent_today -= 1;
                }
            }
            log::warn!("outbox full; a {kind} wasn't queued");
            false
        }
    }

    fn push(&self, inner: &mut Inner, job: StoredJob, now: u64) -> bool {
        // Finished entries make room, oldest first; queued ones never do.
        while inner.outbox.jobs.len() >= QUEUE {
            match inner.outbox.jobs.iter().position(|j| j.state != "queued") {
                Some(i) => {
                    inner.outbox.jobs.remove(i);
                }
                None => return false,
            }
        }
        let id = inner.outbox.next_id.max(1);
        inner.outbox.next_id = id + 1;
        inner.outbox.jobs.push(StoredJob {
            id,
            created: now,
            state: "queued".into(),
            ..job
        });
        self.save(inner);
        true
    }

    pub fn view(&self) -> DeliveryView {
        let inner = self.lock();
        let bot = inner.bot.clone().unwrap_or_default();
        let mail = inner.mail.clone().unwrap_or_default();
        DeliveryView {
            bot_handle: bot.handle,
            bot_instance: bot.instance,
            bot_setup: inner.setup_state.clone(),
            bot_error: bot.last_error,
            mail_sender: if mail.sender_email.is_empty() {
                String::new()
            } else {
                format!("{} <{}>", mail.sender_name, mail.sender_email)
            },
            mail_setup: inner.mail_state.clone(),
            mail_error: mail.last_error,
            sent_today: mail.sent_today,
            daily_limit: DAILY_LIMIT,
            mail_built_in: inner.mail_built_in,
            jobs: inner
                .outbox
                .jobs
                .iter()
                .rev()
                .map(|j| OutboxEntry {
                    id: j.id,
                    kind: j.kind.clone(),
                    text: if j.kind == "email" {
                        format!("{} → {}", j.subject, j.to)
                    } else {
                        j.text.clone()
                    }
                    .chars()
                    .take(100)
                    .collect(),
                    state: j.state.clone(),
                    attempts: j.attempts,
                    error: j.error.clone(),
                    created: j.created,
                    next_at: j.next_at,
                })
                .collect(),
        }
    }

    pub fn begin_setup(&self, setup: Setup) -> Result<(), ApiError> {
        let mut inner = self.lock();
        if inner.setup.is_some() || inner.setup_state == "working" {
            return Err(ApiError::new(409, "busy", "A setup is already running"));
        }
        inner.generation += 1;
        inner.setup = Some(setup);
        inner.setup_state = "working".into();
        Ok(())
    }

    pub fn remove_bot(&self) {
        let mut inner = self.lock();
        inner.generation += 1;
        inner.setup = None;
        inner.bot = None;
        inner.setup_state.clear();
        inner
            .outbox
            .jobs
            .retain(|j| j.state != "queued" || j.kind == "email");
        self.save(&inner);
    }

    pub fn begin_mail_setup(&self, setup: MailSetup) -> Result<(), ApiError> {
        let key = setup.api_key.trim();
        if key.starts_with("xsmtpsib-") {
            return Err(ApiError::bad_request(
                "That's a Brevo SMTP key; the board needs an API key (xkeysib-...): Brevo → SMTP & API → API keys",
            ));
        }
        if key.len() < 20 || key.contains(char::is_whitespace) {
            return Err(ApiError::bad_request(
                "Paste the whole Brevo API key (xkeysib-...)",
            ));
        }
        let mut inner = self.lock();
        if setup.built_in && inner.mail.is_some() {
            return Ok(());
        }
        if inner.mail_setup.is_some() || inner.mail_state == "working" {
            return Err(ApiError::new(409, "busy", "A setup is already running"));
        }
        inner.mail_built_in = setup.built_in;
        inner.mail_setup = Some(setup);
        inner.mail_state = "working".into();
        Ok(())
    }

    pub fn remove_mail(&self) {
        let mut inner = self.lock();
        inner.mail = None;
        inner.mail_built_in = false;
        inner.mail_state.clear();
        inner
            .outbox
            .jobs
            .retain(|j| j.state != "queued" || j.kind != "email");
        self.save(&inner);
    }

    /// Retries a failed job now.
    pub fn retry(&self, id: u32) -> Result<(), ApiError> {
        let mut inner = self.lock();
        let job = inner
            .outbox
            .jobs
            .iter_mut()
            .find(|j| j.id == id)
            .ok_or_else(|| ApiError::not_found("No such delivery"))?;
        if job.state != "failed" {
            return Err(ApiError::new(
                409,
                "not_failed",
                "Only failed deliveries can be retried",
            ));
        }
        job.state = "queued".into();
        job.next_at = 0;
        job.attempts = 0;
        self.save(&inner);
        Ok(())
    }

    fn fetch(&self, anchor: &str) -> &dyn Fetch {
        if anchor == "household-ca" {
            self.household.as_ref()
        } else {
            self.public.as_ref()
        }
    }

    /// Does one network step: a pending setup, else the next due job.
    /// Returns whether it did anything. Runs on the worker thread.
    pub fn run_once(&self, now: u64) -> bool {
        let oauth = self.oauth.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if oauth.as_ref().is_some_and(|o| o.run_once()) {
            return true;
        }
        if now == 0 {
            // Retry times are wall-clock: wait for the clock.
            return false;
        }
        let (setup, generation) = {
            let mut inner = self.lock();
            (inner.setup.take(), inner.generation)
        };
        if let Some(setup) = setup {
            let outcome = self.run_setup(setup);
            let mut inner = self.lock();
            if inner.generation != generation {
                return true;
            }
            match outcome {
                Ok(bot) => {
                    inner.setup_state = format!("ready: {}", bot.handle);
                    log::info!("Mastodon bot ready: {}", bot.handle);
                    inner.bot = Some(bot);
                }
                Err(e) => inner.setup_state = format!("failed: {e}"),
            }
            self.save(&inner);
            return true;
        }
        let mail_setup = {
            let mut inner = self.lock();
            if now >= inner.mail_retry_at {
                inner.mail_setup.take()
            } else {
                None
            }
        };
        if let Some(setup) = mail_setup {
            let outcome = self.run_mail_setup(&setup);
            let mut inner = self.lock();
            match outcome {
                Ok(m) => {
                    inner.mail_state = format!("ready: {} <{}>", m.sender_name, m.sender_email);
                    log::info!("email ready: from {}", m.sender_email);
                    inner.mail = Some(m);
                }
                Err(e) if e.starts_with(NETWORK) => {
                    log::info!("email setup: {e}; trying again in 30 s");
                    inner.mail_setup = Some(setup);
                    inner.mail_retry_at = now + 30;
                }
                Err(e) => {
                    log::warn!("email setup: {e}");
                    inner.mail_state = format!("failed: {e}");
                }
            }
            self.save(&inner);
            return true;
        }
        let (bot, mail, job) = {
            let mut inner = self.lock();
            let ready = |j: &&StoredJob| {
                j.state == "queued"
                    && j.next_at <= now
                    && match j.kind.as_str() {
                        "email" => inner.mail.is_some(),
                        "status" => j.account != 0 || j.op == "revoke" || inner.bot.is_some(),
                        _ => inner.bot.is_some(),
                    }
            };
            let Some(job) = inner.outbox.jobs.iter().find(ready).cloned() else {
                return false;
            };
            inner.active = Some(job.id);
            (inner.bot.clone(), inner.mail.clone(), job)
        };
        let mut posted: Option<Status> = None;
        let result = match (job.kind.as_str(), &bot, &mail) {
            ("email", _, Some(m)) => self.send_email(m, &job),
            ("dm", Some(b), _) => self.post(b, &job),
            ("status", _, _) => self.status(&job, bot.as_ref(), oauth.as_deref()).map(|s| {
                posted = s;
            }),
            _ => {
                self.lock().active = None;
                return false;
            }
        };
        let mut inner = self.lock();
        inner.active = None;
        if let Some(j) = inner.outbox.jobs.iter_mut().find(|j| j.id == job.id) {
            j.attempts += 1;
            match &result {
                Ok(()) => {
                    j.state = "sent".into();
                    j.error.clear();
                    if j.op == "revoke" {
                        // The token has no business staying in the queue.
                        j.text.clear();
                    }
                }
                Err(e) => {
                    j.error = e.clone();
                    j.next_at = now + (60u64 << j.attempts.min(6));
                    if now.saturating_sub(j.created) > GIVE_UP || j.attempts >= 12 {
                        j.state = "failed".into();
                    }
                }
            }
        }
        if job.kind == "status" && result.is_ok() {
            match job.op.as_str() {
                "post" => {
                    if let Some(s) = posted {
                        let published = &mut inner.outbox.published;
                        published.retain(|p| p.item != job.item);
                        published.push(Published {
                            item: job.item.clone(),
                            account: job.account,
                            status: s.id,
                            url: s.url,
                        });
                        if published.len() > PUBLISHED_KEPT {
                            published.remove(0);
                        }
                    }
                }
                "delete" => inner.outbox.published.retain(|p| p.item != job.item),
                _ => {}
            }
        }
        if job.kind == "email" {
            if let Some(m) = inner.mail.as_mut() {
                match &result {
                    Ok(()) => {
                        m.sent += 1;
                        m.last_error.clear();
                    }
                    Err(e) => m.last_error = e.clone(),
                }
            }
        } else if let Some(b) = inner.bot.as_mut() {
            match &result {
                Ok(()) => {
                    b.posts += 1;
                    b.last_error.clear();
                }
                Err(e) => b.last_error = e.clone(),
            }
        }
        self.save(&inner);
        true
    }

    /// A `status` job: post, edit or delete as the person (their link) or
    /// the bot, or revoke an unlinked token.
    fn status(
        &self,
        job: &StoredJob,
        bot: Option<&StoredBot>,
        oauth: Option<&miniframework::oauth::OAuth>,
    ) -> Result<Option<Status>, String> {
        if job.op == "revoke" {
            let provider = crate::api::default_instances()
                .into_iter()
                .find(|p| p.id == job.provider)
                .or_else(|| crate::api::public_provider(&job.provider).ok());
            return match (oauth, provider) {
                (Some(o), Some(provider)) => o
                    .revoke_provider(&provider, &job.text)
                    .map(|_| None)
                    .map_err(|e| e.to_string()),
                (Some(o), None) => o
                    .revoke(&job.provider, &job.text)
                    .map(|_| None)
                    .map_err(|e| e.to_string()),
                (None, _) => Ok(None),
            };
        }
        let (instance, token, anchor) = if job.account == 0 {
            let b = bot.ok_or("no bot")?;
            (b.instance.clone(), b.token.clone(), b.anchor.clone())
        } else {
            let l = self
                .link_of(job.account)
                .ok_or("their Mastodon account isn't linked any more")?;
            (l.instance, l.token, l.anchor)
        };
        let published = self.published(&job.item);
        match job.op.as_str() {
            "post" => {
                let body = json(&StatusPost {
                    status: job.text.clone(),
                    visibility: job.visibility.clone(),
                });
                let key = format!("meritledger-{}-{}", job.id, job.created);
                let reply = self.call_keyed(
                    &anchor,
                    "POST",
                    &format!("{instance}/api/v1/statuses"),
                    &token,
                    &body,
                    &key,
                )?;
                let status: Status =
                    wire::decode(Format::Json, &reply).map_err(|e| e.to_string())?;
                Ok(Some(status))
            }
            "edit" => {
                let Some(p) = published else { return Ok(None) };
                let body = json(&StatusPost {
                    status: job.text.clone(),
                    visibility: String::new(),
                });
                self.call(
                    &anchor,
                    "PUT",
                    &format!("{instance}/api/v1/statuses/{}", p.status),
                    &token,
                    &body,
                )?;
                Ok(None)
            }
            "delete" => {
                let Some(p) = published else { return Ok(None) };
                match self.call(
                    &anchor,
                    "DELETE",
                    &format!("{instance}/api/v1/statuses/{}", p.status),
                    &token,
                    &[],
                ) {
                    Ok(_) => Ok(None),
                    // Already gone there: done here too.
                    Err(e) if e.contains("HTTP 404") => Ok(None),
                    Err(e) => Err(e),
                }
            }
            other => Err(format!("unknown status job {other}")),
        }
    }

    fn post(&self, bot: &StoredBot, job: &StoredJob) -> Result<(), String> {
        let body = json(&StatusPost {
            status: job.text.clone(),
            visibility: "direct".into(),
        });
        let bearer = format!("Bearer {}", bot.token);
        let key = format!("meritledger-{}-{}", job.id, job.created);
        let reply = self
            .fetch(&bot.anchor)
            .send(&Outgoing {
                method: "POST",
                url: &format!("{}/api/v1/statuses", bot.instance),
                headers: &[
                    ("Authorization", &bearer),
                    ("Content-Type", "application/json"),
                    ("Accept", "application/json"),
                    ("Idempotency-Key", &key),
                ],
                body: &body,
                limit: LIMIT,
            })
            .map_err(|e| e.to_string())?;
        if reply.status != 200 {
            return Err(format!("HTTP {}", reply.status));
        }
        Ok(())
    }

    fn send_email(&self, m: &StoredMail, job: &StoredJob) -> Result<(), String> {
        let body = brevo_body(
            &m.sender_name,
            &m.sender_email,
            &job.to,
            &job.to_name,
            &job.subject,
            &job.text,
        );
        let reply = self
            .public
            .send(&Outgoing {
                method: "POST",
                url: &format!("{BREVO}/smtp/email"),
                headers: &[
                    ("api-key", &m.api_key),
                    ("Content-Type", "application/json"),
                    ("Accept", "application/json"),
                ],
                body: &body,
                limit: LIMIT,
            })
            .map_err(|e| e.to_string())?;
        if !(200..300).contains(&reply.status) {
            let detail: String = String::from_utf8_lossy(&reply.body)
                .chars()
                .take(160)
                .collect();
            return Err(format!("Brevo HTTP {}: {detail}", reply.status));
        }
        Ok(())
    }

    fn run_mail_setup(&self, setup: &MailSetup) -> Result<StoredMail, String> {
        let reply = self
            .public
            .send(&Outgoing {
                method: "GET",
                url: &format!("{BREVO}/senders"),
                headers: &[
                    ("api-key", setup.api_key.trim()),
                    ("Accept", "application/json"),
                ],
                body: &[],
                limit: LIMIT,
            })
            .map_err(|e| format!("{NETWORK}{e}"))?;
        if reply.status == 401 {
            return Err("Brevo doesn't know that key".into());
        }
        if reply.status != 200 {
            return Err(format!("Brevo said HTTP {}", reply.status));
        }
        let senders: BrevoSenders =
            wire::decode(Format::Json, &reply.body).map_err(|e| e.to_string())?;
        let want = setup.sender_email.trim().to_lowercase();
        let sender = senders
            .senders
            .iter()
            .filter(|s| s.active)
            .find(|s| want.is_empty() || s.email.to_lowercase() == want)
            .ok_or_else(|| {
                if want.is_empty() {
                    "The Brevo account has no verified sender yet (Brevo → Senders)".to_string()
                } else {
                    format!("{want} isn't a verified sender on that Brevo account")
                }
            })?;
        Ok(StoredMail {
            api_key: setup.api_key.trim().into(),
            sender_email: sender.email.clone(),
            sender_name: "Merit Ledger".into(),
            ..Default::default()
        })
    }

    fn call(
        &self,
        anchor: &str,
        method: &str,
        url: &str,
        bearer: &str,
        body: &[u8],
    ) -> Result<Vec<u8>, String> {
        self.call_keyed(anchor, method, url, bearer, body, "")
    }

    fn call_keyed(
        &self,
        anchor: &str,
        method: &str,
        url: &str,
        bearer: &str,
        body: &[u8],
        idempotency: &str,
    ) -> Result<Vec<u8>, String> {
        let auth = format!("Bearer {bearer}");
        let mut headers = vec![
            ("Accept", "application/json"),
            ("Content-Type", "application/json"),
        ];
        if !bearer.is_empty() {
            headers.push(("Authorization", &auth));
        }
        if !idempotency.is_empty() {
            headers.push(("Idempotency-Key", idempotency));
        }
        let reply = self
            .fetch(anchor)
            .send(&Outgoing {
                method,
                url,
                headers: &headers,
                body,
                limit: LIMIT,
            })
            .map_err(|e| e.to_string())?;
        if !(200..300).contains(&reply.status) {
            let detail: String = String::from_utf8_lossy(&reply.body)
                .chars()
                .take(160)
                .collect();
            return Err(format!(
                "{} → HTTP {}: {detail}",
                url.rsplit_once("/api/").map_or(url, |(_, p)| p),
                reply.status
            ));
        }
        Ok(reply.body)
    }

    fn verify(&self, instance: &str, anchor: &str, token: &str) -> Result<String, String> {
        let me = self.call(
            anchor,
            "GET",
            &format!("{instance}/api/v1/accounts/verify_credentials"),
            token,
            &[],
        )?;
        let me: Account = wire::decode(Format::Json, &me).map_err(|e| e.to_string())?;
        Ok(format!("@{}@{}", me.username, host_of(instance)))
    }

    /// The password for the bot this board creates on mastomini: made once
    /// and kept, so a retried setup finds its bot again.
    fn bot_secret(&self) -> Result<String, String> {
        if let Some(saved) = self
            .kv
            .get("ob_secret")
            .and_then(|b| String::from_utf8(b).ok())
        {
            if !saved.is_empty() {
                return Ok(saved);
            }
        }
        let secret = miniframework::session::random_token().map_err(|e| e.to_string())?;
        self.kv
            .set("ob_secret", secret.as_bytes())
            .map_err(|e| format!("saving the bot's password: {e}"))?;
        Ok(secret)
    }

    fn run_setup(&self, setup: Setup) -> Result<StoredBot, String> {
        let bot = |instance: String, token: String, handle: String, anchor: String| StoredBot {
            instance,
            token,
            handle,
            anchor,
            ..Default::default()
        };
        match setup {
            Setup::Token {
                instance,
                token,
                anchor,
            } => {
                let handle = self.verify(&instance, &anchor, &token)?;
                Ok(bot(instance, token, handle, anchor))
            }
            Setup::Mastomini {
                instance,
                admin,
                password,
                username,
                display_name,
            } => {
                // mastomini's admin API, as mastomini-bots' enrollment uses it.
                let anchor = "household-ca";
                let login = self.call(
                    anchor,
                    "POST",
                    &format!("{instance}/api/mastomini/v1/admin/bots/login"),
                    "",
                    &json(&AdminLogin {
                        username: admin,
                        password,
                    }),
                )?;
                let admin_token = wire::decode::<Account>(Format::Json, &login)
                    .map_err(|e| e.to_string())?
                    .access_token;
                if admin_token.is_empty() {
                    return Err("mastomini sent no admin token".into());
                }
                // mastomini answers a repeated create (same name and password)
                // with the same bot, so the password is saved first: a setup
                // that fails halfway is simply run again.
                let secret = self.bot_secret()?;
                let created = self
                    .call(
                        anchor,
                        "POST",
                        &format!("{instance}/api/mastomini/v1/admin/bots"),
                        &admin_token,
                        &json(&NewBot {
                            username: username.clone(),
                            display_name,
                            password: secret.clone(),
                        }),
                    )
                    .map_err(|e| {
                        if e.contains("HTTP 401") {
                            format!("@{username} already exists on mastomini as a bot this board didn't make. Delete it there, or pick another name")
                        } else {
                            e
                        }
                    })?;
                let id = wire::decode::<Account>(Format::Json, &created)
                    .map_err(|e| e.to_string())?
                    .id;
                let key = self.call(
                    anchor,
                    "POST",
                    &format!("{instance}/api/mastomini/v1/admin/bots/{id}/api_keys"),
                    &admin_token,
                    &json(&KeyRequest {
                        name: KEY_NAME.into(),
                        password: secret,
                    }),
                )?;
                let token = wire::decode::<Account>(Format::Json, &key)
                    .map_err(|e| e.to_string())?
                    .key;
                let handle = self.verify(&instance, anchor, &token)?;
                Ok(bot(instance, token, handle, anchor.into()))
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use miniframework::fetch::Fetched;
    use miniframework::kv::MemKv;
    use std::io;

    /// One request: method, URL, body and headers.
    pub type Sent = (String, String, String, Vec<(String, String)>);

    /// A stand-in for mastomini and Brevo: records requests, answers from a script.
    #[derive(Default)]
    pub struct Fake {
        pub sent: Mutex<Vec<Sent>>,
        pub fail: Mutex<bool>,
    }

    impl Fetch for Fake {
        fn get(&self, _: &str, _: &str, _: usize) -> io::Result<Fetched> {
            Err(io::Error::other("GET not used"))
        }

        fn send(&self, r: &Outgoing<'_>) -> io::Result<Fetched> {
            self.sent.lock().unwrap().push((
                r.method.into(),
                r.url.into(),
                String::from_utf8_lossy(r.body).into_owned(),
                r.headers
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            ));
            if *self.fail.lock().unwrap() {
                return Err(io::Error::other("network down"));
            }
            let body = if r.url.ends_with("/verify_credentials") {
                r#"{"id":"9","username":"meritledger"}"#
            } else if r.url.ends_with("/senders") {
                r#"{"senders":[{"name":"Matt","email":"matt@example.com","active":true}]}"#
            } else if r.url.ends_with("/admin/bots/login") {
                r#"{"access_token":"admintok"}"#
            } else if r.url.ends_with("/admin/bots") {
                r#"{"id":"42"}"#
            } else if r.url.ends_with("/api_keys") {
                r#"{"key":"bottoken"}"#
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

    pub fn ready(fake: Arc<Fake>, kv: Arc<dyn Kv>) -> Outbox {
        let o = Outbox::open(kv, fake.clone(), fake);
        o.begin_setup(Setup::Token {
            instance: "https://mastomini.local".into(),
            token: "tok".into(),
            anchor: "household-ca".into(),
        })
        .unwrap();
        o.begin_mail_setup(MailSetup {
            api_key: "xkeysib-0123456789abcdef".into(),
            ..Default::default()
        })
        .unwrap();
        assert!(o.run_once(1) && o.run_once(1));
        o
    }

    fn dm(text: &str) -> Delivery {
        Delivery {
            channel: "dm",
            to: "@kid@mastomini.local".into(),
            name: "Kid".into(),
            subject: String::new(),
            text: text.into(),
            link: "/events/3".into(),
        }
    }

    #[test]
    fn a_dm_and_an_email_go_out_as_the_mock_expects() {
        let fake = Arc::new(Fake::default());
        let o = ready(fake.clone(), Arc::new(MemKv::default()));
        assert_eq!(o.view().bot_handle, "@meritledger@mastomini.local");
        assert!(o.view().mail_sender.contains("matt@example.com"));
        assert!(o.send(
            &dm("Dad invited you to Uposatha dinner"),
            "https://meritledger.local",
            10
        ));
        let email = Delivery {
            channel: "email",
            to: "kid@home.example".into(),
            subject: "Reminder: Sit".into(),
            text: "Kid,\n\nSit".into(),
            link: String::new(),
            name: "Kid".into(),
        };
        assert!(o.send(&email, "https://meritledger.local", 10));
        fake.sent.lock().unwrap().clear();
        assert!(o.run_once(10) && o.run_once(10));
        assert!(!o.run_once(10), "nothing left");
        let sent = fake.sent.lock().unwrap();
        let (method, url, body, headers) = &sent[0];
        assert_eq!(
            (method.as_str(), url.as_str()),
            ("POST", "https://mastomini.local/api/v1/statuses")
        );
        assert!(body.contains(r#""visibility":"direct""#));
        assert!(body.contains("@kid@mastomini.local Dad invited you to Uposatha dinner"));
        assert!(body.contains("https://meritledger.local/events/3"));
        assert!(headers
            .iter()
            .any(|(k, v)| k == "Authorization" && v == "Bearer tok"));
        assert!(headers.iter().any(|(k, _)| k == "Idempotency-Key"));
        let (_, url, body, _) = &sent[1];
        assert_eq!(url, "https://api.brevo.com/v3/smtp/email");
        assert!(body.contains(r#""to":[{"email":"kid@home.example","name":"Kid"}]"#));
        assert!(body.contains(r#""subject":"Reminder: Sit""#));
        assert!(body.contains(r#""sender":{"name":"Merit Ledger","email":"matt@example.com"}"#));
        assert!(o.view().jobs.iter().all(|j| j.state == "sent"));
    }

    #[test]
    fn a_full_queue_refuses_and_retries_keep_their_key() {
        let fake = Arc::new(Fake::default());
        let kv: Arc<dyn Kv> = Arc::new(MemKv::default());
        let o = ready(fake.clone(), kv.clone());
        *fake.fail.lock().unwrap() = true;
        for i in 0..QUEUE {
            assert!(o.send(&dm(&format!("n{i}")), "", 10), "{i}");
        }
        assert_eq!(o.room(), 0);
        assert!(!o.send(&dm("one too many"), "", 10));
        // A failure backs off and keeps the job.
        fake.sent.lock().unwrap().clear();
        for _ in 0..QUEUE {
            assert!(o.run_once(10));
        }
        assert!(!o.run_once(11), "every job is backing off");
        let first_key = |fake: &Fake| {
            fake.sent.lock().unwrap()[0]
                .3
                .iter()
                .find(|(k, _)| k == "Idempotency-Key")
                .unwrap()
                .1
                .clone()
        };
        let key = first_key(&fake);
        // It survives a reboot and is retried with the same key.
        let o = Outbox::open(kv, fake.clone(), fake.clone());
        assert_eq!(o.room(), 0);
        *fake.fail.lock().unwrap() = false;
        fake.sent.lock().unwrap().clear();
        assert!(o.run_once(10 + 200));
        assert_eq!(first_key(&fake), key);
    }

    #[test]
    fn creates_a_bot_on_mastomini_without_keeping_the_password() {
        let fake = Arc::new(Fake::default());
        let kv: Arc<dyn Kv> = Arc::new(MemKv::default());
        let o = Outbox::open(kv.clone(), fake.clone(), fake.clone());
        let setup = setup_from(&crate::messages::BotSetup {
            instance: "https://mastomini.local/".into(),
            admin: "admin".into(),
            password: "hunter2".into(),
            username: "meritledger".into(),
            ..Default::default()
        })
        .unwrap();
        o.begin_setup(setup).unwrap();
        assert!(o.run_once(1));
        assert!(o.bot_ready());
        assert_eq!(o.view().bot_setup, "ready: @meritledger@mastomini.local");
        let saved = String::from_utf8_lossy(&kv.get("ob_bot").unwrap()).into_owned();
        assert!(saved.contains("bottoken") && !saved.contains("hunter2"));
        assert!(setup_from(&crate::messages::BotSetup {
            instance: "mastomini.local".into(),
            token: "x".into(),
            ..Default::default()
        })
        .is_err());
    }

    #[test]
    fn a_delete_during_the_post_waits_for_it() {
        let fake = Arc::new(Fake::default());
        let o = ready(fake.clone(), Arc::new(MemKv::default()));
        assert!(o.publish("e1", 0, "Sat".into(), "unlisted", 10));
        // Not sent yet: deleting just drops it.
        o.unpublish("e1", 10);
        assert!(o.view().jobs.is_empty());
        // Being sent right now: the delete queues behind it.
        assert!(o.publish("e1", 0, "Sat".into(), "unlisted", 10));
        let id = o.view().jobs[0].id;
        o.lock().active = Some(id);
        o.unpublish("e1", 10);
        let ops: Vec<String> = o.lock().outbox.jobs.iter().map(|j| j.op.clone()).collect();
        assert_eq!(ops, ["post", "delete"]);
        o.lock().active = None;
        fake.sent.lock().unwrap().clear();
        while o.run_once(10) {}
        let sent = fake.sent.lock().unwrap();
        assert!(sent
            .iter()
            .any(|(m, u, _, _)| m == "POST" && u.ends_with("/api/v1/statuses")));
        assert!(sent
            .iter()
            .any(|(m, u, _, _)| m == "DELETE" && u.ends_with("/api/v1/statuses/1")));
        drop(sent);
        assert!(o.published("e1").is_none());
    }

    #[test]
    fn brevo_wants_an_api_key() {
        let o = Outbox::open(
            Arc::new(MemKv::default()),
            Arc::new(Fake::default()),
            Arc::new(Fake::default()),
        );
        let smtp = MailSetup {
            api_key: "xsmtpsib-0123456789abcdef".into(),
            ..Default::default()
        };
        assert_eq!(o.begin_mail_setup(smtp).unwrap_err().status, 400);
    }
}
