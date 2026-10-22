//! The board's own screen (Sprint 10, ahead of Sprint 13's touch menu): a
//! few cards in rotation, drawn strip by strip into the 320 × 172 panel.
//!
//! - **Today:** the time, the date, today's observances (or the next one),
//!   and how many people have practised today.
//! - **A call to service,** while it's fresh: who calls, and what for.
//! - **The next event** in the coming week, with who's coming.
//!
//! - **A verse** from the household's texts, so the rotation never stands
//!   still.
//!
//! It's an LCD, not an OLED, but a picture that never changes can still
//! leave a ghost: there are always at least two cards (today and a verse)
//! in turn, and the board dims the screen at night ([`State::screen_night`]).
//!
//! Pure Rust and platform-free (after gamematcher's `screen/`), so the
//! desktop renders exactly what the board shows: `/api/v1/display/frame.bmp`.
pub mod canvas;
pub mod text;
pub mod ui;

use crate::notify::{local_instant, CALL_SHOWN};
use crate::state::State;
use canvas::{Canvas, Rgb, H, W};
use std::io::{self, Read};
use text::{squeeze, squeeze_line, Align, Face, BODY, BOLD, HUGE, MID, SMALL, TITLE};

/// Rows per strip: 320 × 8 × 2 bytes = 5 KB, the panel driver's DMA buffer
/// (internal RAM, which is scarce: Sprint 10 measured it).
pub const STRIP: i32 = 8;
/// A call to service leads the rotation for this long.
pub const CALL_FRESH: u64 = 30 * 60;

const BG: Rgb = Rgb(18, 16, 12);
const TEXT: Rgb = Rgb(239, 232, 219);
const MUTED: Rgb = Rgb(168, 158, 142);
const ACCENT: Rgb = Rgb(224, 164, 88);
const CALL_BG: Rgb = Rgb(120, 62, 18);
const GOOD: Rgb = Rgb(156, 196, 131);
const PANEL: Rgb = Rgb(52, 46, 36);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Card {
    Today {
        /// "07:42"
        time: String,
        /// "Wednesday" / "14 October"
        weekday: String,
        date: String,
        /// "Lunar 9/9" ("" when there's no lunar calendar).
        lunar: String,
        /// "Uposatha · a feast day", or "Next: Uposatha, Saturday".
        observance: String,
        today: bool,
        /// People who have practised today, and their minutes.
        people: u32,
        minutes: u32,
    },
    Call {
        by: String,
        message: String,
        when: String,
        minutes_ago: u64,
    },
    Event {
        title: String,
        when: String,
        place: String,
        coming: u32,
        maybe: u32,
    },
    Verse {
        text: String,
        source: String,
    },
    /// A menu screen: a title and a grid of buttons (`ui::grid`).
    Buttons {
        title: String,
        labels: Vec<String>,
        cols: usize,
    },
    /// A message, with one big button (`ui::grid(1, 1)`) when `button` isn't "".
    Message {
        title: String,
        body: String,
        button: String,
    },
}

/// Short lines from the household's built-in texts (`public/texts/`).
const VERSES: [(&str, &str); 10] = [
    ("Form is emptiness; emptiness is form.", "The Heart Sutra"),
    (
        "Gate gate paragate parasamgate bodhi svaha.",
        "The Heart Sutra",
    ),
    ("Nam-myoho-renge-kyo", "The daimoku"),
    (
        "How can I help every being enter the highest wisdom?",
        "Lotus Sutra, chapter 16",
    ),
    (
        "May whatever good this practice has done not stop with me.",
        "Dedication of merit",
    ),
    ("May we all wake up, together.", "Dedication of merit"),
    (
        "May all beings be safe, well and at ease.",
        "Loving-kindness",
    ),
    (
        "I take refuge in the Dharma: the way things are, and the teaching that shows it.",
        "The Three Refuges",
    ),
    (
        "Tomorrow is another chance to practise.",
        "Evening reflection",
    ),
    (
        "All the harm I have done, I now acknowledge, and repent.",
        "Verse of repentance",
    ),
];

/// Diacritics the screen's font may lack, as their plain letters.
fn plain(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'ā' | 'á' | 'à' | 'â' | 'ä' => 'a',
            'ī' | 'í' | 'ì' | 'î' | 'ï' => 'i',
            'ū' | 'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ē' | 'é' | 'è' | 'ê' | 'ë' => 'e',
            'ō' | 'ó' | 'ò' | 'ô' | 'ö' => 'o',
            'Ā' => 'A',
            'Ī' => 'I',
            'Ū' => 'U',
            'ṃ' => 'm',
            'ṇ' | 'ṅ' | 'ñ' => 'n',
            'ḷ' => 'l',
            'ṭ' => 't',
            'ḍ' => 'd',
            'ś' => 's',
            c => c,
        })
        .collect()
}

const WEEKDAYS: [&str; 7] = [
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

impl State {
    /// Night (22:00 to 06:00 household time): the board dims the screen.
    pub fn screen_night(&self, now: u64) -> bool {
        let t = now as i64;
        let hour = (t + self.tz.offset_at(t)).rem_euclid(86_400) / 3600;
        !(6..22).contains(&hour)
    }

    /// What the screen shows at `now`: a fresh call to service first, then
    /// today, then the next event this week.
    pub fn screen_cards(&self, now: u64) -> Vec<Card> {
        let t = now as i64;
        let local = t + self.tz.offset_at(t);
        let minutes = local.rem_euclid(86_400) / 60;
        let today = self.tz.local_date(t);
        let mut cards = Vec::new();
        if let Some(c) = self
            .calls
            .first()
            .filter(|c| now.saturating_sub(c.created) < CALL_FRESH.min(CALL_SHOWN))
        {
            cards.push(Card::Call {
                by: if c.by == 0 {
                    "Someone at the board".into()
                } else {
                    self.name_of(c.by)
                },
                message: c.message.clone(),
                when: c.when.clone(),
                minutes_ago: now.saturating_sub(c.created) / 60,
            });
        }
        let system = self.household_calendar();
        let ahead = crate::calendar::observances(
            system,
            self.calendar_options(),
            &self.tz,
            today,
            today.add_days(14),
        );
        let next = ahead.iter().find(|o| o.kind != "moon");
        let observance = match next {
            Some(o) if o.date == today => {
                let names: Vec<String> = ahead
                    .iter()
                    .filter(|x| x.date == today && x.kind != "moon")
                    .map(|x| x.name.clone())
                    .collect();
                let feast = ahead.iter().any(|x| x.date == today && x.feast);
                format!(
                    "{}{}",
                    names.join(" · "),
                    if feast { " · a feast day" } else { "" }
                )
            }
            Some(o) => {
                let days = o.date.days() - today.days();
                let day = if days == 1 {
                    "tomorrow".to_string()
                } else {
                    WEEKDAYS[o.date.weekday() as usize].to_string()
                };
                format!("Next: {}, {day}", o.name)
            }
            None => String::new(),
        };
        let date = today.iso();
        let mut people: Vec<u32> = Vec::new();
        let mut practised = 0;
        for e in self.entries.iter().filter(|e| e.date == date) {
            let Some(kind) = self.kind(&e.kind) else {
                continue;
            };
            if kind.polarity != "merit" {
                continue;
            }
            if !people.contains(&e.person) {
                people.push(e.person);
            }
            if kind.unit == "minutes" {
                practised += e.amount;
            }
        }
        let lunar = crate::calendar::chinese_months(&self.tz, today, today)
            .iter()
            .find(|m| m.start <= today)
            .map(|m| {
                let day = today.days() - m.start.days() + 1;
                format!(
                    "Lunar {}{}/{day}",
                    if m.leap { "leap " } else { "" },
                    m.number
                )
            })
            .unwrap_or_default();
        cards.push(Card::Today {
            time: format!("{:02}:{:02}", minutes / 60, minutes % 60),
            weekday: WEEKDAYS[today.weekday() as usize].into(),
            date: format!("{} {}", today.day, MONTHS[(today.month - 1) as usize]),
            lunar,
            today: next.is_some_and(|o| o.date == today),
            observance,
            people: people.len() as u32,
            minutes: practised,
        });
        // A verse: a different one each hour.
        let (text, source) = VERSES[((now / 3600) % VERSES.len() as u64) as usize];
        cards.push(Card::Verse {
            text: text.into(),
            source: source.into(),
        });
        let week = local_instant(&self.tz, today.add_days(7), 0);
        if let Some(e) = self
            .events
            .iter()
            .filter(|e| !e.cancelled && e.start + 3600 > now && e.start < week)
            .min_by_key(|e| e.start)
        {
            let count = |a: &str| {
                e.rsvps.iter().filter(|r| r.answer == a).count() as u32
                    + e.guests.iter().filter(|g| g.answer == a).count() as u32
            };
            cards.push(Card::Event {
                title: e.title.clone(),
                when: crate::notify::when(&self.tz, e.start),
                place: e.place.clone(),
                coming: count("yes") + 1,
                maybe: count("maybe"),
            });
        }
        cards
    }
}

/// Everything a card needs besides its data.
pub struct Context {
    /// "meritledger.local"
    pub name: String,
    pub page: usize,
    pub pages: usize,
}

/// One card, ready to draw.
pub struct Frame {
    card: Card,
    ctx: Context,
}

impl Frame {
    pub fn new(card: Card, ctx: Context) -> Frame {
        Frame { card, ctx }
    }

    /// Draws rows `top..top + rows` into `px` (`rows` × 320 pixels).
    pub fn draw_strip(&self, px: &mut [u16], top: i32, rows: i32) {
        let mut c = Canvas::new(px, top, rows);
        draw(&mut c, &self.card, &self.ctx);
    }

    /// Calls `send(top, rows, pixels)` for each strip, top to bottom.
    pub fn draw<E>(
        &self,
        buf: &mut [u16],
        mut send: impl FnMut(i32, i32, &[u16]) -> Result<(), E>,
    ) -> Result<(), E> {
        let mut top = 0;
        while top < H {
            let rows = STRIP.min(H - top);
            let px = &mut buf[..(rows * W) as usize];
            self.draw_strip(px, top, rows);
            send(top, rows, px)?;
            top += rows;
        }
        Ok(())
    }
}

fn dots(c: &mut Canvas, ctx: &Context) {
    if ctx.pages < 2 {
        return;
    }
    let gap = 10.0;
    let x0 = W as f32 - 14.0 - gap * (ctx.pages - 1) as f32;
    for i in 0..ctx.pages {
        let color = if i == ctx.page {
            MUTED
        } else {
            Rgb(70, 64, 54)
        };
        c.circle(x0 + gap * i as f32, H as f32 - 10.0, 2.5, color);
    }
}

fn draw(c: &mut Canvas, card: &Card, ctx: &Context) {
    match card {
        Card::Today {
            time,
            weekday,
            date,
            lunar,
            observance,
            today,
            people,
            minutes,
        } => {
            c.fill(BG);
            c.text(&HUGE, 14, 58, Align::Left, TEXT, time);
            c.text(&BOLD, W - 14, 26, Align::Right, TEXT, weekday);
            c.text(&BODY, W - 14, 44, Align::Right, MUTED, date);
            c.text(&SMALL, W - 14, 60, Align::Right, ACCENT, lunar);
            if !observance.is_empty() {
                let color = if *today { ACCENT } else { MUTED };
                let fitted = squeeze(&plain(observance), W - 28, 46, &[&MID, &BOLD, &BODY]);
                c.block(&fitted, 14, 76, Align::Left, color);
            }
            let practice = match (people, minutes) {
                (0, _) => "No one has practised yet today".to_string(),
                (1, 0) => "1 person has practised today".to_string(),
                (n, 0) => format!("{n} people have practised today"),
                (1, m) => format!("1 person has practised today · {m} min"),
                (n, m) => format!("{n} people have practised today · {m} min"),
            };
            c.text(
                &SMALL,
                14,
                H - 26,
                Align::Left,
                if *people > 0 { GOOD } else { MUTED },
                &practice,
            );
            c.text(&SMALL, 14, H - 8, Align::Left, Rgb(110, 102, 90), &ctx.name);
        }
        Card::Call {
            by,
            message,
            when,
            minutes_ago,
        } => {
            c.vgradient(0, 0, W, H, CALL_BG, BG);
            // A bell: a dome, a rim, a clapper.
            c.circle(36.0, 40.0, 16.0, ACCENT);
            c.rect(20, 40, 33, 12, ACCENT);
            c.rect(16, 50, 41, 4, ACCENT);
            c.circle(36.0, 58.0, 4.0, ACCENT);
            let head = format!("{} calls everyone", plain(by));
            let fitted = squeeze_line(&head, W - 84, 2, &[&TITLE, &MID, &BOLD]);
            c.block(&fitted, 70, 18, Align::Left, TEXT);
            let body = if message.is_empty() {
                "to practice".to_string()
            } else {
                plain(message)
            };
            let fitted = squeeze(&body, W - 28, 60, &[&MID, &BOLD, &BODY, &SMALL]);
            c.block(&fitted, 14, 74, Align::Left, TEXT);
            let ago = match minutes_ago {
                0 => "just now".to_string(),
                m => format!("{m} min ago"),
            };
            let tail = if when.is_empty() {
                ago
            } else {
                format!("{} · {ago}", plain(when))
            };
            c.text(&BOLD, 14, H - 12, Align::Left, ACCENT, &tail);
        }
        Card::Buttons {
            title,
            labels,
            cols,
        } => {
            c.fill(BG);
            let head = squeeze_line(&plain(title), W - 20, 1, &[&BOLD, &BODY, &SMALL]);
            c.block(&head, 10, 6, Align::Left, MUTED);
            let cells = ui::grid(labels.len(), *cols);
            // One size for the whole menu (the smallest any label needs),
            // so the buttons read as a set.
            const FACES: [&Face; 4] = [&MID, &BOLD, &BODY, &SMALL];
            let face = cells
                .iter()
                .zip(labels)
                .map(|(r, label)| {
                    let f = squeeze(&plain(label), r.w - 12, r.h - 8, &FACES).face;
                    FACES.iter().position(|g| std::ptr::eq(*g, f)).unwrap_or(3)
                })
                .max()
                .unwrap_or(0);
            for (r, label) in cells.iter().zip(labels) {
                let back = matches!(label.as_str(), "Back" | "More");
                c.round_rect(
                    r.x,
                    r.y,
                    r.w,
                    r.h,
                    10,
                    if back { Rgb(44, 40, 34) } else { PANEL },
                );
                let fitted = squeeze(&plain(label), r.w - 12, r.h - 8, &[FACES[face]]);
                let height = fitted.face.line as i32 * fitted.lines.len() as i32;
                c.block(
                    &fitted,
                    r.x + r.w / 2,
                    r.y + (r.h - height) / 2,
                    Align::Center,
                    if back { MUTED } else { TEXT },
                );
            }
        }
        Card::Message {
            title,
            body,
            button,
        } => {
            c.fill(BG);
            if body.is_empty() && button.is_empty() {
                // A notice: just the words, in the middle.
                let fitted = squeeze(&plain(title), W - 40, H - 40, &[&TITLE, &MID, &BOLD]);
                let height = fitted.face.line as i32 * fitted.lines.len() as i32;
                c.block(&fitted, W / 2, (H - height) / 2, Align::Center, GOOD);
                dots(c, ctx);
                return;
            }
            let head = squeeze_line(&plain(title), W - 20, 1, &[&TITLE, &MID, &BOLD]);
            c.block(&head, 10, 2, Align::Left, GOOD);
            if !button.is_empty() {
                let r = ui::grid(1, 1)[0];
                c.round_rect(r.x, r.y, r.w, r.h, 10, ACCENT);
                c.text(
                    &TITLE,
                    r.x + r.w / 2,
                    r.y + r.h / 2 + 8,
                    Align::Center,
                    BG,
                    button,
                );
            }
            let fitted = squeeze(&plain(body), W - 28, 56, &[&MID, &BOLD, &BODY]);
            c.block(
                &fitted,
                14,
                if button.is_empty() { 50 } else { 106 },
                Align::Left,
                TEXT,
            );
        }
        Card::Verse { text, source } => {
            c.fill(BG);
            let fitted = squeeze(&plain(text), W - 40, H - 60, &[&TITLE, &MID, &BOLD, &BODY]);
            let height = fitted.face.line as i32 * fitted.lines.len() as i32;
            c.block(&fitted, W / 2, (H - 30 - height) / 2, Align::Center, TEXT);
            c.text(&SMALL, W / 2, H - 22, Align::Center, ACCENT, &plain(source));
        }
        Card::Event {
            title,
            when,
            place,
            coming,
            maybe,
        } => {
            c.fill(BG);
            c.text(&SMALL, 14, 20, Align::Left, MUTED, "COMING UP");
            let fitted = squeeze(&plain(title), W - 28, 52, &[&TITLE, &MID, &BOLD]);
            let used = c.block(&fitted, 14, 26, Align::Left, TEXT);
            let line = squeeze_line(&plain(when), W - 28, 1, &[&BOLD, &BODY, &SMALL]);
            c.block(&line, 14, 30 + used, Align::Left, ACCENT);
            let mut who = format!("{coming} coming");
            if *maybe > 0 {
                who.push_str(&format!(" · {maybe} maybe"));
            }
            if !place.is_empty() {
                who.push_str(&format!(" · {}", plain(place)));
            }
            c.text(&BODY, 14, H - 12, Align::Left, GOOD, &who);
        }
    }
    dots(c, ctx);
}

/// The frame as a 24-bit top-down BMP, rendered a strip at a time as it is
/// read (never the whole 165 KB image at once).
pub struct BmpStream {
    frame: Frame,
    header: Vec<u8>,
    sent_header: usize,
    next_top: i32,
    rows: Vec<u8>,
    pos: usize,
    strip: Vec<u16>,
}

pub const BMP_LEN: u64 = 54 + (W as u64 * 3) * H as u64;

impl BmpStream {
    pub fn new(frame: Frame) -> BmpStream {
        let mut h = Vec::with_capacity(54);
        h.extend_from_slice(b"BM");
        h.extend_from_slice(&(BMP_LEN as u32).to_le_bytes());
        h.extend_from_slice(&[0, 0, 0, 0]);
        h.extend_from_slice(&54u32.to_le_bytes());
        h.extend_from_slice(&40u32.to_le_bytes());
        h.extend_from_slice(&(W).to_le_bytes());
        h.extend_from_slice(&(-H).to_le_bytes()); // negative: top-down rows
        h.extend_from_slice(&1u16.to_le_bytes());
        h.extend_from_slice(&24u16.to_le_bytes());
        h.extend_from_slice(&[0; 24]);
        BmpStream {
            frame,
            header: h,
            sent_header: 0,
            next_top: 0,
            rows: Vec::new(),
            pos: 0,
            strip: vec![0; (W * STRIP) as usize],
        }
    }
}

impl Read for BmpStream {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.sent_header < self.header.len() {
            let n = out.len().min(self.header.len() - self.sent_header);
            out[..n].copy_from_slice(&self.header[self.sent_header..self.sent_header + n]);
            self.sent_header += n;
            return Ok(n);
        }
        if self.pos == self.rows.len() {
            if self.next_top >= H {
                return Ok(0);
            }
            let rows = STRIP.min(H - self.next_top);
            let px = &mut self.strip[..(rows * W) as usize];
            self.frame.draw_strip(px, self.next_top, rows);
            self.rows.clear();
            for &p in px.iter() {
                let c = Rgb::from565(p);
                self.rows.extend_from_slice(&[c.2, c.1, c.0]); // BMP is BGR
            }
            self.pos = 0;
            self.next_top += rows;
        }
        let n = out.len().min(self.rows.len() - self.pos);
        out[..n].copy_from_slice(&self.rows[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Context {
        Context {
            name: "meritledger.local".into(),
            page: 0,
            pages: 3,
        }
    }

    pub fn samples() -> Vec<Card> {
        vec![
            Card::Today {
                time: "07:42".into(),
                weekday: "Wednesday".into(),
                date: "14 October".into(),
                lunar: "Lunar 9/3".into(),
                observance: "Uposatha (full moon) · a feast day".into(),
                today: true,
                people: 3,
                minutes: 85,
            },
            Card::Today {
                time: "21:05".into(),
                weekday: "Saturday".into(),
                date: "24 October".into(),
                lunar: "Lunar 9/13".into(),
                observance: "Next: Māgha Pūjā, tomorrow".into(),
                today: false,
                people: 0,
                minutes: 0,
            },
            Card::Call {
                by: "Dad".into(),
                message: "Dinner's ready, then we chant together in the shrine room".into(),
                when: "at 7".into(),
                minutes_ago: 4,
            },
            Card::Event {
                title: "Ullambana vegetarian potluck".into(),
                when: "Saturday 24 October, 18:00".into(),
                place: "Home".into(),
                coming: 4,
                maybe: 1,
            },
            Card::Verse {
                text: VERSES[3].0.into(),
                source: VERSES[3].1.into(),
            },
            Card::Buttons {
                title: "Hold anywhere to close".into(),
                labels: vec![
                    "Record practice".into(),
                    "Call to practice".into(),
                    "Back".into(),
                ],
                cols: 2,
            },
            Card::Buttons {
                title: "What did Dad do?".into(),
                labels: ui::KINDS
                    .iter()
                    .map(|(_, n)| (*n).into())
                    .chain(["Back".to_string()])
                    .collect(),
                cols: 3,
            },
            Card::Buttons {
                title: "Record for whom?".into(),
                labels: ["Dad", "Mom", "Grandma Lin", "Sam", "Wei", "More"]
                    .map(String::from)
                    .to_vec(),
                cols: 3,
            },
            Card::Buttons {
                title: "Call everyone to practice?".into(),
                labels: vec!["Yes, call".into(), "Back".into()],
                cols: 2,
            },
            Card::Message {
                title: "Recorded".into(),
                body: "Meditation for Dad".into(),
                button: "Undo".into(),
            },
            Card::Message {
                title: "Called 4 to practice".into(),
                body: String::new(),
                button: String::new(),
            },
        ]
    }

    #[test]
    fn night_is_ten_to_six() {
        let s = State::new(crate::state::Limits::board());
        let tz = &s.tz;
        let day = crate::tz::Date::parse("2026-10-19").unwrap();
        let at = |h: u32| local_instant(tz, day, h * 60);
        assert!(s.screen_night(at(23)) && s.screen_night(at(5)));
        assert!(!s.screen_night(at(6)) && !s.screen_night(at(21)));
    }

    #[test]
    fn every_card_renders_every_strip() {
        let mut buf = vec![0u16; (W * STRIP) as usize];
        for card in samples() {
            let frame = Frame::new(card, ctx());
            let mut strips = 0;
            frame
                .draw(&mut buf, |_, _, px| {
                    strips += 1;
                    assert!(px.iter().any(|&p| p != 0));
                    Ok::<(), ()>(())
                })
                .unwrap();
            assert_eq!(strips, 22);
        }
    }

    #[test]
    fn bmp_stream_is_complete() {
        let mut bmp = Vec::new();
        BmpStream::new(Frame::new(samples().remove(2), ctx()))
            .read_to_end(&mut bmp)
            .unwrap();
        assert_eq!(bmp.len() as u64, BMP_LEN);
        assert_eq!(&bmp[..2], b"BM");
    }

    #[test]
    fn plain_letters() {
        assert_eq!(plain("Māgha Pūjā, Āsāḷha"), "Magha Puja, Asalha");
    }

    /// `ML_PREVIEW=dir cargo test preview`: writes every sample card as a BMP.
    #[test]
    fn preview() {
        let Some(dir) = std::env::var_os("ML_PREVIEW") else {
            return;
        };
        for (i, card) in samples().into_iter().enumerate() {
            let label = match &card {
                Card::Today { .. } => "today".to_string(),
                Card::Call { .. } => "call".into(),
                Card::Event { .. } => "event".into(),
                Card::Verse { .. } => "verse".into(),
                Card::Buttons { title, .. } => format!("menu {title}"),
                Card::Message { title, .. } => format!("message {title}"),
            };
            let mut bmp = Vec::new();
            let pages = if matches!(card, Card::Buttons { .. } | Card::Message { .. }) {
                0
            } else {
                3
            };
            BmpStream::new(Frame::new(
                card,
                Context {
                    page: i % 3,
                    pages,
                    ..ctx()
                },
            ))
            .read_to_end(&mut bmp)
            .unwrap();
            let name = format!("{i:02} {label}.bmp").replace(['?', '/', ':'], "");
            std::fs::write(std::path::Path::new(&dir).join(name), bmp).unwrap();
        }
    }
}
