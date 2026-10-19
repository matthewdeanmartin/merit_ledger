//! The SD card: long history and backups (`spec/domain.md` § Storage map).
//!
//! On the board ESP-IDF mounts the card's FAT filesystem into the VFS at
//! `/sd`, so this module is plain `std::fs` on both platforms: the desktop
//! uses a directory (`.local/sd`) instead. The board runs fine without a
//! card; everything here then reports "absent" and writes fail cleanly.
//!
//! Write from workers or the main task, never from the serving task: an SD
//! write can stall for tens of milliseconds.
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Total and free bytes of the filesystem at a path, if the platform can say.
pub type SpaceFn = fn(&Path) -> Option<(u64, u64)>;

/// Where the card is mounted, if it is.
pub struct Sd {
    root: Option<PathBuf>,
    space: Option<SpaceFn>,
}

/// What `GET /api/v1/admin/storage` reports about the card.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SdInfo {
    pub present: bool,
    pub total_bytes: u64,
    pub free_bytes: u64,
}

/// One write-sync-read round trip (`Sd::self_test`).
#[derive(Clone, Debug, Default)]
pub struct SdTest {
    pub bytes: u64,
    pub write_ms: u64,
    pub read_ms: u64,
}

impl Sd {
    /// No card: every write fails, `info` says absent.
    pub fn none() -> Self {
        Self {
            root: None,
            space: None,
        }
    }

    /// A card (or the desktop's stand-in directory) at `root`.
    pub fn at(root: impl Into<PathBuf>, space: Option<SpaceFn>) -> Self {
        Self {
            root: Some(root.into()),
            space,
        }
    }

    pub fn present(&self) -> bool {
        self.root.is_some()
    }

    /// `rel` under the card, refusing anything that could climb out of it.
    pub fn path(&self, rel: &str) -> io::Result<PathBuf> {
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no SD card"))?;
        if rel.is_empty()
            || rel
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "bad SD path"));
        }
        Ok(root.join(rel))
    }

    pub fn info(&self) -> SdInfo {
        let Some(root) = &self.root else {
            return SdInfo::default();
        };
        let (total_bytes, free_bytes) = self.space.and_then(|space| space(root)).unwrap_or((0, 0));
        SdInfo {
            present: true,
            total_bytes,
            free_bytes,
        }
    }

    /// Appends `data` to `rel` (creating folders) and syncs it to the card.
    pub fn append(&self, rel: &str, data: &[u8]) -> io::Result<()> {
        let path = self.path(rel)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        file.write_all(data)?;
        file.sync_all()
    }

    /// The whole file at `rel`.
    pub fn read(&self, rel: &str) -> io::Result<Vec<u8>> {
        fs::read(self.path(rel)?)
    }

    /// File names directly in folder `rel`, lower-cased and sorted; empty
    /// when it doesn't exist. Lower-cased because FAT may store an 8.3 name
    /// such as `000001.log` as `000001.LOG`; FAT is case-insensitive, so the
    /// lower-case name opens the same file either way.
    pub fn list(&self, rel: &str) -> io::Result<Vec<String>> {
        let dir = self.path(rel)?;
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut names: Vec<String> = entries
            .filter_map(|e| e.ok())
            // `stat` by path: on ESP-IDF a directory entry may carry no file
            // type, and `DirEntry::file_type` then needs `lstat`, which the
            // FAT VFS doesn't offer, so it fails and every file would vanish.
            .filter(|e| fs::metadata(e.path()).is_ok_and(|m| m.is_file()))
            .filter_map(|e| e.file_name().into_string().ok())
            .map(|n| n.to_lowercase())
            .collect();
        names.sort();
        Ok(names)
    }

    /// Cuts `rel` to `len` bytes (a torn record at the end of a log).
    pub fn truncate(&self, rel: &str, len: u64) -> io::Result<()> {
        let file = OpenOptions::new().write(true).open(self.path(rel)?)?;
        file.set_len(len)?;
        file.sync_all()
    }

    /// Writes `data` as `rel` all at once: to `rel.tmp`, synced, then
    /// renamed over `rel`, so a reader never sees half a file.
    pub fn write_file(&self, rel: &str, data: &[u8]) -> io::Result<()> {
        let path = self.path(rel)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("tmp");
        {
            let mut file = File::create(&tmp)?;
            file.write_all(data)?;
            file.sync_all()?;
        }
        // FAT can't rename over an existing file.
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        fs::rename(tmp, path)
    }

    pub fn remove(&self, rel: &str) -> io::Result<()> {
        fs::remove_file(self.path(rel)?)
    }

    /// The same card, rooted at folder `rel` (the gate's scratch journal).
    pub fn sub(&self, rel: &str) -> io::Result<Sd> {
        Ok(Sd {
            root: Some(self.path(rel)?),
            space: self.space,
        })
    }

    /// Writes `bytes` of a known pattern to `rel`, syncs, reads it back and
    /// compares. Sprint 1 measures the card with this; the file is removed.
    pub fn self_test(&self, rel: &str, bytes: usize) -> io::Result<SdTest> {
        let path = self.path(rel)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let block: Vec<u8> = (0..4096u32).map(|i| (i * 31 % 251) as u8).collect();
        let started = Instant::now();
        {
            let mut file = File::create(&path)?;
            let mut left = bytes;
            while left > 0 {
                let n = left.min(block.len());
                file.write_all(&block[..n])?;
                left -= n;
            }
            file.sync_all()?;
        }
        let write_ms = started.elapsed().as_millis() as u64;
        let started = Instant::now();
        let mut file = File::open(&path)?;
        let mut buf = vec![0u8; block.len()];
        let mut seen = 0usize;
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            if buf[..n] != block[..n] {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "SD read-back differs",
                ));
            }
            seen += n;
        }
        let read_ms = started.elapsed().as_millis() as u64;
        drop(file);
        fs::remove_file(&path)?;
        if seen != bytes {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "SD read-back is short",
            ));
        }
        Ok(SdTest {
            bytes: bytes as u64,
            write_ms,
            read_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ml-sd-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn no_card_reports_absent_and_refuses_writes() {
        let sd = Sd::none();
        assert_eq!(sd.info(), SdInfo::default());
        assert_eq!(
            sd.append("a.log", b"x").unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }

    #[test]
    fn paths_cannot_climb_out() {
        let sd = Sd::at(temp(), None);
        for bad in ["", "../x", "a/../../x", "/abs", "a//b", "./a"] {
            assert!(sd.path(bad).is_err(), "{bad}");
        }
        assert!(sd.path("ml/history/2026-10.log").is_ok());
    }

    #[test]
    fn append_creates_folders_and_accumulates() {
        let root = temp();
        let sd = Sd::at(&root, None);
        sd.append("ml/t/a.log", b"one").unwrap();
        sd.append("ml/t/a.log", b"two").unwrap();
        assert_eq!(fs::read(root.join("ml/t/a.log")).unwrap(), b"onetwo");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn self_test_round_trips_and_cleans_up() {
        let root = temp();
        let sd = Sd::at(&root, Some(|_| Some((100, 40))));
        let t = sd.self_test("ml/selftest.bin", 10_000).unwrap();
        assert_eq!(t.bytes, 10_000);
        assert!(!root.join("ml/selftest.bin").exists());
        assert_eq!(
            sd.info(),
            SdInfo {
                present: true,
                total_bytes: 100,
                free_bytes: 40
            }
        );
        fs::remove_dir_all(root).unwrap();
    }
}
