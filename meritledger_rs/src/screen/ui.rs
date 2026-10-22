//! The board's touch menu (Sprint 13): tap for the next card, hold for
//! the menu; record a practice for someone (with five seconds to undo) or
//! call the household to practice. No sign-in: household trust.
//!
//! Pure and platform-free like the rest of `screen/`: the board feeds it
//! touches and times, does the [`Action`]s it returns, and draws what
//! [`Ui::view`] says. The same [`grid`] lays buttons out for drawing and
//! for hit-testing, so what you see is what you tap.
use super::canvas::{H, W};
use super::Card;

/// Hold this long for the menu.
pub const LONG_PRESS_MS: u64 = 700;
/// Cards change this often.
pub const ROTATE_MS: u64 = 10_000;
/// The menu closes itself after this long untouched.
pub const IDLE_MS: u64 = 30_000;
/// Time to undo a recorded practice.
pub const UNDO_MS: u64 = 5_000;
/// "Called everyone" stays up this long.
pub const NOTICE_MS: u64 = 3_000;
/// The header strip above the buttons.
pub const HEADER: i32 = 30;

/// The practices the board offers (a short list; the phone has them all).
pub const KINDS: [(&str, &str); 5] = [
    ("meditation", "Meditation"),
    ("chanting", "Chanting"),
    ("prostrations", "Prostrations"),
    ("reading", "Reading"),
    ("precepts", "Kept the precepts"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// `n` buttons in a grid of `cols` columns and two rows below the header:
/// each at least 48 px tall.
pub fn grid(n: usize, cols: usize) -> Vec<Rect> {
    const GAP: i32 = 6;
    let cols = cols.max(1) as i32;
    let rows = 2;
    let w = (W - GAP * (cols + 1)) / cols;
    let h = (H - HEADER - GAP * (rows + 1)) / rows;
    (0..n as i32)
        .take((cols * rows) as usize)
        .map(|i| Rect {
            x: GAP + (i % cols) * (w + GAP),
            y: HEADER + GAP + (i / cols) * (h + GAP),
            w,
            h,
        })
        .collect()
}

/// What a touch did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gesture {
    Tap(i32, i32),
    LongPress,
}

/// Turns polled touch samples into taps and long presses.
#[derive(Default)]
pub struct Touch {
    down: Option<(i32, i32, u64)>,
    held: bool,
}

impl Touch {
    /// One sample: where a finger is (None = lifted), at `now` ms.
    pub fn sample(&mut self, at: Option<(i32, i32)>, now: u64) -> Option<Gesture> {
        match (at, self.down) {
            (Some((x, y)), None) => {
                self.down = Some((x, y, now));
                self.held = false;
                None
            }
            (Some(_), Some((_, _, t))) if !self.held && now.saturating_sub(t) >= LONG_PRESS_MS => {
                self.held = true;
                Some(Gesture::LongPress)
            }
            (None, Some((x, y, _))) => {
                self.down = None;
                (!self.held).then_some(Gesture::Tap(x, y))
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Person {
    pub id: u32,
    pub name: String,
}

/// What the board should do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Record { person: u32, kind: &'static str },
    Undo { entry: u32 },
    Call,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Mode {
    Cards,
    Menu,
    People {
        page: usize,
    },
    Kinds {
        person: u32,
    },
    ConfirmCall,
    Done {
        entry: u32,
        text: String,
        until: u64,
    },
    Notice {
        text: String,
        until: u64,
    },
}

pub struct Ui {
    mode: Mode,
    page: usize,
    rotated: u64,
    touched: u64,
}

impl Default for Ui {
    fn default() -> Self {
        Ui {
            mode: Mode::Cards,
            page: 0,
            rotated: 0,
            touched: 0,
        }
    }
}

/// People per page of the picker: five, and a sixth button for Back/More.
const PER_PAGE: usize = 5;

impl Ui {
    pub fn in_menu(&self) -> bool {
        self.mode != Mode::Cards
    }

    /// Time passes: rotate cards, close idle menus. True when the screen
    /// should be redrawn.
    pub fn tick(&mut self, now: u64, cards: usize) -> bool {
        match &self.mode {
            Mode::Cards => {
                if now.saturating_sub(self.rotated) >= ROTATE_MS {
                    self.rotated = now;
                    self.page = (self.page + 1) % cards.max(1);
                    return true;
                }
                false
            }
            Mode::Done { until, .. } | Mode::Notice { until, .. } => {
                if now >= *until {
                    self.close(now);
                    return true;
                }
                false
            }
            _ => {
                if now.saturating_sub(self.touched) >= IDLE_MS {
                    self.close(now);
                    return true;
                }
                false
            }
        }
    }

    fn close(&mut self, now: u64) {
        self.mode = Mode::Cards;
        self.rotated = now;
    }

    pub fn gesture(
        &mut self,
        g: Gesture,
        people: &[Person],
        cards: usize,
        now: u64,
    ) -> Option<Action> {
        self.touched = now;
        match g {
            Gesture::LongPress => {
                self.mode = if self.mode == Mode::Cards {
                    Mode::Menu
                } else {
                    Mode::Cards
                };
                self.rotated = now;
                None
            }
            Gesture::Tap(x, y) => self.tap(x, y, people, cards, now),
        }
    }

    fn hit(&self, x: i32, y: i32, buttons: usize, cols: usize) -> Option<usize> {
        grid(buttons, cols).iter().position(|r| r.contains(x, y))
    }

    fn tap(&mut self, x: i32, y: i32, people: &[Person], cards: usize, now: u64) -> Option<Action> {
        match self.mode.clone() {
            Mode::Cards => {
                self.page = (self.page + 1) % cards.max(1);
                self.rotated = now;
                None
            }
            Mode::Menu => {
                match self.hit(x, y, 3, 2) {
                    Some(0) => self.mode = Mode::People { page: 0 },
                    Some(1) => self.mode = Mode::ConfirmCall,
                    Some(2) => self.close(now),
                    _ => {}
                }
                None
            }
            Mode::People { page } => {
                let shown = people.iter().skip(page * PER_PAGE).take(PER_PAGE).count();
                match self.hit(x, y, shown + 1, 3) {
                    Some(i) if i < shown => {
                        self.mode = Mode::Kinds {
                            person: people[page * PER_PAGE + i].id,
                        };
                    }
                    Some(_) => {
                        let more = people.len() > (page + 1) * PER_PAGE;
                        self.mode = if more {
                            Mode::People { page: page + 1 }
                        } else {
                            Mode::Menu
                        };
                    }
                    None => {}
                }
                None
            }
            Mode::Kinds { person } => match self.hit(x, y, KINDS.len() + 1, 3) {
                Some(i) if i < KINDS.len() => Some(Action::Record {
                    person,
                    kind: KINDS[i].0,
                }),
                Some(_) => {
                    self.mode = Mode::People { page: 0 };
                    None
                }
                None => None,
            },
            Mode::ConfirmCall => match self.hit(x, y, 2, 2) {
                Some(0) => Some(Action::Call),
                Some(_) => {
                    self.mode = Mode::Menu;
                    None
                }
                None => None,
            },
            Mode::Done { entry, .. } => {
                // The big button is Undo.
                if self.hit(x, y, 1, 1).is_some() {
                    return Some(Action::Undo { entry });
                }
                self.close(now);
                None
            }
            Mode::Notice { .. } => {
                self.close(now);
                None
            }
        }
    }

    /// The board recorded it: offer the undo.
    pub fn recorded(&mut self, entry: u32, text: String, now: u64) {
        self.mode = Mode::Done {
            entry,
            text,
            until: now + UNDO_MS,
        };
    }

    /// A short message, then back to the cards.
    pub fn notice(&mut self, text: String, now: u64) {
        self.mode = Mode::Notice {
            text,
            until: now + NOTICE_MS,
        };
    }

    /// What to draw: a card of the rotation, or a menu screen; and the
    /// rotation position for the dots.
    pub fn view(&self, cards: &[Card], people: &[Person]) -> (Card, usize, usize) {
        let buttons = |title: &str, labels: Vec<String>, cols: usize| Card::Buttons {
            title: title.into(),
            labels,
            cols,
        };
        let card = match &self.mode {
            Mode::Cards => {
                let page = self.page % cards.len().max(1);
                return (
                    cards.get(page).cloned().unwrap_or(Card::Message {
                        title: "Merit Ledger".into(),
                        body: "Starting up".into(),
                        button: String::new(),
                    }),
                    page,
                    cards.len(),
                );
            }
            Mode::Menu => buttons(
                "Hold anywhere to close",
                vec![
                    "Record practice".into(),
                    "Call to practice".into(),
                    "Back".into(),
                ],
                2,
            ),
            Mode::People { page } => {
                let mut labels: Vec<String> = people
                    .iter()
                    .skip(page * PER_PAGE)
                    .take(PER_PAGE)
                    .map(|p| p.name.clone())
                    .collect();
                labels.push(if people.len() > (page + 1) * PER_PAGE {
                    "More".into()
                } else {
                    "Back".into()
                });
                buttons("Record for whom?", labels, 3)
            }
            Mode::Kinds { person } => {
                let name = people
                    .iter()
                    .find(|p| p.id == *person)
                    .map_or("", |p| p.name.as_str());
                let mut labels: Vec<String> = KINDS.iter().map(|(_, n)| (*n).into()).collect();
                labels.push("Back".into());
                buttons(&format!("What did {name} do?"), labels, 3)
            }
            Mode::ConfirmCall => buttons(
                "Call everyone to practice?",
                vec!["Yes, call".into(), "Back".into()],
                2,
            ),
            Mode::Done { text, .. } => Card::Message {
                title: "Recorded".into(),
                body: text.clone(),
                button: "Undo".into(),
            },
            Mode::Notice { text, .. } => Card::Message {
                title: text.clone(),
                body: String::new(),
                button: String::new(),
            },
        };
        (card, 0, 0)
    }
}

/// A raw touch (the controller's portrait coordinates, 172 × 320) on the
/// landscape screen, with the admin's corrections.
pub fn landscape(raw_x: i32, raw_y: i32, flip_x: bool, flip_y: bool) -> (i32, i32) {
    let (mut x, mut y) = (raw_y, (H - 1) - raw_x);
    if flip_x {
        x = W - 1 - x;
    }
    if flip_y {
        y = H - 1 - y;
    }
    (x.clamp(0, W - 1), y.clamp(0, H - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn people() -> Vec<Person> {
        ["Dad", "Mum", "Kid"]
            .iter()
            .enumerate()
            .map(|(i, n)| Person {
                id: i as u32 + 1,
                name: (*n).into(),
            })
            .collect()
    }

    fn centre(r: Rect) -> (i32, i32) {
        (r.x + r.w / 2, r.y + r.h / 2)
    }

    fn tap(ui: &mut Ui, button: usize, of: usize, cols: usize, now: u64) -> Option<Action> {
        let (x, y) = centre(grid(of, cols)[button]);
        ui.gesture(Gesture::Tap(x, y), &people(), 2, now)
    }

    #[test]
    fn buttons_are_big_enough_and_on_screen() {
        for (n, cols) in [(3, 2), (6, 3), (2, 2), (1, 1)] {
            for r in grid(n, cols) {
                assert!(r.h >= 48, "{n}/{cols}: {r:?}");
                assert!(r.x >= 0 && r.x + r.w <= W && r.y + r.h <= H, "{r:?}");
            }
        }
    }

    #[test]
    fn taps_and_long_presses() {
        let mut t = Touch::default();
        assert_eq!(t.sample(Some((10, 20)), 0), None);
        assert_eq!(t.sample(Some((11, 20)), 100), None);
        assert_eq!(t.sample(None, 150), Some(Gesture::Tap(10, 20)));
        assert_eq!(t.sample(Some((5, 5)), 1_000), None);
        assert_eq!(t.sample(Some((5, 5)), 1_700), Some(Gesture::LongPress));
        assert_eq!(t.sample(Some((5, 5)), 2_000), None, "once");
        assert_eq!(t.sample(None, 2_100), None, "no tap after a long press");
    }

    #[test]
    fn record_for_someone_then_undo() {
        let mut ui = Ui::default();
        assert_eq!(ui.gesture(Gesture::LongPress, &people(), 2, 0), None);
        assert!(ui.in_menu());
        assert_eq!(tap(&mut ui, 0, 3, 2, 100), None); // Record practice
        assert_eq!(tap(&mut ui, 1, 4, 3, 200), None); // Mum
        let action = tap(&mut ui, 0, 6, 3, 300); // Meditation
        assert_eq!(
            action,
            Some(Action::Record {
                person: 2,
                kind: "meditation"
            })
        );
        ui.recorded(41, "Meditation for Mum".into(), 400);
        let (card, ..) = ui.view(&[], &people());
        assert!(matches!(card, Card::Message { ref button, .. } if button == "Undo"));
        assert_eq!(tap(&mut ui, 0, 1, 1, 500), Some(Action::Undo { entry: 41 }));
        // Without an undo, it closes by itself.
        ui.recorded(42, "x".into(), 1_000);
        assert!(!ui.tick(1_000 + UNDO_MS - 1, 2));
        assert!(ui.tick(1_000 + UNDO_MS, 2));
        assert!(!ui.in_menu());
    }

    #[test]
    fn call_to_practice_needs_a_confirm() {
        let mut ui = Ui::default();
        ui.gesture(Gesture::LongPress, &people(), 2, 0);
        assert_eq!(tap(&mut ui, 1, 3, 2, 10), None);
        assert_eq!(tap(&mut ui, 1, 2, 2, 20), None, "Back");
        assert_eq!(tap(&mut ui, 1, 3, 2, 30), None);
        assert_eq!(tap(&mut ui, 0, 2, 2, 40), Some(Action::Call));
    }

    #[test]
    fn cards_rotate_and_menus_time_out() {
        let mut ui = Ui::default();
        assert!(ui.tick(ROTATE_MS, 2));
        assert_eq!(ui.view(&[card(), card()], &[]).1, 1);
        assert!(!ui.tick(ROTATE_MS + 10, 2));
        ui.gesture(Gesture::LongPress, &people(), 2, 20_000);
        assert!(!ui.tick(20_000 + IDLE_MS - 1, 2));
        assert!(ui.tick(20_000 + IDLE_MS, 2));
        assert!(!ui.in_menu());
    }

    #[test]
    fn many_people_page() {
        let many: Vec<Person> = (1..=7)
            .map(|i| Person {
                id: i,
                name: format!("P{i}"),
            })
            .collect();
        let mut ui = Ui::default();
        ui.gesture(Gesture::LongPress, &many, 2, 0);
        let (x, y) = centre(grid(3, 2)[0]);
        ui.gesture(Gesture::Tap(x, y), &many, 2, 1);
        let (Card::Buttons { labels, .. }, ..) = ui.view(&[], &many) else {
            panic!()
        };
        assert_eq!(labels.last().unwrap(), "More");
        let (x, y) = centre(grid(6, 3)[5]);
        ui.gesture(Gesture::Tap(x, y), &many, 2, 2);
        let (Card::Buttons { labels, .. }, ..) = ui.view(&[], &many) else {
            panic!()
        };
        assert_eq!(labels, ["P6", "P7", "Back"]);
    }

    #[test]
    fn raw_touches_map_onto_the_landscape_screen() {
        assert_eq!(landscape(0, 0, false, false), (0, H - 1));
        assert_eq!(landscape(H - 1, W - 1, false, false), (W - 1, 0));
        assert_eq!(landscape(0, 0, true, true), (W - 1, 0));
        assert_eq!(landscape(500, 900, false, false), (W - 1, 0), "clamped");
    }

    fn card() -> Card {
        Card::Message {
            title: "t".into(),
            body: String::new(),
            button: String::new(),
        }
    }
}
