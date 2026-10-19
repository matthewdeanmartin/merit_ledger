//! The entry journal on the SD card (`docs/SD_CARD.md`): append-only logs
//! in `ml/journal/NNNNNN.log`, so a tap writes one small record instead of
//! rewriting the snapshot.
//!
//! A record is `A5 4C | len: u16 LE | crc32: u32 LE | payload`, the payload
//! a [`StoredEntry`] as CBOR with integer keys. An edit appends the whole
//! new version; a delete appends a tombstone (`deleted: true`). Reading
//! keeps the **last** record per id, in file order then offset order.
//!
//! Memory is bounded by design: files are at most about [`ROTATE_AT`]
//! (read one at a time, records decoded one at a time), and finding each
//! id's last record needs an index of 12 bytes per record. A household of
//! four logging five things a day writes ~7,300 records a year.
//!
//! Crash safety: a batch is one write and one fsync per file. A torn record
//! can only be at the end of the newest file; [`Journal::open`] cuts it
//! off before anything is appended after it. A bad record in an older file
//! (a card fault) stops reading that file, keeps the rest, and is reported.
use crate::messages::StoredEntry;
use crate::sd::Sd;
use miniframework::wire::{self, Format};
use std::io;
use std::sync::Arc;

const DIR: &str = "ml/journal";
const MAGIC: [u8; 2] = [0xA5, 0x4C];
const HEADER: usize = 8;
/// A file is closed for appends once it reaches this size.
pub const ROTATE_AT: u64 = 1024 * 1024;

pub fn encode(entry: &StoredEntry) -> io::Result<Vec<u8>> {
    let payload =
        wire::to_vec(Format::CborInt, entry).map_err(|e| io::Error::other(e.to_string()))?;
    let len =
        u16::try_from(payload.len()).map_err(|_| io::Error::other("journal record over 64 KiB"))?;
    let mut out = Vec::with_capacity(HEADER + payload.len());
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&crc32fast::hash(&payload).to_le_bytes());
    out.extend_from_slice(&payload);
    Ok(out)
}

/// The CRC-checked records of one file, in order, not yet decoded:
/// (offset, payload).
pub struct Frames<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Frames<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Frames { bytes, at: 0 }
    }

    /// Bytes read as valid so far: once the iterator ends, where the
    /// valid part of the file stops.
    pub fn valid(&self) -> usize {
        self.at
    }
}

impl<'a> Iterator for Frames<'a> {
    type Item = (usize, &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        let b = self.bytes;
        let at = self.at;
        if at + HEADER > b.len() || b[at..at + 2] != MAGIC {
            return None;
        }
        let len = u16::from_le_bytes([b[at + 2], b[at + 3]]) as usize;
        let crc = u32::from_le_bytes(b[at + 4..at + 8].try_into().unwrap());
        let payload = b.get(at + HEADER..at + HEADER + len)?;
        if crc32fast::hash(payload) != crc {
            return None;
        }
        self.at = at + HEADER + len;
        Some((at, payload))
    }
}

/// What reading a journal needs to know about a record without decoding
/// all of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Head {
    pub id: u32,
    pub person: u32,
    /// Days since 1970-01-01 of the entry's date (`i32::MIN` for none).
    pub day: i32,
    pub deleted: bool,
}

fn cbor_head(b: &[u8], i: &mut usize) -> Option<(u8, u64)> {
    let ib = *b.get(*i)?;
    *i += 1;
    let (major, ai) = (ib >> 5, ib & 0x1F);
    let n = match ai {
        0..=23 => return Some((major, u64::from(ai))),
        24 => 1,
        25 => 2,
        26 => 4,
        27 => 8,
        _ => return None,
    };
    let raw = b.get(*i..*i + n)?;
    *i += n;
    Some((major, raw.iter().fold(0u64, |v, x| v << 8 | u64::from(*x))))
}

/// Skips one CBOR item (any type) without allocating.
fn cbor_skip(b: &[u8], i: &mut usize) -> Option<()> {
    let (major, v) = cbor_head(b, i)?;
    match major {
        0 | 1 => {}
        2 | 3 => {
            *i = i.checked_add(v as usize)?;
            if *i > b.len() {
                return None;
            }
        }
        4 => {
            for _ in 0..v {
                cbor_skip(b, i)?;
            }
        }
        5 => {
            for _ in 0..v * 2 {
                cbor_skip(b, i)?;
            }
        }
        // Simple values and floats: cbor_head already consumed them.
        7 => {}
        _ => return None,
    }
    Some(())
}

/// The head of a record (CBOR map with integer keys): reads keys 1 id,
/// 2 person, 4 date and 13 deleted, and skips every other field, so new
/// fields don't break it. `None` if the bytes don't look like that; the
/// caller then decodes in full.
pub fn peek(payload: &[u8]) -> Option<Head> {
    let mut i = 0;
    let (major, fields) = cbor_head(payload, &mut i)?;
    if major != 5 {
        return None;
    }
    let mut head = Head {
        id: 0,
        person: 0,
        day: i32::MIN,
        deleted: false,
    };
    let mut seen_id = false;
    for _ in 0..fields {
        let (major, key) = cbor_head(payload, &mut i)?;
        if major != 0 {
            return None;
        }
        match key {
            1 | 2 => {
                let (major, v) = cbor_head(payload, &mut i)?;
                if major != 0 {
                    return None;
                }
                let v = u32::try_from(v).ok()?;
                if key == 1 {
                    head.id = v;
                    seen_id = true;
                } else {
                    head.person = v;
                }
            }
            4 => {
                let (major, len) = cbor_head(payload, &mut i)?;
                if major != 3 {
                    return None;
                }
                let date = std::str::from_utf8(payload.get(i..i + len as usize)?).ok()?;
                i += len as usize;
                head.day = crate::tz::Date::parse(date).map_or(i32::MIN, |d| d.days() as i32);
            }
            13 => {
                head.deleted = match *payload.get(i)? {
                    0xF5 => true,
                    0xF4 => false,
                    _ => return None,
                };
                i += 1;
            }
            _ => cbor_skip(payload, &mut i)?,
        }
    }
    seen_id.then_some(head)
}

fn decode(payload: &[u8]) -> Option<StoredEntry> {
    wire::decode::<StoredEntry>(Format::CborInt, payload).ok()
}

/// The head, cheaply when the layout is the expected one, else by decoding.
fn head_of(payload: &[u8]) -> Option<Head> {
    peek(payload).or_else(|| {
        let e = decode(payload)?;
        Some(Head {
            id: e.id,
            person: e.person,
            day: crate::tz::Date::parse(&e.date).map_or(i32::MIN, |d| d.days() as i32),
            deleted: e.deleted,
        })
    })
}

/// Decodes the records of one file in order (tests and tools).
pub fn records(bytes: &[u8]) -> impl Iterator<Item = (usize, StoredEntry)> + '_ {
    Frames::new(bytes).map_while(|(at, p)| decode(p).map(|e| (at, e)))
}

fn file_name(n: u32) -> String {
    format!("{n:06}.log")
}

fn is_journal_file(n: &str) -> bool {
    n.len() == 10 && n.ends_with(".log") && n[..6].chars().all(|c| c.is_ascii_digit())
}

/// What opening the journal found (`/api/v1/admin/storage`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JournalInfo {
    pub files: u32,
    pub bytes: u64,
    pub records: u64,
    /// Bytes cut off the end of the newest file (a write torn by power loss).
    pub torn_bytes: u64,
    /// Older files whose reading stopped at a bad record.
    pub damaged_files: Vec<String>,
    /// The highest entry id in any record, tombstones included: deleted
    /// ids are never handed out again.
    pub max_id: u32,
}

/// Where one id's last record is: 20 bytes per record while reading.
#[derive(Clone, Copy)]
struct Slot {
    head: Head,
    file: u32,
    offset: u32,
}

pub struct Journal {
    sd: Arc<Sd>,
    /// The file appends go to, and its length.
    file: u32,
    len: u64,
    pub info: JournalInfo,
}

impl Journal {
    fn names(sd: &Sd) -> io::Result<Vec<String>> {
        Ok(sd
            .list(DIR)?
            .into_iter()
            .filter(|n| is_journal_file(n))
            .collect())
    }

    /// Opens the journal: checks every file, cuts a torn tail off the
    /// newest, and counts what is there. Holds one file at a time.
    pub fn open(sd: Arc<Sd>) -> io::Result<Journal> {
        let names = Self::names(&sd)?;
        let mut info = JournalInfo::default();
        let mut len = 0;
        for (i, name) in names.iter().enumerate() {
            let rel = format!("{DIR}/{name}");
            let bytes = sd.read(&rel)?;
            let mut frames = Frames::new(&bytes);
            let mut valid = 0;
            for (at, payload) in frames.by_ref() {
                // A frame whose payload doesn't decode is damage too.
                let Some(head) = head_of(payload) else { break };
                info.records += 1;
                info.max_id = info.max_id.max(head.id);
                valid = at + HEADER + payload.len();
            }
            let valid = valid.min(frames.valid());
            if valid < bytes.len() {
                if i + 1 == names.len() {
                    sd.truncate(&rel, valid as u64)?;
                    info.torn_bytes = (bytes.len() - valid) as u64;
                    log::warn!("journal: cut {} torn bytes off {name}", info.torn_bytes);
                } else {
                    info.damaged_files.push(name.clone());
                    log::error!(
                        "journal: {name} has a bad record at {valid}; kept what came before"
                    );
                }
            }
            info.bytes += valid as u64;
            len = valid as u64;
        }
        info.files = names.len() as u32;
        let file = names.last().and_then(|n| n[..6].parse().ok()).unwrap_or(1);
        Ok(Journal {
            sd,
            file,
            len,
            info,
        })
    }

    /// Appends `records` with one fsync per file touched, starting a new
    /// file whenever this one reaches [`ROTATE_AT`].
    pub fn append(&mut self, records: &[StoredEntry]) -> io::Result<()> {
        let mut chunk = Vec::new();
        for r in records {
            let bytes = encode(r)?;
            if self.len + (chunk.len() + bytes.len()) as u64 > ROTATE_AT
                && (self.len > 0 || !chunk.is_empty())
            {
                self.write_chunk(&chunk)?;
                chunk.clear();
                self.file += 1;
                self.len = 0;
                self.info.files += 1;
            }
            chunk.extend_from_slice(&bytes);
            self.info.records += 1;
            self.info.max_id = self.info.max_id.max(r.id);
        }
        self.write_chunk(&chunk)?;
        if self.info.files == 0 && self.len > 0 {
            self.info.files = 1;
        }
        Ok(())
    }

    fn write_chunk(&mut self, chunk: &[u8]) -> io::Result<()> {
        if chunk.is_empty() {
            return Ok(());
        }
        self.sd
            .append(&format!("{DIR}/{}", file_name(self.file)), chunk)?;
        self.len += chunk.len() as u64;
        self.info.bytes += chunk.len() as u64;
        Ok(())
    }

    /// The latest `date` in the newest file: the board's best guess at
    /// "today" before its clock is set (entries are appended roughly in
    /// date order).
    pub fn newest_date(&self) -> io::Result<Option<String>> {
        let Some(name) = Self::names(&self.sd)?.pop() else {
            return Ok(None);
        };
        let bytes = self.sd.read(&format!("{DIR}/{name}"))?;
        Ok(records(&bytes)
            .filter(|(_, e)| !e.deleted)
            .map(|(_, e)| e.date)
            .max())
    }

    /// A tombstone for `id`.
    pub fn tombstone(id: u32) -> StoredEntry {
        StoredEntry {
            id,
            deleted: true,
            ..Default::default()
        }
    }

    /// Calls `f` with every entry's final version (deleted ids left out),
    /// oldest file first.
    pub fn for_each(&self, f: impl FnMut(StoredEntry)) -> io::Result<()> {
        self.for_each_where(|_| true, f)
    }

    /// Like [`Journal::for_each`], for the final versions whose head
    /// `keep` accepts (a person, a date range). Two passes over the card:
    /// an index of each id's last record from cheap head peeks (20 bytes
    /// per record), then a full decode of only the records wanted.
    pub fn for_each_where(
        &self,
        keep: impl Fn(&Head) -> bool,
        mut f: impl FnMut(StoredEntry),
    ) -> io::Result<()> {
        let names = Self::names(&self.sd)?;
        let mut slots: Vec<Slot> = Vec::with_capacity(self.info.records as usize);
        for (i, name) in names.iter().enumerate() {
            let bytes = self.sd.read(&format!("{DIR}/{name}"))?;
            for (at, payload) in Frames::new(&bytes) {
                let Some(head) = head_of(payload) else { break };
                slots.push(Slot {
                    head,
                    file: i as u32,
                    offset: at as u32,
                });
            }
        }
        // Stable: among one id's records, file order is kept, so the last wins.
        slots.sort_by_key(|s| s.head.id);
        let mut finals: Vec<(u32, u32)> = Vec::new();
        for (i, s) in slots.iter().enumerate() {
            let last_of_id = slots.get(i + 1).is_none_or(|n| n.head.id != s.head.id);
            if last_of_id && !s.head.deleted && keep(&s.head) {
                finals.push((s.file, s.offset));
            }
        }
        drop(slots);
        finals.sort_unstable();
        let mut want = finals.iter().peekable();
        for (i, name) in names.iter().enumerate() {
            if want.peek().is_none_or(|(file, _)| *file != i as u32) {
                continue;
            }
            let bytes = self.sd.read(&format!("{DIR}/{name}"))?;
            for (at, payload) in Frames::new(&bytes) {
                if want.peek() == Some(&&(i as u32, at as u32)) {
                    want.next();
                    if let Some(e) = decode(payload) {
                        f(e);
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ml-journal-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn entry(id: u32, note: &str) -> StoredEntry {
        StoredEntry {
            id,
            person: 1,
            kind: "meditation".into(),
            date: "2026-10-18".into(),
            amount: 20,
            note: note.into(),
            visibility: "public".into(),
            rev: 1,
            ..Default::default()
        }
    }

    fn all(j: &Journal) -> Vec<StoredEntry> {
        let mut out = Vec::new();
        j.for_each(|e| out.push(e)).unwrap();
        out.sort_by_key(|e| e.id);
        out
    }

    fn notes(j: &Journal) -> Vec<(u32, String)> {
        all(j).into_iter().map(|e| (e.id, e.note)).collect()
    }

    #[test]
    fn last_version_wins_and_tombstones_delete() {
        let dir = temp("wins");
        let sd = Arc::new(Sd::at(&dir, None));
        let mut j = Journal::open(sd.clone()).unwrap();
        assert!(all(&j).is_empty());
        j.append(&[entry(1, "first"), entry(2, "keep")]).unwrap();
        j.append(&[entry(1, "edited"), Journal::tombstone(2), entry(3, "new")])
            .unwrap();
        let j = Journal::open(sd).unwrap();
        assert_eq!(
            notes(&j),
            [(1, "edited".to_string()), (3, "new".to_string())]
        );
        assert_eq!((j.info.records, j.info.max_id), (5, 3));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_deleted_id_counts_toward_max_id() {
        let dir = temp("maxid");
        let sd = Arc::new(Sd::at(&dir, None));
        let mut j = Journal::open(sd.clone()).unwrap();
        j.append(&[entry(7, "x"), Journal::tombstone(7)]).unwrap();
        let j = Journal::open(sd).unwrap();
        assert!(all(&j).is_empty());
        assert_eq!(j.info.max_id, 7);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_torn_tail_is_cut_and_appends_continue() {
        let dir = temp("torn");
        let sd = Arc::new(Sd::at(&dir, None));
        let mut j = Journal::open(sd.clone()).unwrap();
        j.append(&[entry(1, "safe")]).unwrap();
        j.append(&[entry(2, "torn")]).unwrap();
        let path = dir.join("ml/journal/000001.log");
        let len = std::fs::metadata(&path).unwrap().len();
        let f = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        f.set_len(len - 5).unwrap();
        drop(f);
        let mut j = Journal::open(sd.clone()).unwrap();
        assert_eq!(notes(&j), [(1, "safe".to_string())]);
        assert!(j.info.torn_bytes > 0);
        // Appending after the repair is readable.
        j.append(&[entry(3, "after")]).unwrap();
        let j = Journal::open(sd).unwrap();
        assert_eq!(all(&j).iter().map(|e| e.id).collect::<Vec<_>>(), [1, 3]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rotates_files_even_within_one_batch_and_reads_across_them() {
        let dir = temp("rotate");
        let sd = Arc::new(Sd::at(&dir, None));
        let mut j = Journal::open(sd.clone()).unwrap();
        // ~2.6 MiB in one batch (an import) splits into several files.
        let note = "x".repeat(400);
        let records: Vec<_> = (1..=6_000u32).map(|i| entry(i, &note)).collect();
        j.append(&records).unwrap();
        let files = sd.list("ml/journal").unwrap();
        assert!(files.len() >= 3, "{files:?}");
        for f in &files {
            let len = std::fs::metadata(dir.join("ml/journal").join(f))
                .unwrap()
                .len();
            assert!(len <= ROTATE_AT, "{f}: {len}");
        }
        // An edit of an entry in the first file, written to the last.
        j.append(&[entry(1, "edited late")]).unwrap();
        let j = Journal::open(sd).unwrap();
        let streamed = all(&j);
        assert_eq!(streamed.len(), 6_000);
        assert_eq!(streamed[0].note, "edited late");
        assert_eq!(j.info.records, 6_001);
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// The fast path must agree with a full decode, and must actually be
    /// taken (else every read silently falls back to decoding in full).
    #[test]
    fn peek_matches_a_full_decode() {
        let mut cases = vec![Journal::tombstone(5), Journal::tombstone(70_000)];
        for (id, person) in [
            (1u32, 1u32),
            (23, 2),
            (24, 255),
            (300, 256),
            (70_000, 65_536),
            (u32::MAX, 3),
        ] {
            cases.push(StoredEntry {
                id,
                person,
                note: "n".repeat(id as usize % 600),
                rejoiced_by: vec![1, 300, 70_000],
                ..entry(id, "")
            });
        }
        cases.push(StoredEntry {
            date: String::new(),
            ..entry(9, "no date")
        });
        for e in cases {
            let payload = wire::to_vec(Format::CborInt, &e).unwrap();
            let head = peek(&payload).unwrap_or_else(|| panic!("peek fell back for {e:?}"));
            assert_eq!(head.id, e.id);
            assert_eq!(head.person, e.person);
            assert_eq!(head.deleted, e.deleted);
            let day = crate::tz::Date::parse(&e.date).map_or(i32::MIN, |d| d.days() as i32);
            assert_eq!(head.day, day);
        }
        assert!(peek(b"\x01\x02").is_none());
    }

    #[test]
    fn for_each_where_reads_only_what_it_keeps() {
        let dir = temp("where");
        let sd = Arc::new(Sd::at(&dir, None));
        let mut j = Journal::open(sd.clone()).unwrap();
        j.append(&[
            entry(1, "a"),
            StoredEntry {
                person: 2,
                ..entry(2, "b")
            },
            entry(3, "c"),
        ])
        .unwrap();
        j.append(&[StoredEntry {
            person: 2,
            ..entry(1, "moved to 2")
        }])
        .unwrap();
        let mut got = Vec::new();
        j.for_each_where(|h| h.person == 2, |e| got.push((e.id, e.note)))
            .unwrap();
        got.sort();
        assert_eq!(got, [(1, "moved to 2".to_string()), (2, "b".to_string())]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Found on the board: FAT lists the 8.3 name `000001.log` as
    /// `000001.LOG`. The journal must still find (and append to) it.
    #[test]
    fn upper_case_names_from_fat_are_found() {
        let dir = temp("fatcase");
        let sd = Arc::new(Sd::at(&dir, None));
        std::fs::create_dir_all(dir.join("ml/journal")).unwrap();
        std::fs::write(
            dir.join("ml/journal/000001.LOG"),
            encode(&entry(1, "a")).unwrap(),
        )
        .unwrap();
        let mut j = Journal::open(sd.clone()).unwrap();
        assert_eq!(j.info.records, 1);
        j.append(&[entry(2, "b")]).unwrap();
        let j = Journal::open(sd).unwrap();
        assert_eq!(all(&j).iter().map(|e| e.id).collect::<Vec<_>>(), [1, 2]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_bad_record_in_an_older_file_keeps_the_rest() {
        let dir = temp("damaged");
        let sd = Arc::new(Sd::at(&dir, None));
        std::fs::create_dir_all(dir.join("ml/journal")).unwrap();
        let mut one = encode(&entry(1, "a")).unwrap();
        let mut bad = encode(&entry(2, "b")).unwrap();
        bad[HEADER] ^= 0xFF;
        one.extend_from_slice(&bad);
        one.extend_from_slice(&encode(&entry(3, "c")).unwrap());
        std::fs::write(dir.join("ml/journal/000001.log"), one).unwrap();
        std::fs::write(
            dir.join("ml/journal/000002.log"),
            encode(&entry(4, "d")).unwrap(),
        )
        .unwrap();
        let j = Journal::open(sd).unwrap();
        assert_eq!(all(&j).iter().map(|e| e.id).collect::<Vec<_>>(), [1, 4]);
        assert_eq!(j.info.damaged_files, ["000001.log"]);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
