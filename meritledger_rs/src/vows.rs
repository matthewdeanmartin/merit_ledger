//! Vows and the vow blog (`sprints/sprint09.md`, `spec/domain.md` § Vows).
//!
//! The lifecycle is v1's state machine (`pygameui/.../vow_state.py`), with
//! v1's `repair_in_progress` called `breached`:
//!
//! - A breach never fails a vow: it opens repair. `repair` records a
//!   repentance entry and takes the vow up again.
//! - Others see a breach only if its person chose to show it (it defaults
//!   to `generic`); they see the repair as "Renewed this vow".
//! - Post bodies live on the SD card (`ml/posts/<id>.md`); without a card
//!   they sit in the snapshot under a small quota. `api.rs` does the file
//!   I/O; these functions say what to write.
use crate::ledger::{check_note, check_visibility, clock_not_set, Actor, Viewer};
use crate::markdown;
use crate::messages::{
    EntryInput, PostInput, PostView, StoredPost, StoredVow, StoredVowEvent, VowActionInput,
    VowDetail, VowEventView, VowInput, VowUpdate, VowView, VowsPage,
};
use crate::state::State;
use crate::tz::Date;
use miniframework::ApiError;

pub const TEXT_MAX: usize = 280;
pub const TITLE_MAX: usize = 100;
/// A post body on the card, and in the snapshot without one.
pub const BODY_MAX: usize = 8_000;
pub const SNAPSHOT_BODY_MAX: usize = 2_000;
/// Timeline events per vow.
pub const EVENTS_MAX: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Pause,
    Resume,
    Breach,
    Repair,
    Complete,
    Retire,
}

pub const ACTIONS: [Action; 6] = [
    Action::Pause,
    Action::Resume,
    Action::Breach,
    Action::Repair,
    Action::Complete,
    Action::Retire,
];

impl Action {
    pub fn parse(s: &str) -> Option<Action> {
        ACTIONS.into_iter().find(|a| a.name() == s)
    }

    pub fn name(self) -> &'static str {
        match self {
            Action::Pause => "pause",
            Action::Resume => "resume",
            Action::Breach => "breach",
            Action::Repair => "repair",
            Action::Complete => "complete",
            Action::Retire => "retire",
        }
    }

    /// The states it's legal from, and the state it leads to.
    fn rule(self) -> (&'static [&'static str], &'static str) {
        match self {
            Action::Pause => (&["active", "breached"], "paused"),
            Action::Resume => (&["paused"], "active"),
            Action::Breach => (&["active", "paused", "breached"], "breached"),
            Action::Repair => (&["breached"], "active"),
            Action::Complete => (&["active", "breached"], "completed"),
            Action::Retire => (&["active", "paused", "breached", "completed"], "retired"),
        }
    }
}

pub fn can(action: Action, state: &str) -> bool {
    action.rule().0.contains(&state)
}

/// The state after `action`, or 409 when it isn't allowed from `state`.
pub fn transition(state: &str, action: Action) -> Result<&'static str, ApiError> {
    let (from, to) = action.rule();
    if from.contains(&state) {
        Ok(to)
    } else {
        Err(ApiError::new(
            409,
            "illegal_transition",
            format!("You can't {} a vow that is {state}", action.name()),
        ))
    }
}

fn label(action: &str, own: bool) -> &'static str {
    match action {
        "took" => "Took this vow",
        "pause" => "Paused",
        "resume" => "Took it up again",
        "breach" if own => "Broke this vow",
        "breach" => "Broke this vow, and is repairing it",
        "repair" => "Renewed this vow",
        "complete" => "Completed",
        "retire" => "Set this vow down",
        _ => "",
    }
}

fn check_text(text: &str) -> Result<String, ApiError> {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let len = text.chars().count();
    if (1..=TEXT_MAX).contains(&len) {
        Ok(text)
    } else {
        Err(ApiError::bad_request(format!(
            "A vow is 1 to {TEXT_MAX} characters"
        )))
    }
}

fn parse_date(s: &str) -> Result<Date, ApiError> {
    Date::parse(s).ok_or_else(|| ApiError::bad_request("Dates look like 2026-10-18"))
}

fn forbidden(message: &str) -> ApiError {
    ApiError::new(403, "forbidden", message.to_string())
}

/// Who may change `person`'s vows: themselves, or an admin.
fn person_of(actor: Actor) -> Result<(u32, bool), ApiError> {
    match actor {
        Actor::Person { id, admin, .. } => Ok((id, admin)),
        Actor::Board => Err(forbidden("The board can't change vows")),
    }
}

/// What `api.rs` should do with a post body after a change.
#[derive(Debug, PartialEq, Eq)]
pub enum BodyWrite {
    /// Nothing: it's in the snapshot.
    None,
    /// Write it to the card for post `id`.
    Card { id: u32, body: String },
}

impl State {
    fn today_or_503(&self, now: Option<u64>) -> Result<Date, ApiError> {
        self.today(now).ok_or_else(clock_not_set)
    }

    fn vow_index(&self, actor: Actor, id: u32) -> Result<usize, ApiError> {
        let (me, admin) = person_of(actor)?;
        let at = self
            .vows
            .iter()
            .position(|v| v.id == id)
            .ok_or_else(|| ApiError::not_found("No such vow"))?;
        if self.vows[at].person != me && !admin {
            return Err(forbidden("Only its person can change a vow"));
        }
        Ok(at)
    }

    /// Takes a vow; returns its id.
    pub fn add_vow(
        &mut self,
        actor: Actor,
        input: &VowInput,
        now: Option<u64>,
    ) -> Result<u32, ApiError> {
        let (person, _) = person_of(actor)?;
        let today = self.today_or_503(now)?;
        let mut text = input.text.clone();
        if !input.precept_set.is_empty() {
            let item =
                crate::precepts::item(&input.precept_set, &input.precept_kind, input.precept_n)
                    .ok_or_else(|| ApiError::bad_request("No such precept"))?;
            if !self.has_taken(
                person,
                &input.precept_set,
                &input.precept_kind,
                input.precept_n,
            ) {
                return Err(ApiError::new(
                    409,
                    "not_taken",
                    "Record that you've taken this precept first",
                ));
            }
            if self.vows.iter().any(|v| {
                v.person == person
                    && v.precept_set == input.precept_set
                    && v.precept_kind == input.precept_kind
                    && v.precept_n == input.precept_n
                    && v.state != "retired"
                    && v.state != "completed"
            }) {
                return Err(ApiError::new(
                    409,
                    "already",
                    "That precept is already one of your vows",
                ));
            }
            if text.trim().is_empty() {
                text = item.text.clone();
            }
        }
        let text = check_text(&text)?;
        let started = if input.started.is_empty() {
            today
        } else {
            parse_date(&input.started)?
        };
        if started > today {
            return Err(ApiError::bad_request("That date hasn't happened yet"));
        }
        let ends = self.check_ends(&input.ends, started)?;
        let visibility = if input.visibility.is_empty() {
            "public".to_string()
        } else {
            check_visibility(&input.visibility)?
        };
        if self.vows.len() >= self.limits.vows {
            return Err(ApiError::new(
                409,
                "full",
                format!("The board holds at most {} vows", self.limits.vows),
            ));
        }
        let id = self.next_vow;
        self.next_vow += 1;
        self.vows.push(StoredVow {
            id,
            person,
            text,
            state: "active".into(),
            started: started.iso(),
            ends,
            visibility: visibility.clone(),
            created: now.unwrap_or(0),
            rev: 1,
            precept_set: if input.precept_set.is_empty() {
                String::new()
            } else {
                input.precept_set.clone()
            },
            precept_kind: if input.precept_set.is_empty() {
                String::new()
            } else {
                input.precept_kind.clone()
            },
            precept_n: if input.precept_set.is_empty() {
                0
            } else {
                input.precept_n
            },
            events: vec![StoredVowEvent {
                action: "took".into(),
                date: started.iso(),
                at: now.unwrap_or(0),
                note: String::new(),
                visibility,
                entry: 0,
            }],
        });
        Ok(id)
    }

    fn check_ends(&self, ends: &str, started: Date) -> Result<String, ApiError> {
        if ends.is_empty() {
            return Ok(String::new());
        }
        let ends = parse_date(ends)?;
        if ends < started {
            return Err(ApiError::bad_request("A vow can't end before it starts"));
        }
        Ok(ends.iso())
    }

    pub fn update_vow(
        &mut self,
        actor: Actor,
        id: u32,
        change: &VowUpdate,
    ) -> Result<(), ApiError> {
        let at = self.vow_index(actor, id)?;
        if self.vows[at].rev != change.rev {
            return Err(ApiError::new(
                409,
                "stale",
                "This vow changed; reload and try again",
            ));
        }
        let text = change.text.as_deref().map(check_text).transpose()?;
        let visibility = change
            .visibility
            .as_deref()
            .map(check_visibility)
            .transpose()?;
        let started = parse_date(&self.vows[at].started)?;
        let ends = change
            .ends
            .as_deref()
            .map(|e| self.check_ends(e, started))
            .transpose()?;
        let vow = &mut self.vows[at];
        if let Some(t) = text {
            vow.text = t;
        }
        if let Some(v) = visibility {
            // The "took" event follows the vow.
            if let Some(e) = vow.events.iter_mut().find(|e| e.action == "took") {
                e.visibility = v.clone();
            }
            vow.visibility = v;
        }
        if let Some(e) = ends {
            vow.ends = e;
        }
        vow.rev += 1;
        Ok(())
    }

    /// Pause, resume, breach, repair, complete or retire. A repair records
    /// a repentance entry (`generic`, like every repentance) and links it.
    pub fn vow_action(
        &mut self,
        actor: Actor,
        id: u32,
        action: Action,
        input: &VowActionInput,
        now: Option<u64>,
    ) -> Result<(), ApiError> {
        let at = self.vow_index(actor, id)?;
        let next = transition(&self.vows[at].state, action)?;
        let today = self.today_or_503(now)?;
        let note = check_note(&input.note)?;
        let visibility = match (input.visibility.as_str(), action) {
            ("", Action::Breach | Action::Repair) => "generic".to_string(),
            ("", _) => self.vows[at].visibility.clone(),
            (v, _) => check_visibility(v)?,
        };
        if self.vows[at].events.len() >= EVENTS_MAX {
            return Err(ApiError::new(
                409,
                "full",
                format!("A vow's timeline holds {EVENTS_MAX} events; take it as a new vow"),
            ));
        }
        let entry = if action == Action::Repair {
            let person = self.vows[at].person;
            let input = EntryInput {
                person,
                kind: "repentance".into(),
                note: if note.is_empty() {
                    "Repairing a vow".into()
                } else {
                    note.clone()
                },
                ..Default::default()
            };
            self.record(actor, &input, now)?
        } else {
            0
        };
        let vow = &mut self.vows[at];
        vow.state = next.into();
        vow.rev += 1;
        vow.events.push(StoredVowEvent {
            action: action.name().into(),
            date: today.iso(),
            at: now.unwrap_or(0),
            note,
            visibility,
            entry,
        });
        Ok(())
    }

    /// Deletes a vow and its posts; returns the posts whose bodies are on
    /// the card.
    pub fn delete_vow(&mut self, actor: Actor, id: u32) -> Result<Vec<u32>, ApiError> {
        let at = self.vow_index(actor, id)?;
        self.vows.remove(at);
        let on_card = self
            .posts
            .iter()
            .filter(|p| p.vow == id && p.on_card)
            .map(|p| p.id)
            .collect();
        self.posts.retain(|p| p.vow != id);
        Ok(on_card)
    }

    fn own(viewer: Viewer, person: u32) -> bool {
        viewer.admin || (viewer.id != 0 && viewer.id == person)
    }

    /// `vow` as `viewer` may see it.
    pub fn vow_view(&self, viewer: Viewer, vow: &StoredVow) -> Option<VowView> {
        let own = Self::own(viewer, vow.person);
        if vow.visibility == "private" && !own {
            return None;
        }
        let generic = !own && vow.visibility == "generic";
        let shown = |e: &&StoredVowEvent| own || e.action == "repair" || e.visibility == "public";
        let events: Vec<VowEventView> = if generic {
            Vec::new()
        } else {
            vow.events
                .iter()
                .filter(shown)
                .map(|e| VowEventView {
                    action: e.action.clone(),
                    date: e.date.clone(),
                    label: label(&e.action, own).into(),
                    // A repair's note is a repentance: its person's only.
                    note: if own || (e.action != "repair" && e.visibility == "public") {
                        e.note.clone()
                    } else {
                        String::new()
                    },
                    visibility: e.visibility.clone(),
                })
                .collect()
        };
        // An unshown breach: to others the vow is simply kept.
        let state = if !own
            && vow.state == "breached"
            && vow
                .events
                .iter()
                .rev()
                .find(|e| e.action == "breach")
                .is_some_and(|e| e.visibility != "public")
        {
            "active".to_string()
        } else {
            vow.state.clone()
        };
        let precept_title =
            crate::precepts::item(&vow.precept_set, &vow.precept_kind, vow.precept_n);
        Some(VowView {
            id: vow.id,
            person: vow.person,
            person_name: self.name_of(vow.person),
            text: if generic {
                String::new()
            } else {
                vow.text.clone()
            },
            state,
            started: vow.started.clone(),
            ends: if generic {
                String::new()
            } else {
                vow.ends.clone()
            },
            visibility: vow.visibility.clone(),
            rev: vow.rev,
            precept_set: if generic || precept_title.is_none() {
                String::new()
            } else {
                vow.precept_set.clone()
            },
            precept_kind: if generic || precept_title.is_none() {
                String::new()
            } else {
                vow.precept_kind.clone()
            },
            precept_n: if generic || precept_title.is_none() {
                0
            } else {
                vow.precept_n
            },
            events,
            posts: if generic {
                0
            } else {
                self.posts.iter().filter(|p| p.vow == vow.id).count() as u32
            },
            can_edit: own,
            generic,
            actions: if own {
                ACTIONS
                    .into_iter()
                    .filter(|a| can(*a, &vow.state))
                    .map(|a| a.name().to_string())
                    .collect()
            } else {
                Vec::new()
            },
        })
    }

    /// `post` as `viewer` may see it: only on a vow they can read in full.
    pub fn post_view(
        &self,
        viewer: Viewer,
        post: &StoredPost,
        body: &dyn Fn(&StoredPost) -> String,
    ) -> Option<PostView> {
        let vow = self.vows.iter().find(|v| v.id == post.vow)?;
        let own = Self::own(viewer, post.person);
        if !own && vow.visibility != "public" {
            return None;
        }
        let md = body(post);
        Some(PostView {
            id: post.id,
            vow: post.vow,
            person: post.person,
            person_name: self.name_of(post.person),
            title: post.title.clone(),
            date: post.date.clone(),
            created: post.created,
            rev: post.rev,
            html: markdown::to_html(&md),
            body: if own { md } else { String::new() },
            can_edit: own,
            vow_text: vow.text.clone(),
        })
    }

    fn posts_newest_first<'a>(&'a self, mut posts: Vec<&'a StoredPost>) -> Vec<&'a StoredPost> {
        posts.sort_by(|a, b| {
            b.date
                .cmp(&a.date)
                .then(b.created.cmp(&a.created))
                .then(b.id.cmp(&a.id))
        });
        posts
    }

    /// A profile's vows, precepts and blog (the newest `posts` posts).
    pub fn vows_page(
        &self,
        viewer: Viewer,
        person: u32,
        posts: usize,
        body: &dyn Fn(&StoredPost) -> String,
    ) -> Result<VowsPage, ApiError> {
        let p = self
            .person(person)
            .ok_or_else(|| ApiError::not_found("No such person"))?;
        let mut vows: Vec<VowView> = self
            .vows
            .iter()
            .filter(|v| v.person == person)
            .filter_map(|v| self.vow_view(viewer, v))
            .collect();
        // Kept vows first, then by when they were taken, newest first.
        let rank = |s: &str| match s {
            "active" | "breached" => 0,
            "paused" => 1,
            "completed" => 2,
            _ => 3,
        };
        vows.sort_by(|a, b| {
            rank(&a.state)
                .cmp(&rank(&b.state))
                .then(b.started.cmp(&a.started))
                .then(b.id.cmp(&a.id))
        });
        let mine = self.posts.iter().filter(|q| q.person == person).collect();
        Ok(VowsPage {
            person,
            person_name: p.display_name.clone(),
            own: Self::own(viewer, person),
            vows,
            precepts: self.precepts_of(viewer, person),
            posts: self
                .posts_newest_first(mine)
                .into_iter()
                .filter_map(|q| self.post_view(viewer, q, body))
                .take(posts)
                .collect(),
        })
    }

    pub fn vow_detail(
        &self,
        viewer: Viewer,
        id: u32,
        body: &dyn Fn(&StoredPost) -> String,
    ) -> Result<VowDetail, ApiError> {
        let not_found = || ApiError::not_found("No such vow");
        let stored = self
            .vows
            .iter()
            .find(|v| v.id == id)
            .ok_or_else(not_found)?;
        let vow = self.vow_view(viewer, stored).ok_or_else(not_found)?;
        let posts = self
            .posts_newest_first(self.posts.iter().filter(|p| p.vow == id).collect())
            .into_iter()
            .filter_map(|p| self.post_view(viewer, p, body))
            .collect();
        Ok(VowDetail { vow, posts })
    }

    fn check_post(
        &self,
        input: &PostInput,
        card: bool,
        today: Date,
    ) -> Result<(String, Date, String), ApiError> {
        let title = input.title.split_whitespace().collect::<Vec<_>>().join(" ");
        if !(1..=TITLE_MAX).contains(&title.chars().count()) {
            return Err(ApiError::bad_request(format!(
                "A title is 1 to {TITLE_MAX} characters"
            )));
        }
        let body = input.body.replace("\r\n", "\n").trim().to_string();
        let max = if card { BODY_MAX } else { SNAPSHOT_BODY_MAX };
        if body.len() > max {
            return Err(ApiError::bad_request(if card {
                format!("A post is at most {max} bytes")
            } else {
                format!("Without an SD card a post is at most {max} bytes")
            }));
        }
        let date = if input.date.is_empty() {
            today
        } else {
            parse_date(&input.date)?
        };
        if date > today {
            return Err(ApiError::bad_request("That date hasn't happened yet"));
        }
        Ok((title, date, body))
    }

    /// Bodies kept in the snapshot, without `except`.
    fn snapshot_body_bytes(&self, except: u32) -> usize {
        self.posts
            .iter()
            .filter(|p| p.id != except)
            .map(|p| p.body.len())
            .sum()
    }

    fn place_body(
        &self,
        id: u32,
        body: String,
        card: bool,
    ) -> Result<(String, BodyWrite), ApiError> {
        if card {
            return Ok((String::new(), BodyWrite::Card { id, body }));
        }
        if self.snapshot_body_bytes(id) + body.len() > self.limits.snapshot_post_bytes {
            return Err(ApiError::new(
                409,
                "full",
                "Without an SD card the board has no room for more posts",
            ));
        }
        Ok((body, BodyWrite::None))
    }

    /// Adds a post to vow `vow`; `card` says whether the SD card is there.
    pub fn add_post(
        &mut self,
        actor: Actor,
        vow: u32,
        input: &PostInput,
        card: bool,
        now: Option<u64>,
    ) -> Result<(u32, BodyWrite), ApiError> {
        let at = self.vow_index(actor, vow)?;
        let today = self.today_or_503(now)?;
        let (title, date, body) = self.check_post(input, card, today)?;
        if self.posts.len() >= self.limits.posts {
            return Err(ApiError::new(
                409,
                "full",
                format!("The board holds at most {} posts", self.limits.posts),
            ));
        }
        let id = self.next_post;
        let (body, write) = self.place_body(id, body, card)?;
        self.next_post += 1;
        self.posts.push(StoredPost {
            id,
            vow,
            person: self.vows[at].person,
            title,
            date: date.iso(),
            created: now.unwrap_or(0),
            rev: 1,
            body,
            on_card: card,
        });
        Ok((id, write))
    }

    fn post_index(&self, actor: Actor, id: u32) -> Result<usize, ApiError> {
        let (me, admin) = person_of(actor)?;
        let at = self
            .posts
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| ApiError::not_found("No such post"))?;
        if self.posts[at].person != me && !admin {
            return Err(forbidden("Only its person can change a post"));
        }
        Ok(at)
    }

    pub fn update_post(
        &mut self,
        actor: Actor,
        id: u32,
        input: &PostInput,
        card: bool,
        now: Option<u64>,
    ) -> Result<BodyWrite, ApiError> {
        let at = self.post_index(actor, id)?;
        if self.posts[at].rev != input.rev {
            return Err(ApiError::new(
                409,
                "stale",
                "This post changed; reload and try again",
            ));
        }
        let today = self.today_or_503(now)?;
        let (title, date, body) = self.check_post(input, card, today)?;
        let (body, write) = self.place_body(id, body, card)?;
        let post = &mut self.posts[at];
        post.title = title;
        post.date = date.iso();
        post.body = body;
        post.on_card = card;
        post.rev += 1;
        Ok(write)
    }

    /// Deletes a post; true if its body was on the card.
    pub fn delete_post(&mut self, actor: Actor, id: u32) -> Result<bool, ApiError> {
        let at = self.post_index(actor, id)?;
        Ok(self.posts.remove(at).on_card)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::PreceptsInput;
    use crate::state::{Limits, GUEST};

    // ---- the state machine (v1's tests/test_vows.py) ----------------------

    #[test]
    fn breach_moves_to_repair() {
        assert_eq!(transition("active", Action::Breach).unwrap(), "breached");
    }

    #[test]
    fn cannot_pause_retired() {
        assert!(!can(Action::Pause, "retired"));
        assert_eq!(
            transition("retired", Action::Pause).unwrap_err().status,
            409
        );
    }

    #[test]
    fn resume_only_from_paused() {
        assert!(can(Action::Resume, "paused"));
        assert!(!can(Action::Resume, "active"));
    }

    #[test]
    fn retire_from_anywhere_but_retired() {
        for s in ["active", "paused", "breached", "completed"] {
            assert!(can(Action::Retire, s), "{s}");
        }
        assert!(!can(Action::Retire, "retired"));
    }

    #[test]
    fn repair_only_after_a_breach() {
        assert!(!can(Action::Repair, "active"));
        assert_eq!(transition("breached", Action::Repair).unwrap(), "active");
        for a in ACTIONS {
            assert_eq!(Action::parse(a.name()), Some(a));
        }
    }

    // ---- vows in the state -------------------------------------------------

    const NOW: Option<u64> = Some(1_792_000_000); // 2026-10-14 in New York

    fn state() -> (State, u32, u32) {
        let mut s = State::new(Limits {
            pin_rounds: 2,
            ..Limits::board()
        });
        let dad = s.setup("Dad", "1234", 0).unwrap();
        let guest = s.add_person("Guest", GUEST, "1111", "", 0).unwrap();
        (s, dad, guest)
    }

    fn me(id: u32) -> Actor {
        Actor::Person {
            id,
            admin: false,
            guest: false,
        }
    }

    fn sees(id: u32) -> Viewer {
        Viewer { id, admin: false }
    }

    fn vow(text: &str, visibility: &str) -> VowInput {
        VowInput {
            text: text.into(),
            visibility: visibility.into(),
            ..Default::default()
        }
    }

    fn act(s: &mut State, who: u32, id: u32, a: Action) -> Result<(), ApiError> {
        s.vow_action(me(who), id, a, &VowActionInput::default(), NOW)
    }

    #[test]
    fn a_vows_life() {
        let (mut s, dad, _) = state();
        let id = s
            .add_vow(me(dad), &vow("Sit every morning", ""), NOW)
            .unwrap();
        act(&mut s, dad, id, Action::Pause).unwrap();
        assert_eq!(act(&mut s, dad, id, Action::Pause).unwrap_err().status, 409);
        act(&mut s, dad, id, Action::Resume).unwrap();
        act(&mut s, dad, id, Action::Complete).unwrap();
        act(&mut s, dad, id, Action::Retire).unwrap();
        let v = &s.vows[0];
        assert_eq!(v.state, "retired");
        let actions: Vec<_> = v.events.iter().map(|e| e.action.as_str()).collect();
        assert_eq!(actions, ["took", "pause", "resume", "complete", "retire"]);
        assert!(s.vow_view(sees(dad), v).unwrap().actions.is_empty());
    }

    #[test]
    fn a_repair_records_repentance_and_others_see_renewal() {
        let (mut s, dad, guest) = state();
        let id = s.add_vow(me(dad), &vow("Right speech", ""), NOW).unwrap();
        act(&mut s, dad, id, Action::Breach).unwrap();
        let theirs = s.vow_view(sees(guest), &s.vows[0]).unwrap();
        assert_eq!(theirs.state, "active", "an unshown breach stays private");
        assert_eq!(theirs.events.len(), 1);
        s.vow_action(
            me(dad),
            id,
            Action::Repair,
            &VowActionInput {
                note: "Apologized to my sister".into(),
                ..Default::default()
            },
            NOW,
        )
        .unwrap();
        let v = &s.vows[0];
        assert_eq!(v.state, "active");
        let repair = v.events.last().unwrap();
        let entry = s.entries.iter().find(|e| e.id == repair.entry).unwrap();
        assert_eq!((entry.kind.as_str(), entry.person), ("repentance", dad));
        assert_eq!(entry.visibility, "generic");
        let theirs = s.vow_view(sees(guest), v).unwrap();
        let labels: Vec<_> = theirs.events.iter().map(|e| e.label.as_str()).collect();
        assert_eq!(labels, ["Took this vow", "Renewed this vow"]);
        assert!(theirs.events.iter().all(|e| e.note.is_empty()));
        let mine = s.vow_view(sees(dad), v).unwrap();
        assert_eq!(mine.events.len(), 3);
        assert_eq!(mine.events[2].note, "Apologized to my sister");
    }

    #[test]
    fn a_shown_breach_is_shown() {
        let (mut s, dad, guest) = state();
        let id = s.add_vow(me(dad), &vow("No alcohol", ""), NOW).unwrap();
        s.vow_action(
            me(dad),
            id,
            Action::Breach,
            &VowActionInput {
                note: "A toast at the wedding".into(),
                visibility: "public".into(),
            },
            NOW,
        )
        .unwrap();
        let theirs = s.vow_view(sees(guest), &s.vows[0]).unwrap();
        assert_eq!(theirs.state, "breached");
        assert_eq!(theirs.events[1].note, "A toast at the wedding");
    }

    #[test]
    fn visibility_matches_entries() {
        let (mut s, dad, guest) = state();
        s.add_vow(me(dad), &vow("Public one", "public"), NOW)
            .unwrap();
        s.add_vow(me(dad), &vow("Generic one", "generic"), NOW)
            .unwrap();
        s.add_vow(me(dad), &vow("Private one", "private"), NOW)
            .unwrap();
        let none = |_: &StoredPost| String::new();
        for (viewer, texts) in [
            (sees(0), vec!["", "Public one"]),
            (sees(guest), vec!["", "Public one"]),
            (sees(dad), vec!["Private one", "Generic one", "Public one"]),
            (
                Viewer {
                    id: guest,
                    admin: true,
                },
                vec!["Private one", "Generic one", "Public one"],
            ),
        ] {
            let page = s.vows_page(viewer, dad, 20, &none).unwrap();
            let got: Vec<_> = page.vows.iter().map(|v| v.text.as_str()).collect();
            assert_eq!(got, texts, "{viewer:?}");
        }
    }

    #[test]
    fn only_its_person_changes_a_vow() {
        let (mut s, dad, guest) = state();
        let id = s.add_vow(me(dad), &vow("Mine", ""), NOW).unwrap();
        assert_eq!(
            act(&mut s, guest, id, Action::Pause).unwrap_err().status,
            403
        );
        assert_eq!(s.delete_vow(me(guest), id).unwrap_err().status, 403);
        assert_eq!(
            s.vow_action(
                Actor::Board,
                id,
                Action::Pause,
                &VowActionInput::default(),
                NOW
            )
            .unwrap_err()
            .status,
            403
        );
        let admin = Actor::Person {
            id: guest,
            admin: true,
            guest: false,
        };
        s.vow_action(admin, id, Action::Pause, &VowActionInput::default(), NOW)
            .unwrap();
    }

    #[test]
    fn vows_are_checked() {
        let (mut s, dad, _) = state();
        assert_eq!(
            s.add_vow(me(dad), &vow("", ""), NOW).unwrap_err().status,
            400
        );
        assert_eq!(
            s.add_vow(me(dad), &vow(&"x".repeat(281), ""), NOW)
                .unwrap_err()
                .status,
            400
        );
        assert_eq!(
            s.add_vow(me(dad), &vow("x", "secret"), NOW)
                .unwrap_err()
                .status,
            400
        );
        let late = VowInput {
            started: "2026-10-30".into(),
            ..vow("x", "")
        };
        assert_eq!(s.add_vow(me(dad), &late, NOW).unwrap_err().status, 400);
        let backwards = VowInput {
            started: "2026-10-01".into(),
            ends: "2026-09-01".into(),
            ..vow("x", "")
        };
        assert_eq!(s.add_vow(me(dad), &backwards, NOW).unwrap_err().status, 400);
        assert_eq!(
            s.add_vow(me(dad), &vow("x", ""), None).unwrap_err().status,
            503
        );
        s.limits.vows = 1;
        s.add_vow(me(dad), &vow("one", ""), NOW).unwrap();
        let err = s.add_vow(me(dad), &vow("two", ""), NOW).unwrap_err();
        assert_eq!((err.status, err.code), (409, "full"));
    }

    #[test]
    fn edits_check_the_revision() {
        let (mut s, dad, _) = state();
        let id = s.add_vow(me(dad), &vow("Sit", ""), NOW).unwrap();
        let change = VowUpdate {
            rev: 1,
            text: Some("Sit daily".into()),
            visibility: Some("generic".into()),
            ..Default::default()
        };
        s.update_vow(me(dad), id, &change).unwrap();
        assert_eq!(
            s.update_vow(me(dad), id, &change).unwrap_err().code,
            "stale"
        );
        assert_eq!(s.vows[0].text, "Sit daily");
        assert_eq!(s.vows[0].events[0].visibility, "generic");
    }

    // ---- precepts as vows --------------------------------------------------

    fn take(s: &mut State, who: u32, set: &str, majors: &[u32], minors: &[u32]) {
        let input = PreceptsInput {
            majors: majors.to_vec(),
            minors: minors.to_vec(),
            ..Default::default()
        };
        s.set_precepts(me(who), set, &input, NOW).unwrap();
    }

    fn precept_vow(set: &str, kind: &str, n: u32) -> VowInput {
        VowInput {
            precept_set: set.into(),
            precept_kind: kind.into(),
            precept_n: n,
            ..Default::default()
        }
    }

    #[test]
    fn a_taken_precept_becomes_a_vow() {
        let (mut s, dad, guest) = state();
        let err = s
            .add_vow(me(dad), &precept_vow("five", "major", 1), NOW)
            .unwrap_err();
        assert_eq!(err.code, "not_taken");
        take(&mut s, dad, "five", &[1, 4], &[]);
        assert_eq!(
            s.add_vow(me(dad), &precept_vow("five", "major", 2), NOW)
                .unwrap_err()
                .code,
            "not_taken"
        );
        assert_eq!(
            s.add_vow(me(dad), &precept_vow("five", "major", 9), NOW)
                .unwrap_err()
                .status,
            400
        );
        let id = s
            .add_vow(me(dad), &precept_vow("five", "major", 1), NOW)
            .unwrap();
        let v = s
            .vow_view(sees(guest), s.vows.iter().find(|v| v.id == id).unwrap())
            .unwrap();
        assert_eq!(v.text, "Do not kill.");
        assert_eq!((v.precept_set.as_str(), v.precept_n), ("five", 1));
        assert_eq!(
            s.add_vow(me(dad), &precept_vow("five", "major", 1), NOW)
                .unwrap_err()
                .code,
            "already"
        );
        act(&mut s, dad, id, Action::Retire).unwrap();
        s.add_vow(me(dad), &precept_vow("five", "major", 1), NOW)
            .unwrap();
    }

    // ---- posts ---------------------------------------------------------------

    fn post(title: &str, body: &str) -> PostInput {
        PostInput {
            title: title.into(),
            body: body.into(),
            ..Default::default()
        }
    }

    #[test]
    fn posts_follow_their_vow() {
        let (mut s, dad, guest) = state();
        let open = s.add_vow(me(dad), &vow("Open", "public"), NOW).unwrap();
        let quiet = s.add_vow(me(dad), &vow("Quiet", "generic"), NOW).unwrap();
        let (a, write) = s
            .add_post(me(dad), open, &post("Day one", "*Sat*"), true, NOW)
            .unwrap();
        assert_eq!(
            write,
            BodyWrite::Card {
                id: a,
                body: "*Sat*".into()
            }
        );
        s.add_post(me(dad), quiet, &post("Hidden", "x"), true, NOW)
            .unwrap();
        assert_eq!(
            s.add_post(me(guest), open, &post("Not mine", "x"), true, NOW)
                .unwrap_err()
                .status,
            403
        );
        let body = |p: &StoredPost| {
            if p.id == a {
                "*Sat*".to_string()
            } else {
                "x".to_string()
            }
        };
        let theirs = s.vows_page(sees(guest), dad, 20, &body).unwrap();
        assert_eq!(theirs.posts.len(), 1);
        assert_eq!(theirs.posts[0].html, "<p><em>Sat</em></p>");
        assert!(
            theirs.posts[0].body.is_empty(),
            "markdown is for its author"
        );
        assert_eq!(
            s.vows_page(sees(dad), dad, 20, &body).unwrap().posts.len(),
            2
        );
        assert!(s
            .vow_detail(sees(guest), quiet, &body)
            .unwrap()
            .posts
            .is_empty());
        assert_eq!(s.delete_vow(me(dad), open).unwrap(), vec![a]);
        assert_eq!(s.posts.len(), 1);
    }

    #[test]
    fn without_a_card_posts_have_a_quota() {
        let (mut s, dad, _) = state();
        let id = s.add_vow(me(dad), &vow("Sit", ""), NOW).unwrap();
        assert_eq!(
            s.add_post(me(dad), id, &post("Long", &"x".repeat(2_001)), false, NOW)
                .unwrap_err()
                .status,
            400
        );
        s.limits.snapshot_post_bytes = 3_000;
        let (p, write) = s
            .add_post(me(dad), id, &post("One", &"x".repeat(2_000)), false, NOW)
            .unwrap();
        assert_eq!(write, BodyWrite::None);
        assert_eq!(s.posts[0].body.len(), 2_000);
        let err = s
            .add_post(me(dad), id, &post("Two", &"x".repeat(1_500)), false, NOW)
            .unwrap_err();
        assert_eq!((err.status, err.code), (409, "full"));
        // Editing a post may reuse its own room.
        let edit = PostInput {
            rev: 1,
            ..post("One", &"y".repeat(1_900))
        };
        s.update_post(me(dad), p, &edit, false, NOW).unwrap();
        assert_eq!(
            s.update_post(me(dad), p, &edit, false, NOW)
                .unwrap_err()
                .code,
            "stale"
        );
    }
}
