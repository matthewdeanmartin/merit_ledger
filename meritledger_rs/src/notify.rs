//! Notifications, reminders and calls to service (`sprints/sprint10.md`).
//!
//! - **The bell:** each person has a short ring of notifications, newest
//!   first. While one is unread, another of the same kind about the same
//!   thing bumps its count instead of adding a line, and only a *new* line
//!   is sent on: ten quick answers to an event are one DM, not ten.
//! - **Channels:** the bell always; a DM from the household Mastodon bot
//!   for notifications; an email for reminders. Each person can turn the
//!   DMs or the emails off. New lines leave here as [`Delivery`]s, which
//!   `api.rs` hands to the outbox once the change is saved.
//! - **Reminders** fire on the board's minute tick with a synced clock.
//!   After a reboot, one missed by less than an hour still fires; older
//!   ones are skipped, not sent late.
use crate::ledger::{Actor, Viewer};
use crate::messages::{
    CallInput, CallView, NotificationView, ReminderInput, StoredCall, StoredNotification,
    StoredReminder,
};
use crate::state::{State, GUEST};
use crate::tz::{Date, Tz};
use miniframework::ApiError;

pub const CALL: &str = "call";
pub const INVITE: &str = "invite";
pub const RSVP: &str = "rsvp";
pub const CHANGED: &str = "changed";
pub const CANCELLED: &str = "cancelled";
pub const REMINDER: &str = "reminder";
pub const OBSERVANCE: &str = "observance";
pub const REJOICE: &str = "rejoice";

/// A reminder this late after a reboot still fires; later ones are skipped.
pub const CATCH_UP: u64 = 3600;
/// Observance notices go out at this local time.
pub const OBSERVANCE_AT: u32 = 7 * 60;
/// Calls to service kept (and shown on the board) for this long.
pub const CALL_SHOWN: u64 = 3 * 3600;
const CALLS_KEPT: usize = 8;

/// A message for the outbox: a DM to a Mastodon handle or an email.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delivery {
    /// `dm` or `email`.
    pub channel: &'static str,
    /// The handle (`@matt@mastomini.local`) or the address.
    pub to: String,
    pub name: String,
    pub subject: String,
    pub text: String,
    /// A page on the board ("/events/3"), appended as a link.
    pub link: String,
}

/// `matt@mastomini.local` or `@matt@mastomini.local` -> `@matt@mastomini.local`;
/// "" clears it.
pub fn normalize_handle(raw: &str) -> Result<String, ApiError> {
    let h = raw.trim().trim_start_matches('@').to_lowercase();
    if h.is_empty() {
        return Ok(String::new());
    }
    let ok = h.split_once('@').is_some_and(|(user, host)| {
        !user.is_empty()
            && user
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
            && host.contains('.')
            && host
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    });
    if ok && h.len() <= 100 {
        Ok(format!("@{h}"))
    } else {
        Err(ApiError::bad_request(
            "A Mastodon handle looks like @name@mastomini.local",
        ))
    }
}

/// `HH:MM` -> minutes after midnight.
pub fn parse_hhmm(s: &str) -> Option<u32> {
    let (h, m) = s.split_once(':')?;
    if h.len() != 2 || m.len() != 2 {
        return None;
    }
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// UTC seconds of household-local `date` at `minutes` after midnight.
pub fn local_instant(tz: &Tz, date: Date, minutes: u32) -> u64 {
    let naive = date.days() * 86_400 + i64::from(minutes) * 60;
    // The offset depends on the instant; two rounds settle it except in the
    // skipped hour of a DST change, where an hour either way is fine.
    let guess = naive - tz.offset_at(naive);
    (naive - tz.offset_at(guess)).max(0) as u64
}

/// "Saturday 18 October, 19:00" in the household zone.
pub fn when(tz: &Tz, utc: u64) -> String {
    let t = utc as i64;
    let local = t + tz.offset_at(t);
    let date = tz.local_date(t);
    let minutes = local.rem_euclid(86_400) / 60;
    const DAYS: [&str; 7] = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ];
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    format!(
        "{} {} {}, {:02}:{:02}",
        DAYS[date.weekday() as usize],
        date.day,
        MONTHS[(date.month - 1) as usize],
        minutes / 60,
        minutes % 60
    )
}

impl State {
    /// Tells `to` about `kind` on `subject`, done by `actor` (0 = the
    /// board). Nobody is told about their own doing, except reminders.
    pub fn notify(&mut self, to: u32, kind: &str, subject: u32, actor: u32, text: &str, now: u64) {
        if to == actor && kind != REMINDER {
            return;
        }
        let limit = self.limits.notifications.max(1);
        let id = self.next_notification;
        let Some(person) = self.people.iter_mut().find(|p| p.id == to && !p.disabled) else {
            return;
        };
        let list = &mut person.notifications;
        let coalesces = matches!(kind, RSVP | REJOICE | CHANGED);
        if coalesces {
            if let Some(i) = list
                .iter()
                .position(|n| !n.read && n.kind == kind && n.subject == subject)
            {
                let mut n = list.remove(i);
                n.count += 1;
                n.actor = actor;
                n.created = now;
                list.insert(0, n);
                return;
            }
        }
        list.insert(
            0,
            StoredNotification {
                id,
                kind: kind.into(),
                subject,
                count: 1,
                actor,
                created: now,
                read: false,
                text: text.into(),
            },
        );
        list.truncate(limit);
        self.next_notification += 1;
        let n = self.people.iter().find(|p| p.id == to).and_then(|p| {
            p.notifications
                .first()
                .map(|n| (p.clone(), self.notification_view(n)))
        });
        if let Some((p, view)) = n {
            if kind == REMINDER {
                if !p.email.is_empty() && !p.email_off {
                    self.deliveries.push(Delivery {
                        channel: "email",
                        to: p.email.clone(),
                        name: p.display_name.clone(),
                        subject: format!("Reminder: {}", view.text),
                        text: format!("{},\n\n{}", p.display_name, view.text),
                        link: view.link,
                    });
                }
            } else if !p.mastodon.is_empty() && !p.dm_off {
                self.deliveries.push(Delivery {
                    channel: "dm",
                    to: p.mastodon.clone(),
                    name: p.display_name.clone(),
                    subject: String::new(),
                    text: view.text,
                    link: view.link,
                });
            }
        }
    }

    /// One notification as a sentence and a link.
    pub fn notification_view(&self, n: &StoredNotification) -> NotificationView {
        let who = self.name_of(n.actor);
        let event = self.events.iter().find(|e| e.id == n.subject);
        let title = event.map_or_else(|| "an event".to_string(), |e| e.title.clone());
        let (text, link) = match n.kind.as_str() {
            CALL => (
                if n.actor == 0 {
                    "Someone at the board calls everyone to practice".to_string()
                } else if n.text.is_empty() {
                    format!("{who} calls everyone to practice")
                } else {
                    format!("{who}: {}", n.text)
                },
                "/house".to_string(),
            ),
            INVITE => (
                match event {
                    Some(e) => format!("{who} invited you to {title}, {}", when(&self.tz, e.start)),
                    None => format!("{who} invited you to {title}"),
                },
                format!("/events/{}", n.subject),
            ),
            RSVP if n.count > 1 => (
                format!("{} answers to {title}", n.count),
                format!("/events/{}", n.subject),
            ),
            RSVP => (
                format!("{who} answered {title}"),
                format!("/events/{}", n.subject),
            ),
            CHANGED => (
                match event {
                    Some(e) => format!("{title} changed: now {}", when(&self.tz, e.start)),
                    None => format!("{title} changed"),
                },
                format!("/events/{}", n.subject),
            ),
            CANCELLED => (
                format!("{title} is cancelled"),
                format!("/events/{}", n.subject),
            ),
            REMINDER => (
                n.text.clone(),
                if n.subject == 0 {
                    String::new()
                } else {
                    format!("/events/{}", n.subject)
                },
            ),
            OBSERVANCE => (n.text.clone(), "/calendar".into()),
            REJOICE if n.count > 1 => (
                format!("{} people rejoiced in your practice", n.count),
                String::new(),
            ),
            REJOICE => (format!("{who} rejoiced in your practice"), String::new()),
            _ => (n.text.clone(), String::new()),
        };
        NotificationView {
            id: n.id,
            kind: n.kind.clone(),
            text,
            link,
            count: n.count,
            created: n.created,
            read: n.read,
        }
    }

    pub fn notifications_of(&self, person: u32) -> (Vec<NotificationView>, u32) {
        let Some(p) = self.person(person) else {
            return (Vec::new(), 0);
        };
        let views = p
            .notifications
            .iter()
            .map(|n| self.notification_view(n))
            .collect();
        let unread = p.notifications.iter().filter(|n| !n.read).count() as u32;
        (views, unread)
    }

    /// (unread, newest id): what the bell polls.
    pub fn summary(&self, person: u32) -> (u32, u32) {
        self.person(person).map_or((0, 0), |p| {
            (
                p.notifications.iter().filter(|n| !n.read).count() as u32,
                p.notifications.first().map_or(0, |n| n.id),
            )
        })
    }

    /// Marks notifications up to `up_to` (0 = all) read; true if any changed.
    pub fn mark_read(&mut self, person: u32, up_to: u32) -> bool {
        let Ok(p) = self.person_mut(person) else {
            return false;
        };
        let mut changed = false;
        for n in p
            .notifications
            .iter_mut()
            .filter(|n| !n.read && (up_to == 0 || n.id <= up_to))
        {
            n.read = true;
            changed = true;
        }
        changed
    }

    /// Where the household bot DMs `person` ("" = nowhere), and which
    /// channels they've turned off.
    pub fn set_delivery(
        &mut self,
        person: u32,
        mastodon: Option<&str>,
        dm_off: Option<bool>,
        email_off: Option<bool>,
    ) -> Result<(), ApiError> {
        let handle = mastodon.map(normalize_handle).transpose()?;
        let p = self.person_mut(person)?;
        if let Some(h) = handle {
            p.mastodon = h;
        }
        if let Some(off) = dm_off {
            p.dm_off = off;
        }
        if let Some(off) = email_off {
            p.email_off = off;
        }
        Ok(())
    }

    // ---- calls to service ----------------------------------------------------

    /// Calls the household (or `to`) to practice; returns how many were told.
    pub fn call_to_service(
        &mut self,
        actor: Actor,
        input: &CallInput,
        now: u64,
    ) -> Result<usize, ApiError> {
        // The board's touch menu calls as "the board" (household trust).
        let (me, guest) = match actor {
            Actor::Person { id, guest, .. } => (id, guest),
            Actor::Board => (0, false),
        };
        if guest {
            return Err(ApiError::new(
                403,
                "guest",
                "Guests can't call the household",
            ));
        }
        let message = input
            .message
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let when = input.when.trim().to_string();
        if message.chars().count() > 200 || when.chars().count() > 40 {
            return Err(ApiError::bad_request(
                "A call is at most 200 characters, and its time 40",
            ));
        }
        let everyone: Vec<u32> = self
            .people
            .iter()
            .filter(|p| !p.disabled && p.id != me)
            .map(|p| p.id)
            .collect();
        let to: Vec<u32> = if input.to.is_empty() {
            everyone
        } else {
            if let Some(bad) = input.to.iter().find(|id| !everyone.contains(id)) {
                return Err(ApiError::bad_request(format!("No such person: {bad}")));
            }
            input.to.clone()
        };
        let text = match (message.is_empty(), when.is_empty()) {
            (true, true) => String::new(),
            (false, true) => message.clone(),
            (true, false) => format!("practice, {when}"),
            (false, false) => format!("{message} ({when})"),
        };
        let id = self.calls.iter().map(|c| c.id).max().unwrap_or(0) + 1;
        self.calls.insert(
            0,
            StoredCall {
                id,
                by: me,
                message,
                when,
                created: now,
                to: if input.to.is_empty() {
                    Vec::new()
                } else {
                    to.clone()
                },
            },
        );
        self.calls.truncate(CALLS_KEPT);
        for person in &to {
            self.notify(*person, CALL, id, me, &text, now);
        }
        Ok(to.len())
    }

    /// Calls from the last few hours, newest first.
    pub fn recent_calls(&self, now: u64) -> Vec<CallView> {
        self.calls
            .iter()
            .filter(|c| now.saturating_sub(c.created) < CALL_SHOWN)
            .map(|c| CallView {
                id: c.id,
                by: c.by,
                by_name: self.name_of(c.by),
                message: c.message.clone(),
                when: c.when.clone(),
                created: c.created,
            })
            .collect()
    }

    // ---- reminders -------------------------------------------------------------

    fn checked_reminder(input: &ReminderInput) -> Result<StoredReminder, ApiError> {
        let bad = |m: &str| Err(ApiError::bad_request(m.to_string()));
        if parse_hhmm(&input.time).is_none() {
            return bad("Time: HH:MM");
        }
        match input.kind.as_str() {
            "daily" => {
                if input.days > 0x7f {
                    return bad("Days: a weekday mask");
                }
            }
            "observance" => {
                if input.before > 7 {
                    return bad("Up to 7 days before");
                }
                if !matches!(
                    input.observances.as_str(),
                    "any" | "uposatha" | "fasting" | "festival" | "feast"
                ) {
                    return bad("Observances: any, uposatha, fasting, festival or feast");
                }
            }
            _ => return bad("A reminder is daily or for observances"),
        }
        let text = input.text.split_whitespace().collect::<Vec<_>>().join(" ");
        if text.chars().count() > 120 {
            return bad("Reminder text: at most 120 characters");
        }
        Ok(StoredReminder {
            id: 0,
            kind: input.kind.clone(),
            time: input.time.clone(),
            days: if input.kind == "daily" { input.days } else { 0 },
            before: if input.kind == "observance" {
                input.before
            } else {
                0
            },
            observances: if input.kind == "observance" {
                input.observances.clone()
            } else {
                String::new()
            },
            text,
            last_due: 0,
            off: input.off,
        })
    }

    pub fn add_reminder(
        &mut self,
        person: u32,
        input: &ReminderInput,
        now: u64,
    ) -> Result<u32, ApiError> {
        let mut r = Self::checked_reminder(input)?;
        let limit = self.limits.reminders;
        let id = self.next_reminder;
        // Nothing earlier today fires for a reminder made now.
        r.last_due = now;
        r.id = id;
        let p = self.person_mut(person)?;
        if p.reminders.len() >= limit {
            return Err(ApiError::new(
                409,
                "full",
                format!("Up to {limit} reminders each"),
            ));
        }
        p.reminders.push(r);
        self.next_reminder += 1;
        Ok(id)
    }

    pub fn update_reminder(
        &mut self,
        person: u32,
        id: u32,
        input: &ReminderInput,
        now: u64,
    ) -> Result<(), ApiError> {
        let mut r = Self::checked_reminder(input)?;
        let slot = self
            .person_mut(person)?
            .reminders
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| ApiError::not_found("No such reminder"))?;
        r.id = id;
        r.last_due = now;
        *slot = r;
        Ok(())
    }

    pub fn delete_reminder(&mut self, person: u32, id: u32) -> Result<(), ApiError> {
        let p = self.person_mut(person)?;
        let before = p.reminders.len();
        p.reminders.retain(|r| r.id != id);
        if p.reminders.len() == before {
            return Err(ApiError::not_found("No such reminder"));
        }
        Ok(())
    }

    /// The text a reminder says on `date`: its own, or the observance's.
    fn reminder_due(&self, person: u32, r: &StoredReminder, date: Date) -> Option<String> {
        if r.off {
            return None;
        }
        match r.kind.as_str() {
            "daily" => {
                // Bit 0 = Monday; `weekday()` has 0 = Sunday.
                let bit = (date.weekday() + 6) % 7;
                (r.days == 0 || r.days & (1 << bit) != 0).then(|| {
                    if r.text.is_empty() {
                        "Time to practice".to_string()
                    } else {
                        r.text.clone()
                    }
                })
            }
            _ => {
                let day = date.add_days(i64::from(r.before));
                let system = self.calendar_of(person);
                let found = crate::calendar::observances(
                    system,
                    self.calendar_options(),
                    &self.tz,
                    day,
                    day,
                );
                let o = found.iter().find(|o| match r.observances.as_str() {
                    "any" => o.kind != "moon",
                    "feast" => o.feast,
                    k => o.kind == k,
                })?;
                let lead = match r.before {
                    0 => "Today".to_string(),
                    1 => "Tomorrow".to_string(),
                    n => format!("In {n} days"),
                };
                Some(if r.text.is_empty() {
                    format!("{lead}: {}", o.name)
                } else {
                    format!("{lead}: {}. {}", o.name, r.text)
                })
            }
        }
    }

    /// Fires reminders that came due by `now`: today's and yesterday's due
    /// times (so a late tick or a reboot over midnight is covered), each
    /// once. Missed by more than [`CATCH_UP`]: skipped. True if anything
    /// changed.
    pub fn run_reminders(&mut self, now: u64) -> bool {
        let today = self.tz.local_date(now as i64);
        let mut fired = Vec::new();
        let mut changed = false;
        let people: Vec<u32> = self
            .people
            .iter()
            .filter(|p| !p.disabled)
            .map(|p| p.id)
            .collect();
        for person in people {
            let reminders = self
                .person(person)
                .map(|p| p.reminders.clone())
                .unwrap_or_default();
            for r in reminders {
                let Some(minutes) = parse_hhmm(&r.time) else {
                    continue;
                };
                let mut last_due = r.last_due;
                for date in [today.add_days(-1), today] {
                    let due = local_instant(&self.tz, date, minutes);
                    if due <= last_due || due > now {
                        continue;
                    }
                    last_due = due;
                    if now - due < CATCH_UP {
                        if let Some(text) = self.reminder_due(person, &r, date) {
                            fired.push((person, text));
                        }
                    }
                }
                if last_due != r.last_due {
                    changed = true;
                    if let Some(slot) = self
                        .person_mut(person)
                        .ok()
                        .and_then(|p| p.reminders.iter_mut().find(|x| x.id == r.id))
                    {
                        slot.last_due = last_due;
                    }
                }
            }
        }
        for (person, text) in fired {
            self.notify(person, REMINDER, 0, 0, &text, now);
        }
        changed
    }

    /// Once a day after [`OBSERVANCE_AT`]: today's observances in each
    /// person's calendar go in their bell (and DM). True if it ran.
    pub fn run_observance_notices(&mut self, now: u64) -> bool {
        let today = self.tz.local_date(now as i64);
        if self.observance_day == today.iso() || now < local_instant(&self.tz, today, OBSERVANCE_AT)
        {
            return false;
        }
        self.observance_day = today.iso();
        // Missed by hours (the board was off all morning): don't send late.
        if now - local_instant(&self.tz, today, OBSERVANCE_AT) > 4 * 3600 {
            return true;
        }
        let people: Vec<u32> = self
            .people
            .iter()
            .filter(|p| !p.disabled && p.role != GUEST)
            .map(|p| p.id)
            .collect();
        for person in people {
            let system = self.calendar_of(person);
            let found = crate::calendar::observances(
                system,
                self.calendar_options(),
                &self.tz,
                today,
                today,
            );
            let names: Vec<String> = found
                .iter()
                .filter(|o| o.kind != "moon")
                .map(|o| {
                    if o.feast {
                        format!("{} (a feast day)", o.name)
                    } else {
                        o.name.clone()
                    }
                })
                .collect();
            if !names.is_empty() {
                let text = format!("Today: {}", names.join(", "));
                self.notify(person, OBSERVANCE, 0, 0, &text, now);
            }
        }
        true
    }

    /// Whether `viewer` may see `person`'s reminders (themselves, or an admin).
    pub fn may_manage(viewer: Viewer, person: u32) -> bool {
        viewer.admin || (viewer.id != 0 && viewer.id == person)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Limits, MEMBER};

    fn state() -> (State, u32, u32) {
        let mut s = State::new(Limits {
            pin_rounds: 2,
            ..Limits::board()
        });
        let dad = s.setup("Dad", "1234", 0).unwrap();
        let kid = s
            .add_person("Kid", MEMBER, "1111", "kid@home.example", 0)
            .unwrap();
        s.person_mut(kid).unwrap().mastodon = "@kid@mastomini.local".into();
        (s, dad, kid)
    }

    fn me(id: u32) -> Actor {
        Actor::Person {
            id,
            admin: false,
            guest: false,
        }
    }

    /// 2026-10-14 (a Wednesday) at `hh:mm` in New York.
    fn at(hh: u32, mm: u32) -> u64 {
        let tz = Tz::parse(crate::state::DEFAULT_TZ).unwrap();
        local_instant(&tz, Date::parse("2026-10-14").unwrap(), hh * 60 + mm)
    }

    #[test]
    fn local_times_round_trip() {
        let t = at(7, 0);
        let tz = Tz::parse(crate::state::DEFAULT_TZ).unwrap();
        assert_eq!(when(&tz, t), "Wednesday 14 October, 07:00");
        assert_eq!(t % 86_400, 11 * 3600, "07:00 EDT is 11:00 UTC");
        assert_eq!(parse_hhmm("07:30"), Some(450));
        assert_eq!(parse_hhmm("24:00"), None);
        assert_eq!(parse_hhmm("7:30"), None);
    }

    #[test]
    fn unread_news_coalesces_and_sends_once() {
        let (mut s, dad, kid) = state();
        for i in 0..10 {
            s.notify(kid, RSVP, 7, dad, "", 100 + i);
        }
        let p = s.person(kid).unwrap();
        assert_eq!(p.notifications.len(), 1);
        assert_eq!(p.notifications[0].count, 10);
        assert_eq!(s.deliveries.len(), 1, "ten quick changes, one DM");
        assert_eq!(s.deliveries[0].channel, "dm");
        assert_eq!(s.deliveries[0].to, "@kid@mastomini.local");
        // Once read, the next one is news again.
        assert!(s.mark_read(kid, 0));
        s.notify(kid, RSVP, 7, dad, "", 200);
        assert_eq!(s.person(kid).unwrap().notifications.len(), 2);
        assert_eq!(
            s.summary(kid),
            (1, s.person(kid).unwrap().notifications[0].id)
        );
        // Nobody hears about their own doing; the ring is bounded.
        s.notify(kid, CALL, 1, kid, "", 300);
        assert_eq!(s.person(kid).unwrap().notifications.len(), 2);
        for i in 0..100 {
            s.notify(kid, CALL, i, dad, "", 400);
        }
        assert_eq!(s.person(kid).unwrap().notifications.len(), 30);
    }

    #[test]
    fn dms_and_emails_follow_preferences() {
        let (mut s, dad, kid) = state();
        s.person_mut(kid).unwrap().dm_off = true;
        s.notify(kid, CALL, 1, dad, "Come and chant", 1);
        assert!(s.deliveries.is_empty());
        s.notify(kid, REMINDER, 0, 0, "Sit", 2);
        assert_eq!(s.deliveries.len(), 1);
        assert_eq!(s.deliveries[0].channel, "email");
        assert_eq!(s.deliveries[0].to, "kid@home.example");
        s.person_mut(kid).unwrap().email_off = true;
        s.notify(kid, REMINDER, 0, 0, "Sit", 3);
        assert_eq!(s.deliveries.len(), 1);
    }

    #[test]
    fn calls_reach_the_household() {
        let (mut s, dad, kid) = state();
        let input = CallInput {
            message: "Dinner, then chanting".into(),
            when: "at 7".into(),
            to: vec![],
        };
        assert_eq!(s.call_to_service(me(dad), &input, 1_000).unwrap(), 1);
        let (list, unread) = s.notifications_of(kid);
        assert_eq!(unread, 1);
        assert_eq!(list[0].text, "Dad: Dinner, then chanting (at 7)");
        assert_eq!(s.recent_calls(1_000 + 60).len(), 1);
        assert!(s.recent_calls(1_000 + CALL_SHOWN).is_empty());
        let guest = Actor::Person {
            id: kid,
            admin: false,
            guest: true,
        };
        assert_eq!(s.call_to_service(guest, &input, 1).unwrap_err().status, 403);
        let nobody = CallInput {
            to: vec![99],
            ..input
        };
        assert_eq!(
            s.call_to_service(me(dad), &nobody, 1).unwrap_err().status,
            400
        );
    }

    fn daily(time: &str) -> ReminderInput {
        ReminderInput {
            kind: "daily".into(),
            time: time.into(),
            text: "Sit".into(),
            ..Default::default()
        }
    }

    #[test]
    fn a_reminder_fires_once_at_its_time() {
        let (mut s, _, kid) = state();
        s.add_reminder(kid, &daily("07:00"), at(6, 0)).unwrap();
        assert!(!s.run_reminders(at(6, 59)));
        assert!(s.run_reminders(at(7, 0)));
        assert_eq!(s.deliveries.len(), 1);
        assert!(!s.run_reminders(at(7, 1)), "once");
        assert_eq!(s.deliveries.len(), 1);
        assert_eq!(s.deliveries[0].subject, "Reminder: Sit");
    }

    #[test]
    fn after_a_reboot_recent_reminders_catch_up_and_old_ones_skip() {
        let (mut s, _, kid) = state();
        s.add_reminder(kid, &daily("07:00"), at(6, 0)).unwrap();
        s.add_reminder(kid, &daily("09:00"), at(6, 0)).unwrap();
        // The board was off from 6:30 to 10:30; its snapshot comes back.
        let rebooted = State::from_stored(s.to_stored(), s.limits.clone());
        let mut s = rebooted;
        s.run_reminders(at(10, 30));
        // 07:00 is 3.5 h late: skipped. 09:00 is 1.5 h late: skipped too.
        assert!(s.deliveries.is_empty());
        // Off from 8:30 to 9:40: the 09:00 reminder is 40 minutes late, so it fires.
        let (mut s, _, kid) = state();
        s.add_reminder(kid, &daily("09:00"), at(6, 0)).unwrap();
        let mut s = State::from_stored(s.to_stored(), s.limits.clone());
        s.run_reminders(at(9, 40));
        assert_eq!(s.deliveries.len(), 1);
        s.run_reminders(at(9, 41));
        assert_eq!(s.deliveries.len(), 1);
    }

    #[test]
    fn weekday_masks() {
        let (mut s, _, kid) = state();
        // Wednesday is bit 2.
        let mut r = daily("07:00");
        r.days = 0b0000_0100;
        s.add_reminder(kid, &r, at(6, 0)).unwrap();
        r.days = 0b0000_0001;
        s.add_reminder(kid, &r, at(6, 0)).unwrap();
        s.run_reminders(at(7, 5));
        assert_eq!(s.deliveries.len(), 1);
        assert!(s.add_reminder(kid, &daily("7am"), 0).is_err());
    }
}
