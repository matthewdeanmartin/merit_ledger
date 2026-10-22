//! Events (`spec/domain.md` § Calendar, Event): a feast, a group sit, a
//! service. People in the house are invited by name (their bell and a DM);
//! anyone else gets a link with a token, shared from the host's own phone
//! (the "intents" in the UI: email, text, WhatsApp, Mastodon...), and
//! answers on the board without an account. The board sends no invitation
//! email itself.
//!
//! Members see every event (it's one household); guest accounts see the
//! ones they're invited to; a token sees its own event.
use crate::ledger::{Actor, Viewer};
use crate::messages::{
    EventInput, EventPerson, EventView, GuestView, RsvpInput, StoredEvent, StoredGuest, StoredRsvp,
};
use crate::notify::{self, CANCELLED, CHANGED, INVITE, REMINDER, RSVP};
use crate::state::{State, GUEST};
use miniframework::ApiError;

pub const GUESTS_MAX: usize = 24;
/// "Starting soon" goes out this long before an event.
pub const REMIND_BEFORE: u64 = 3600;

fn clean(raw: &str, max: usize, what: &str) -> Result<String, ApiError> {
    let s = raw.trim().to_string();
    if s.chars().count() > max {
        return Err(ApiError::bad_request(format!(
            "{what}: at most {max} characters"
        )));
    }
    Ok(s)
}

fn check_answer(a: &str) -> Result<String, ApiError> {
    match a {
        "yes" | "maybe" | "no" => Ok(a.into()),
        _ => Err(ApiError::bad_request("Answer yes, maybe or no")),
    }
}

impl State {
    fn can_see_event(&self, viewer: Viewer, e: &StoredEvent) -> bool {
        if viewer.id == 0 {
            return false;
        }
        if viewer.admin || e.host == viewer.id || e.invitees.contains(&viewer.id) {
            return true;
        }
        self.person(viewer.id).is_some_and(|p| p.role != GUEST)
    }

    /// The event as `viewer` sees it, or as guest `guest` (by token).
    pub fn event_view(
        &self,
        viewer: Viewer,
        e: &StoredEvent,
        guest: Option<&StoredGuest>,
    ) -> EventView {
        let host = viewer.admin || viewer.id == e.host;
        let answer = |person: u32| e.rsvps.iter().find(|r| r.person == person);
        let mut people: Vec<u32> = vec![e.host];
        people.extend(e.invitees.iter().filter(|p| **p != e.host));
        people.extend(
            e.rsvps
                .iter()
                .map(|r| r.person)
                .filter(|p| *p != e.host && !e.invitees.contains(p)),
        );
        let mine = answer(viewer.id);
        EventView {
            id: e.id,
            host: e.host,
            host_name: self.name_of(e.host),
            title: e.title.clone(),
            start: e.start,
            end: e.end,
            place: e.place.clone(),
            notes: e.notes.clone(),
            people: people
                .into_iter()
                .map(|p| EventPerson {
                    person: p,
                    name: self.name_of(p),
                    answer: answer(p).map_or_else(
                        || {
                            if p == e.host {
                                "yes".into()
                            } else {
                                String::new()
                            }
                        },
                        |r| r.answer.clone(),
                    ),
                    bringing: answer(p).map(|r| r.bringing.clone()).unwrap_or_default(),
                })
                .collect(),
            guests: e
                .guests
                .iter()
                .map(|g| GuestView {
                    id: g.id,
                    name: g.name.clone(),
                    answer: g.answer.clone(),
                    bringing: g.bringing.clone(),
                    link: if host {
                        format!("/e/{}?t={}", e.id, g.token)
                    } else {
                        String::new()
                    },
                })
                .collect(),
            my_answer: match guest {
                Some(g) => g.answer.clone(),
                None => mine.map(|r| r.answer.clone()).unwrap_or_default(),
            },
            my_bringing: match guest {
                Some(g) => g.bringing.clone(),
                None => mine.map(|r| r.bringing.clone()).unwrap_or_default(),
            },
            can_edit: host,
            cancelled: e.cancelled,
            rev: e.rev,
            ics: format!("/cal/event/{}.ics", e.id),
            guest: guest.map_or(0, |g| g.id),
        }
    }

    /// Events `viewer` may see that end after `since`, soonest first.
    pub fn events_for(&self, viewer: Viewer, since: u64) -> Vec<EventView> {
        let mut list: Vec<&StoredEvent> = self
            .events
            .iter()
            .filter(|e| e.end.max(e.start).saturating_add(3600) >= since)
            .filter(|e| self.can_see_event(viewer, e))
            .collect();
        list.sort_by_key(|e| (e.start, e.id));
        list.into_iter()
            .map(|e| self.event_view(viewer, e, None))
            .collect()
    }

    pub fn event(&self, viewer: Viewer, id: u32) -> Result<EventView, ApiError> {
        self.events
            .iter()
            .find(|e| e.id == id && self.can_see_event(viewer, e))
            .map(|e| self.event_view(viewer, e, None))
            .ok_or_else(|| ApiError::not_found("No such event"))
    }

    /// The event a guest's token opens.
    pub fn event_for_guest(&self, id: u32, token: &str) -> Result<EventView, ApiError> {
        let not_found = || ApiError::not_found("This invitation link isn't valid any more");
        let e = self
            .events
            .iter()
            .find(|e| e.id == id)
            .ok_or_else(not_found)?;
        let g = e
            .guests
            .iter()
            .find(|g| !token.is_empty() && g.token == token)
            .ok_or_else(not_found)?;
        Ok(self.event_view(
            Viewer {
                id: 0,
                admin: false,
                outsider: false,
            },
            e,
            Some(g),
        ))
    }

    fn checked_event(&self, input: &EventInput, host: u32) -> Result<StoredEvent, ApiError> {
        let title = clean(&input.title, 100, "Title")?;
        if title.is_empty() {
            return Err(ApiError::bad_request("Give the event a title"));
        }
        if input.start == 0 {
            return Err(ApiError::bad_request("When does it start?"));
        }
        // Year 9999 at the latest: past that the arithmetic and the
        // calendar feeds stop making sense (found by tests/props.rs).
        const LAST: u64 = 253_402_300_799;
        if input.start > LAST || input.end > LAST {
            return Err(ApiError::bad_request("That date is too far away"));
        }
        if input.end != 0 && input.end < input.start {
            return Err(ApiError::bad_request("It can't end before it starts"));
        }
        let mut invitees = input.invitees.clone();
        invitees.sort_unstable();
        invitees.dedup();
        invitees.retain(|p| *p != host);
        if let Some(bad) = invitees
            .iter()
            .find(|p| !self.person(**p).is_some_and(|q| !q.disabled))
        {
            return Err(ApiError::bad_request(format!("No such person: {bad}")));
        }
        Ok(StoredEvent {
            host,
            title,
            start: input.start,
            end: input.end,
            place: clean(&input.place, 100, "Place")?,
            notes: clean(&input.notes, 1_000, "Notes")?,
            invitees,
            ..Default::default()
        })
    }

    pub fn add_event(
        &mut self,
        actor: Actor,
        input: &EventInput,
        now: u64,
    ) -> Result<u32, ApiError> {
        let Actor::Person {
            id: host, guest, ..
        } = actor
        else {
            return Err(ApiError::new(
                403,
                "forbidden",
                "The board can't plan events",
            ));
        };
        if guest {
            return Err(ApiError::new(403, "guest", "Guests can't plan events"));
        }
        let mut e = self.checked_event(input, host)?;
        if self.events.len() >= self.limits.events {
            // Long-past events make room; upcoming ones never do.
            let old = self
                .events
                .iter()
                .position(|e| e.end.max(e.start) + 30 * 86_400 < now);
            match old {
                Some(i) => {
                    self.events.remove(i);
                }
                None => {
                    return Err(ApiError::new(
                        409,
                        "full",
                        format!("The board holds {} events", self.limits.events),
                    ))
                }
            }
        }
        e.id = self.next_event;
        e.rev = 1;
        e.created = now;
        e.next_guest = 1;
        self.next_event += 1;
        let (id, invitees) = (e.id, e.invitees.clone());
        self.events.push(e);
        for p in invitees {
            self.notify(p, INVITE, id, host, "", now);
        }
        Ok(id)
    }

    fn hosted(&self, actor: Actor, id: u32) -> Result<(usize, u32), ApiError> {
        let Actor::Person { id: me, admin, .. } = actor else {
            return Err(ApiError::new(
                403,
                "forbidden",
                "The board can't change events",
            ));
        };
        let at = self
            .events
            .iter()
            .position(|e| e.id == id)
            .ok_or_else(|| ApiError::not_found("No such event"))?;
        if self.events[at].host != me && !admin {
            return Err(ApiError::new(
                403,
                "forbidden",
                "Only its host (or an admin) can change an event",
            ));
        }
        Ok((at, me))
    }

    /// Changes an event; newly invited people get an invitation, and
    /// everyone else involved hears that it changed (when its time did).
    pub fn update_event(
        &mut self,
        actor: Actor,
        id: u32,
        input: &EventInput,
        now: u64,
    ) -> Result<(), ApiError> {
        let (at, me) = self.hosted(actor, id)?;
        if self.events[at].rev != input.rev {
            return Err(ApiError::new(
                409,
                "stale",
                "This event changed; reload and try again",
            ));
        }
        let host = self.events[at].host;
        let new = self.checked_event(input, host)?;
        let old = self.events[at].clone();
        let moved = old.start != new.start || old.end != new.end || old.place != new.place;
        let e = &mut self.events[at];
        e.title = new.title;
        e.start = new.start;
        e.end = new.end;
        e.place = new.place;
        e.notes = new.notes;
        e.invitees = new.invitees.clone();
        e.rev += 1;
        if moved {
            e.reminded = false;
        }
        for p in &new.invitees {
            if !old.invitees.contains(p) {
                self.notify(*p, INVITE, id, me, "", now);
            } else if moved {
                self.notify(*p, CHANGED, id, me, "", now);
            }
        }
        if moved && host != me {
            self.notify(host, CHANGED, id, me, "", now);
        }
        Ok(())
    }

    pub fn cancel_event(&mut self, actor: Actor, id: u32, now: u64) -> Result<(), ApiError> {
        let (at, me) = self.hosted(actor, id)?;
        let e = &mut self.events[at];
        if e.cancelled {
            return Ok(());
        }
        e.cancelled = true;
        e.rev += 1;
        let mut told = e.invitees.clone();
        told.push(e.host);
        for p in told {
            self.notify(p, CANCELLED, id, me, "", now);
        }
        Ok(())
    }

    pub fn delete_event(&mut self, actor: Actor, id: u32) -> Result<(), ApiError> {
        let (at, _) = self.hosted(actor, id)?;
        self.events.remove(at);
        Ok(())
    }

    /// A member's answer; the host hears about it (coalesced).
    pub fn rsvp(
        &mut self,
        actor: Actor,
        id: u32,
        input: &RsvpInput,
        now: u64,
    ) -> Result<(), ApiError> {
        let Actor::Person { id: me, admin, .. } = actor else {
            return Err(ApiError::new(403, "forbidden", "Answer as a person"));
        };
        let answer = check_answer(&input.answer)?;
        let bringing = clean(&input.bringing, 100, "Bringing")?;
        let visible = self
            .event(
                Viewer {
                    id: me,
                    admin,
                    outsider: false,
                },
                id,
            )
            .is_ok();
        let e = self
            .events
            .iter_mut()
            .find(|e| e.id == id && visible)
            .ok_or_else(|| ApiError::not_found("No such event"))?;
        if e.cancelled {
            return Err(ApiError::new(409, "cancelled", "This event is cancelled"));
        }
        e.rsvps.retain(|r| r.person != me);
        e.rsvps.push(StoredRsvp {
            person: me,
            answer,
            bringing,
            at: now,
        });
        let host = e.host;
        self.notify(host, RSVP, id, me, "", now);
        Ok(())
    }

    /// A guest's answer, by the token in their link.
    pub fn guest_rsvp(&mut self, id: u32, input: &RsvpInput, now: u64) -> Result<(), ApiError> {
        let answer = check_answer(&input.answer)?;
        let bringing = clean(&input.bringing, 100, "Bringing")?;
        let not_found = || ApiError::not_found("This invitation link isn't valid any more");
        let e = self
            .events
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or_else(not_found)?;
        if e.cancelled {
            return Err(ApiError::new(409, "cancelled", "This event is cancelled"));
        }
        let g = e
            .guests
            .iter_mut()
            .find(|g| !input.token.is_empty() && g.token == input.token)
            .ok_or_else(not_found)?;
        g.answer = answer;
        g.bringing = bringing;
        g.at = now;
        let host = e.host;
        self.notify(host, RSVP, id, 0, "", now);
        Ok(())
    }

    /// Adds a guest; returns their token (the link the host shares).
    pub fn add_guest(
        &mut self,
        actor: Actor,
        id: u32,
        name: &str,
        token: String,
    ) -> Result<String, ApiError> {
        let (at, _) = self.hosted(actor, id)?;
        let name = clean(name, 60, "Name")?;
        if name.is_empty() {
            return Err(ApiError::bad_request("Who are you inviting?"));
        }
        let e = &mut self.events[at];
        if e.guests.len() >= GUESTS_MAX {
            return Err(ApiError::new(
                409,
                "full",
                format!("Up to {GUESTS_MAX} guests per event"),
            ));
        }
        let gid = e.next_guest.max(1);
        e.next_guest = gid + 1;
        e.guests.push(StoredGuest {
            id: gid,
            name,
            token: token.clone(),
            ..Default::default()
        });
        e.rev += 1;
        Ok(token)
    }

    pub fn remove_guest(&mut self, actor: Actor, id: u32, guest: u32) -> Result<(), ApiError> {
        let (at, _) = self.hosted(actor, id)?;
        let e = &mut self.events[at];
        let before = e.guests.len();
        e.guests.retain(|g| g.id != guest);
        if e.guests.len() == before {
            return Err(ApiError::not_found("No such guest"));
        }
        e.rev += 1;
        Ok(())
    }

    /// "Starting soon" for the host and everyone who said yes or maybe, once
    /// per event, from [`REMIND_BEFORE`] until it starts. True if any went out.
    pub fn run_event_reminders(&mut self, now: u64) -> bool {
        let due: Vec<(u32, Vec<u32>, String)> = self
            .events
            .iter()
            .filter(|e| {
                !e.cancelled && !e.reminded && e.start > now && e.start - now <= REMIND_BEFORE
            })
            .map(|e| {
                let mut to = vec![e.host];
                to.extend(
                    e.rsvps
                        .iter()
                        .filter(|r| r.answer != "no" && r.person != e.host)
                        .map(|r| r.person),
                );
                let text = format!("{} starts {}", e.title, notify::when(&self.tz, e.start));
                (e.id, to, text)
            })
            .collect();
        for (id, to, text) in &due {
            if let Some(e) = self.events.iter_mut().find(|e| e.id == *id) {
                e.reminded = true;
            }
            for p in to {
                self.notify(*p, REMINDER, *id, 0, text, now);
            }
        }
        !due.is_empty()
    }
}

/// One event as an iCalendar file.
pub fn ics(e: &StoredEvent, origin: &str) -> String {
    use crate::ics::{line, raw_line, utc_stamp};
    let mut out = String::new();
    for l in [
        "BEGIN:VCALENDAR",
        "VERSION:2.0",
        "PRODID:-//Merit Ledger//EN",
        "CALSCALE:GREGORIAN",
        "BEGIN:VEVENT",
    ] {
        raw_line(l, &mut out);
    }
    raw_line(&format!("UID:event-{}@meritledger", e.id), &mut out);
    raw_line(
        &format!("DTSTAMP:{}", utc_stamp(e.created.max(1))),
        &mut out,
    );
    raw_line(&format!("DTSTART:{}", utc_stamp(e.start)), &mut out);
    let end = if e.end > e.start {
        e.end
    } else {
        e.start.saturating_add(3600)
    };
    raw_line(&format!("DTEND:{}", utc_stamp(end)), &mut out);
    line("SUMMARY", &e.title, &mut out);
    if !e.place.is_empty() {
        line("LOCATION", &e.place, &mut out);
    }
    let mut description = e.notes.clone();
    if !description.is_empty() {
        description.push_str("\n\n");
    }
    description.push_str(&format!("{origin}/events/{}", e.id));
    line("DESCRIPTION", &description, &mut out);
    if e.cancelled {
        raw_line("STATUS:CANCELLED", &mut out);
    }
    raw_line("END:VEVENT", &mut out);
    raw_line("END:VCALENDAR", &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Limits, MEMBER};

    fn state() -> (State, u32, u32, u32) {
        let mut s = State::new(Limits {
            pin_rounds: 2,
            ..Limits::board()
        });
        let dad = s.setup("Dad", "1234", 0).unwrap();
        let mum = s.add_person("Mum", MEMBER, "1111", "", 0).unwrap();
        let visitor = s.add_person("Visitor", GUEST, "2222", "", 0).unwrap();
        (s, dad, mum, visitor)
    }

    fn me(id: u32) -> Actor {
        Actor::Person {
            id,
            admin: false,
            guest: false,
        }
    }

    fn sees(id: u32) -> Viewer {
        Viewer {
            id,
            admin: false,
            outsider: false,
        }
    }

    fn feast(invitees: Vec<u32>) -> EventInput {
        EventInput {
            title: "Uposatha dinner".into(),
            start: 10_000,
            place: "Home".into(),
            invitees,
            ..Default::default()
        }
    }

    #[test]
    fn invitations_answers_and_changes() {
        let (mut s, dad, mum, visitor) = state();
        let id = s.add_event(me(dad), &feast(vec![mum]), 1).unwrap();
        assert_eq!(s.notifications_of(mum).0[0].kind, INVITE);
        // The visitor (a guest account) isn't invited, so doesn't see it.
        assert!(s.events_for(sees(visitor), 0).is_empty());
        assert_eq!(s.events_for(sees(mum), 0).len(), 1);
        let yes = RsvpInput {
            answer: "yes".into(),
            bringing: "Dumplings".into(),
            ..Default::default()
        };
        s.rsvp(me(mum), id, &yes, 2).unwrap();
        let v = s.event(sees(dad), id).unwrap();
        let mum_row = v.people.iter().find(|p| p.person == mum).unwrap();
        assert_eq!(
            (mum_row.answer.as_str(), mum_row.bringing.as_str()),
            ("yes", "Dumplings")
        );
        assert_eq!(s.notifications_of(dad).0[0].kind, RSVP);
        // Moving it tells Mum.
        let moved = EventInput {
            start: 20_000,
            rev: 1,
            ..feast(vec![mum])
        };
        assert_eq!(
            s.update_event(me(mum), id, &moved, 3).unwrap_err().status,
            403
        );
        s.update_event(me(dad), id, &moved, 3).unwrap();
        assert_eq!(s.notifications_of(mum).0[0].kind, CHANGED);
        s.cancel_event(me(dad), id, 4).unwrap();
        assert_eq!(s.notifications_of(mum).0[0].kind, CANCELLED);
        assert_eq!(s.rsvp(me(mum), id, &yes, 5).unwrap_err().code, "cancelled");
    }

    #[test]
    fn guests_answer_by_link() {
        let (mut s, dad, _, _) = state();
        let id = s.add_event(me(dad), &feast(vec![]), 1).unwrap();
        let token = s
            .add_guest(me(dad), id, "Auntie Lin", "tok123".into())
            .unwrap();
        let v = s.event(sees(dad), id).unwrap();
        assert_eq!(v.guests[0].link, format!("/e/{id}?t=tok123"));
        assert_eq!(s.event_for_guest(id, "nope").unwrap_err().status, 404);
        assert_eq!(s.event_for_guest(id, "").unwrap_err().status, 404);
        let theirs = s.event_for_guest(id, &token).unwrap();
        assert!(
            theirs.guests.iter().all(|g| g.link.is_empty()),
            "guests don't see links"
        );
        let answer = RsvpInput {
            answer: "maybe".into(),
            bringing: String::new(),
            token: token.clone(),
        };
        s.guest_rsvp(id, &answer, 2).unwrap();
        assert_eq!(s.event_for_guest(id, &token).unwrap().my_answer, "maybe");
        assert_eq!(
            s.notifications_of(dad).0[0].text,
            "the board answered Uposatha dinner"
        );
    }

    #[test]
    fn starting_soon_once() {
        let (mut s, dad, mum, _) = state();
        let id = s.add_event(me(dad), &feast(vec![mum]), 1).unwrap();
        let yes = RsvpInput {
            answer: "yes".into(),
            ..Default::default()
        };
        s.rsvp(me(mum), id, &yes, 2).unwrap();
        assert!(!s.run_event_reminders(10_000 - REMIND_BEFORE - 1));
        assert!(s.run_event_reminders(10_000 - 600));
        assert!(!s.run_event_reminders(10_000 - 300));
        assert_eq!(s.notifications_of(mum).0[0].kind, REMINDER);
        assert!(s.notifications_of(dad).0[0]
            .text
            .starts_with("Uposatha dinner starts"));
    }

    #[test]
    fn an_ics_file() {
        let (mut s, dad, _, _) = state();
        let id = s.add_event(me(dad), &feast(vec![]), 1).unwrap();
        let file = ics(
            s.events.iter().find(|e| e.id == id).unwrap(),
            "https://meritledger.local",
        );
        assert!(file.contains("SUMMARY:Uposatha dinner\r\n"));
        assert!(file.contains("DTSTART:19700101T024640Z\r\n"));
        assert!(file.contains("meritledger.local/events/1"));
    }
}
