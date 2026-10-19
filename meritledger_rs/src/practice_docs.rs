//! Build-a-practice (`sprints/sprint08.md`, `spec/domain.md` § Texts and
//! practices): household texts and practice documents.
//!
//! - Built-in texts ship with the UI (`meritledger_ui/public/texts/`); the
//!   board only knows their ids. Household texts (`h<id>`) are the
//!   household's own, with bodies on the SD card (`ml/texts/<id>.md`), or in
//!   the snapshot under a small quota without one.
//! - A practice is an ordered list of elements. Public ones can be followed
//!   by anyone on the Wi-Fi (readers at a group practice needn't sign in).
//! - "I did this" records an entry for each element that logs as a
//!   practice kind: 20 minutes of meditation, 108 daimoku, and so on.
use crate::ledger::{Actor, Viewer};
use crate::messages::{
    EntryInput, PracticeInput, PracticeView, StoredElement, StoredPracticeDoc, StoredText,
    TextInput, TextView,
};
use crate::state::State;
use crate::vows::BodyWrite;
use miniframework::ApiError;

pub const KINDS: [&str; 8] = [
    "meditate",
    "chant",
    "prostrations",
    "recite",
    "read",
    "copy",
    "art",
    "note",
];
pub const ELEMENTS_MAX: usize = 40;
pub const TITLE_MAX: usize = 100;
pub const NOTE_MAX: usize = 1_000;
/// A household text on the card, and in the snapshot without one.
pub const TEXT_MAX: usize = 32 * 1024;
pub const SNAPSHOT_TEXT_MAX: usize = 4 * 1024;

fn forbidden(message: &str) -> ApiError {
    ApiError::new(403, "forbidden", message.to_string())
}

fn person_of(actor: Actor, what: &str) -> Result<(u32, bool, bool), ApiError> {
    match actor {
        Actor::Person { id, admin, guest } => Ok((id, admin, guest)),
        Actor::Board => Err(forbidden(&format!("The board can't change {what}"))),
    }
}

fn check_title(raw: &str) -> Result<String, ApiError> {
    let title = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if (1..=TITLE_MAX).contains(&title.chars().count()) {
        Ok(title)
    } else {
        Err(ApiError::bad_request(format!(
            "A title is 1 to {TITLE_MAX} characters"
        )))
    }
}

/// A built-in text id: lower-case letters, digits and dashes.
fn is_builtin_id(id: &str) -> bool {
    (1..=40).contains(&id.len())
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !id.starts_with('-')
}

/// `h12` -> 12.
pub fn household_id(id: &str) -> Option<u32> {
    id.strip_prefix('h')?.parse().ok()
}

impl State {
    // ---- texts -------------------------------------------------------------

    fn text_view(&self, viewer: Viewer, t: &StoredText, body: String) -> TextView {
        TextView {
            id: format!("h{}", t.id),
            title: t.title.clone(),
            tradition: t.tradition.clone(),
            license: t.license.clone(),
            owner: t.owner,
            owner_name: self.name_of(t.owner),
            rev: t.rev,
            body,
            can_edit: viewer.admin || (viewer.id != 0 && viewer.id == t.owner),
        }
    }

    /// The household's texts, by title.
    pub fn texts_for(&self, viewer: Viewer) -> Vec<TextView> {
        let mut list: Vec<TextView> = self
            .texts
            .iter()
            .map(|t| self.text_view(viewer, t, String::new()))
            .collect();
        list.sort_by_key(|t| t.title.to_lowercase());
        list
    }

    pub fn text_with_body(
        &self,
        viewer: Viewer,
        id: u32,
        body: &dyn Fn(&StoredText) -> String,
    ) -> Result<TextView, ApiError> {
        let t = self
            .texts
            .iter()
            .find(|t| t.id == id)
            .ok_or_else(|| ApiError::not_found("No such text"))?;
        Ok(self.text_view(viewer, t, body(t)))
    }

    fn checked_text(&self, input: &TextInput, card: bool) -> Result<(String, String), ApiError> {
        let title = check_title(&input.title)?;
        let body = input.body.replace("\r\n", "\n").trim().to_string();
        let max = if card { TEXT_MAX } else { SNAPSHOT_TEXT_MAX };
        if body.is_empty() || body.len() > max {
            return Err(ApiError::bad_request(if card {
                format!("A text is 1 to {max} bytes")
            } else {
                format!("Without an SD card a text is at most {max} bytes")
            }));
        }
        for (field, value) in [("Tradition", &input.tradition), ("License", &input.license)] {
            if value.chars().count() > 100 {
                return Err(ApiError::bad_request(format!(
                    "{field}: at most 100 characters"
                )));
            }
        }
        Ok((title, body))
    }

    fn place_text(
        &self,
        id: u32,
        body: String,
        card: bool,
    ) -> Result<(String, BodyWrite), ApiError> {
        if card {
            return Ok((String::new(), BodyWrite::Card { id, body }));
        }
        let used: usize = self
            .texts
            .iter()
            .filter(|t| t.id != id)
            .map(|t| t.body.len())
            .sum();
        if used + body.len() > self.limits.snapshot_text_bytes {
            return Err(ApiError::new(
                409,
                "full",
                "Without an SD card the board has no room for more texts",
            ));
        }
        Ok((body, BodyWrite::None))
    }

    pub fn add_text(
        &mut self,
        actor: Actor,
        input: &TextInput,
        card: bool,
        now: Option<u64>,
    ) -> Result<(u32, BodyWrite), ApiError> {
        let (owner, _, guest) = person_of(actor, "texts")?;
        if guest {
            return Err(ApiError::new(
                403,
                "guest",
                "Guests can't add to the household's texts",
            ));
        }
        let (title, body) = self.checked_text(input, card)?;
        if self.texts.len() >= self.limits.texts {
            return Err(ApiError::new(
                409,
                "full",
                format!("The board holds at most {} texts", self.limits.texts),
            ));
        }
        let id = self.next_text;
        let (body, write) = self.place_text(id, body, card)?;
        self.next_text += 1;
        self.texts.push(StoredText {
            id,
            owner,
            title,
            license: input.license.trim().to_string(),
            tradition: input.tradition.trim().to_string(),
            rev: 1,
            created: now.unwrap_or(0),
            body,
            on_card: card,
        });
        Ok((id, write))
    }

    fn text_index(&self, actor: Actor, id: u32) -> Result<usize, ApiError> {
        let (me, admin, _) = person_of(actor, "texts")?;
        let at = self
            .texts
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| ApiError::not_found("No such text"))?;
        if self.texts[at].owner != me && !admin {
            return Err(forbidden(
                "Only who added a text (or an admin) can change it",
            ));
        }
        Ok(at)
    }

    pub fn update_text(
        &mut self,
        actor: Actor,
        id: u32,
        input: &TextInput,
        card: bool,
    ) -> Result<BodyWrite, ApiError> {
        let at = self.text_index(actor, id)?;
        if self.texts[at].rev != input.rev {
            return Err(ApiError::new(
                409,
                "stale",
                "This text changed; reload and try again",
            ));
        }
        let (title, body) = self.checked_text(input, card)?;
        let (body, write) = self.place_text(id, body, card)?;
        let t = &mut self.texts[at];
        t.title = title;
        t.body = body;
        t.on_card = card;
        t.license = input.license.trim().to_string();
        t.tradition = input.tradition.trim().to_string();
        t.rev += 1;
        Ok(write)
    }

    /// Deletes a household text; true if its body was on the card. A
    /// practice that uses it keeps the reference and shows it as missing.
    pub fn delete_text(&mut self, actor: Actor, id: u32) -> Result<bool, ApiError> {
        let at = self.text_index(actor, id)?;
        Ok(self.texts.remove(at).on_card)
    }

    // ---- practices ---------------------------------------------------------

    fn checked_elements(
        &self,
        owner: u32,
        elements: &[StoredElement],
    ) -> Result<Vec<StoredElement>, ApiError> {
        if !(1..=ELEMENTS_MAX).contains(&elements.len()) {
            return Err(ApiError::bad_request(format!(
                "A practice has 1 to {ELEMENTS_MAX} steps"
            )));
        }
        elements
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let bad = |m: String| Err(ApiError::bad_request(format!("Step {}: {m}", i + 1)));
                if !KINDS.contains(&e.kind.as_str()) {
                    return bad(format!("a step is one of {}", KINDS.join(", ")));
                }
                let title = e.title.split_whitespace().collect::<Vec<_>>().join(" ");
                if title.chars().count() > TITLE_MAX {
                    return bad(format!("a heading is at most {TITLE_MAX} characters"));
                }
                let text = e.text.trim();
                let text_ok = text.is_empty()
                    || household_id(text).is_some_and(|id| self.texts.iter().any(|t| t.id == id))
                    || (household_id(text).is_none() && is_builtin_id(text));
                if !text_ok {
                    return bad("no such text".into());
                }
                if e.minutes > 600 {
                    return bad("up to 600 minutes".into());
                }
                if e.count > 100_000 {
                    return bad("a count up to 100,000".into());
                }
                let timer = e.timer.trim();
                if !timer.is_empty() && !self.timers_for(owner).iter().any(|t| t.key == timer) {
                    return bad("no such timer".into());
                }
                let logs_as = e.logs_as.trim();
                if !logs_as.is_empty() {
                    let usable = self
                        .kind(logs_as)
                        .is_some_and(|k| (k.owner == 0 || k.owner == owner) && !k.archived);
                    if !usable {
                        return bad("it logs as a practice that doesn't exist".into());
                    }
                }
                let note = e.note.trim();
                if note.chars().count() > NOTE_MAX {
                    return bad(format!("notes are at most {NOTE_MAX} characters"));
                }
                Ok(StoredElement {
                    kind: e.kind.clone(),
                    title,
                    text: text.to_string(),
                    minutes: e.minutes,
                    count: e.count,
                    timer: timer.to_string(),
                    logs_as: logs_as.to_string(),
                    note: note.to_string(),
                })
            })
            .collect()
    }

    fn check_doc_visibility(v: &str) -> Result<String, ApiError> {
        match v {
            "" | "public" => Ok("public".into()),
            "private" => Ok("private".into()),
            _ => Err(ApiError::bad_request(
                "visibility must be public or private",
            )),
        }
    }

    pub fn practice_view(&self, viewer: Viewer, p: &StoredPracticeDoc) -> Option<PracticeView> {
        let own = viewer.admin || (viewer.id != 0 && viewer.id == p.owner);
        if p.visibility == "private" && !own {
            return None;
        }
        Some(PracticeView {
            id: p.id,
            owner: p.owner,
            owner_name: self.name_of(p.owner),
            title: p.title.clone(),
            visibility: p.visibility.clone(),
            elements: p.elements.clone(),
            rev: p.rev,
            can_edit: own,
        })
    }

    /// Practices `viewer` may see: their own first, then by title.
    pub fn practices_for(&self, viewer: Viewer) -> Vec<PracticeView> {
        let mut list: Vec<PracticeView> = self
            .practices
            .iter()
            .filter_map(|p| self.practice_view(viewer, p))
            .collect();
        list.sort_by(|a, b| {
            (a.owner != viewer.id)
                .cmp(&(b.owner != viewer.id))
                .then(a.title.to_lowercase().cmp(&b.title.to_lowercase()))
        });
        list
    }

    pub fn practice(&self, viewer: Viewer, id: u32) -> Result<PracticeView, ApiError> {
        self.practices
            .iter()
            .find(|p| p.id == id)
            .and_then(|p| self.practice_view(viewer, p))
            .ok_or_else(|| ApiError::not_found("No such practice"))
    }

    pub fn add_practice(
        &mut self,
        actor: Actor,
        input: &PracticeInput,
        now: Option<u64>,
    ) -> Result<u32, ApiError> {
        let (owner, _, _) = person_of(actor, "practices")?;
        let title = check_title(&input.title)?;
        let visibility = Self::check_doc_visibility(&input.visibility)?;
        let elements = self.checked_elements(owner, &input.elements)?;
        if self.practices.len() >= self.limits.practices {
            return Err(ApiError::new(
                409,
                "full",
                format!(
                    "The board holds at most {} practices",
                    self.limits.practices
                ),
            ));
        }
        let id = self.next_practice;
        self.next_practice += 1;
        self.practices.push(StoredPracticeDoc {
            id,
            owner,
            title,
            visibility,
            elements,
            rev: 1,
            created: now.unwrap_or(0),
        });
        Ok(id)
    }

    fn practice_index(&self, actor: Actor, id: u32) -> Result<usize, ApiError> {
        let (me, admin, _) = person_of(actor, "practices")?;
        let at = self
            .practices
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| ApiError::not_found("No such practice"))?;
        if self.practices[at].owner != me && !admin {
            return Err(forbidden(
                "Only who built a practice (or an admin) can change it",
            ));
        }
        Ok(at)
    }

    pub fn replace_practice(
        &mut self,
        actor: Actor,
        id: u32,
        input: &PracticeInput,
    ) -> Result<(), ApiError> {
        let at = self.practice_index(actor, id)?;
        if self.practices[at].rev != input.rev {
            return Err(ApiError::new(
                409,
                "stale",
                "This practice changed; reload and try again",
            ));
        }
        let owner = self.practices[at].owner;
        let title = check_title(&input.title)?;
        let visibility = Self::check_doc_visibility(&input.visibility)?;
        let elements = self.checked_elements(owner, &input.elements)?;
        let p = &mut self.practices[at];
        p.title = title;
        p.visibility = visibility;
        p.elements = elements;
        p.rev += 1;
        Ok(())
    }

    pub fn delete_practice(&mut self, actor: Actor, id: u32) -> Result<(), ApiError> {
        let at = self.practice_index(actor, id)?;
        self.practices.remove(at);
        Ok(())
    }

    /// "I did this": one entry per element that logs as a practice, for
    /// the actor, on `date` ("" = today). Returns the entry ids.
    pub fn did_practice(
        &mut self,
        actor: Actor,
        id: u32,
        date: &str,
        now: Option<u64>,
    ) -> Result<Vec<u32>, ApiError> {
        let (me, admin, _) = person_of(actor, "practices")?;
        let doc = self.practice(Viewer { id: me, admin }, id)?.clone();
        let mut ids = Vec::new();
        for e in doc.elements.iter().filter(|e| !e.logs_as.is_empty()) {
            let Some(kind) = self.kind(&e.logs_as) else {
                continue;
            };
            let amount = match kind.unit.as_str() {
                "minutes" if e.minutes > 0 => Some(e.minutes),
                "count" if e.count > 0 => Some(e.count),
                _ => None,
            };
            let input = EntryInput {
                kind: kind.key.clone(),
                amount,
                date: (!date.is_empty()).then(|| date.to_string()),
                note: doc.title.chars().take(200).collect(),
                source: Some("manual".into()),
                ..Default::default()
            };
            ids.push(self.record(actor, &input, now)?);
        }
        if ids.is_empty() {
            return Err(ApiError::bad_request(
                "Nothing in this practice records a practice; edit a step's \"Logs as\"",
            ));
        }
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Limits, GUEST};

    const NOW: Option<u64> = Some(1_792_000_000);

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

    fn step(kind: &str, text: &str, logs_as: &str, minutes: u32, count: u32) -> StoredElement {
        StoredElement {
            kind: kind.into(),
            text: text.into(),
            logs_as: logs_as.into(),
            minutes,
            count,
            ..Default::default()
        }
    }

    fn gongyo() -> PracticeInput {
        PracticeInput {
            title: "Morning gongyo".into(),
            visibility: "public".into(),
            elements: vec![
                step("recite", "lotus-2", "recite", 0, 0),
                step("recite", "lotus-16-verses", "", 0, 0),
                StoredElement {
                    timer: "mala108".into(),
                    ..step("chant", "daimoku", "daimoku", 0, 108)
                },
                step("meditate", "", "meditation", 10, 0),
                step("note", "", "", 0, 0),
            ],
            rev: 0,
        }
    }

    #[test]
    fn i_did_this_logs_each_step() {
        let (mut s, dad, _) = state();
        let id = s.add_practice(me(dad), &gongyo(), NOW).unwrap();
        let ids = s.did_practice(me(dad), id, "", NOW).unwrap();
        assert_eq!(ids.len(), 3);
        let logged: Vec<(&str, u32)> = s
            .entries
            .iter()
            .map(|e| (e.kind.as_str(), e.amount))
            .collect();
        assert_eq!(
            logged,
            [("recite", 0), ("daimoku", 108), ("meditation", 10)]
        );
        assert!(s
            .entries
            .iter()
            .all(|e| e.note == "Morning gongyo" && e.person == dad));
    }

    #[test]
    fn steps_are_checked() {
        let (mut s, dad, _) = state();
        let with = |e: StoredElement| PracticeInput {
            elements: vec![e],
            ..gongyo()
        };
        for e in [
            step("dance", "", "", 0, 0),
            step("recite", "../../etc/passwd", "", 0, 0),
            step("recite", "h9", "", 0, 0),
            step("meditate", "", "", 601, 0),
            step("chant", "", "nonsense", 0, 0),
            StoredElement {
                timer: "nope".into(),
                ..step("meditate", "", "", 10, 0)
            },
        ] {
            assert_eq!(
                s.add_practice(me(dad), &with(e.clone()), NOW)
                    .unwrap_err()
                    .status,
                400,
                "{e:?}"
            );
        }
        let empty = PracticeInput {
            elements: vec![],
            ..gongyo()
        };
        assert_eq!(
            s.add_practice(me(dad), &empty, NOW).unwrap_err().status,
            400
        );
        let too_many = PracticeInput {
            elements: vec![step("note", "", "", 0, 0); 41],
            ..gongyo()
        };
        assert_eq!(
            s.add_practice(me(dad), &too_many, NOW).unwrap_err().status,
            400
        );
    }

    #[test]
    fn private_practices_and_ownership() {
        let (mut s, dad, guest) = state();
        let private = PracticeInput {
            visibility: "private".into(),
            ..gongyo()
        };
        let id = s.add_practice(me(dad), &private, NOW).unwrap();
        let theirs = Viewer {
            id: guest,
            admin: false,
        };
        assert!(s.practices_for(theirs).is_empty());
        assert_eq!(s.practice(theirs, id).unwrap_err().status, 404);
        assert_eq!(
            s.did_practice(me(guest), id, "", NOW).unwrap_err().status,
            404
        );
        let public = s.add_practice(me(dad), &gongyo(), NOW).unwrap();
        assert_eq!(s.practices_for(theirs).len(), 1);
        // Anyone may do a public practice; only its builder changes it.
        s.did_practice(me(guest), public, "", NOW).unwrap();
        assert_eq!(
            s.replace_practice(me(guest), public, &gongyo())
                .unwrap_err()
                .status,
            403
        );
        let edit = PracticeInput {
            rev: 1,
            title: "Evening gongyo".into(),
            ..gongyo()
        };
        s.replace_practice(me(dad), public, &edit).unwrap();
        assert_eq!(
            s.replace_practice(me(dad), public, &edit).unwrap_err().code,
            "stale"
        );
        s.delete_practice(me(dad), public).unwrap();
    }

    #[test]
    fn household_texts() {
        let (mut s, dad, guest) = state();
        let input = TextInput {
            title: "Our dedication".into(),
            body: "May all beings be well.".into(),
            ..Default::default()
        };
        let as_guest = Actor::Person {
            id: guest,
            admin: false,
            guest: true,
        };
        assert_eq!(
            s.add_text(as_guest, &input, true, NOW).unwrap_err().status,
            403
        );
        let (id, write) = s.add_text(me(dad), &input, true, NOW).unwrap();
        assert_eq!(
            write,
            BodyWrite::Card {
                id,
                body: "May all beings be well.".into()
            }
        );
        // A practice can use it by `h<id>`.
        let uses = PracticeInput {
            elements: vec![step("recite", &format!("h{id}"), "", 0, 0)],
            ..gongyo()
        };
        s.add_practice(me(dad), &uses, NOW).unwrap();
        // Without a card: a small quota.
        s.limits.snapshot_text_bytes = 30;
        let small = TextInput {
            title: "Short".into(),
            body: "x".repeat(25),
            ..Default::default()
        };
        s.add_text(me(dad), &small, false, NOW).unwrap();
        let err = s.add_text(me(dad), &small, false, NOW).unwrap_err();
        assert_eq!((err.status, err.code), (409, "full"));
        assert!(s.delete_text(me(dad), id).unwrap());
    }
}
