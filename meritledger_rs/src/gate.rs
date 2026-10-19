//! Sprint 1 hardware gate (`sprints/sprint01.md`): while running, a worker
//! fetches each target over HTTPS in turn, one connection at a time, so
//! outbound TLS overlaps whatever load the bench puts on the server, and a
//! second worker writes to the SD card. Results are counts and timings;
//! heap comes from `/api/v1/sys`. Built only with the `gate` feature, never
//! in a release. Ported from gamematcher.
use miniframework::message;
use std::io;
use std::sync::Mutex;
use std::time::{Duration, Instant};

message! {
    pub struct GateTarget {
        1 name: String,
        2 url: String,
        3 ok: u32,
        4 failed: u32,
        5 last_ms: u32,
        6 max_ms: u32,
        7 last_error: String,
        8 last_status: u32,
        9 last_bytes: u32,
    }
}

message! {
    pub struct GateStats {
        1 running: bool,
        2 remaining_secs: u32,
        3 targets: Vec<GateTarget>,
    }
}

#[derive(Default)]
pub struct Gate {
    inner: Mutex<(Option<Instant>, Vec<GateTarget>)>,
}

/// One outbound fetch: (status, body bytes).
pub type FetchFn = dyn Fn(&str) -> io::Result<(u16, usize)> + Send + Sync;

impl Gate {
    pub fn set_targets(&self, targets: &[(&str, &str)]) {
        let mut inner = self.inner.lock().unwrap();
        inner.1 = targets
            .iter()
            .map(|(name, url)| GateTarget {
                name: name.to_string(),
                url: url.to_string(),
                ..Default::default()
            })
            .collect();
    }

    pub fn start(&self, secs: u64) {
        let mut inner = self.inner.lock().unwrap();
        inner.0 = Some(Instant::now() + Duration::from_secs(secs));
        for t in &mut inner.1 {
            *t = GateTarget {
                name: std::mem::take(&mut t.name),
                url: std::mem::take(&mut t.url),
                ..Default::default()
            };
        }
    }

    pub fn stats(&self) -> GateStats {
        let inner = self.inner.lock().unwrap();
        let remaining = inner.0.map_or(0, |end| {
            end.saturating_duration_since(Instant::now()).as_secs()
        });
        GateStats {
            running: remaining > 0,
            remaining_secs: remaining as u32,
            targets: inner.1.clone(),
        }
    }

    /// One round: fetch every target once if running. `fetchers[i]` serves
    /// target `i`. Never holds the lock during a fetch.
    pub fn run_once(&self, fetchers: &[&FetchFn]) {
        let urls: Vec<String> = {
            let inner = self.inner.lock().unwrap();
            if inner.0.is_none_or(|end| Instant::now() >= end) {
                return;
            }
            inner.1.iter().map(|t| t.url.clone()).collect()
        };
        for (i, url) in urls.iter().enumerate() {
            let Some(fetch) = fetchers.get(i) else { break };
            let started = Instant::now();
            let result = fetch(url);
            let ms = started.elapsed().as_millis() as u32;
            let mut inner = self.inner.lock().unwrap();
            let Some(t) = inner.1.get_mut(i) else { break };
            t.last_ms = ms;
            t.max_ms = t.max_ms.max(ms);
            match result {
                Ok((status, bytes)) if (200..300).contains(&status) => {
                    t.ok += 1;
                    t.last_status = status as u32;
                    t.last_bytes = bytes as u32;
                }
                Ok((status, _)) => {
                    t.failed += 1;
                    t.last_status = status as u32;
                    t.last_error = format!("HTTP {status}");
                }
                Err(e) => {
                    t.failed += 1;
                    t.last_error = e.to_string();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_successes_and_failures() {
        let gate = Gate::default();
        gate.set_targets(&[("a", "https://a"), ("b", "https://b")]);
        let ok = |_: &str| Ok((200u16, 10usize));
        let bad = |_: &str| Err(io::Error::other("tls alloc failed"));
        gate.run_once(&[&ok, &bad]);
        assert_eq!(gate.stats().targets[0].ok, 0, "idle until started");
        gate.start(60);
        gate.run_once(&[&ok, &bad]);
        let stats = gate.stats();
        assert!(stats.running);
        assert_eq!(stats.targets[0].ok, 1);
        assert_eq!(stats.targets[1].failed, 1);
        assert_eq!(stats.targets[1].last_error, "tls alloc failed");
    }
}

message! {
    /// `POST /api/v1/gate/journal?entries=N`: a synthetic journal, timed.
    pub struct JournalBench {
        1 records: u32,
        2 bytes: u64,
        /// Writing them in batches of 500 (one fsync each).
        3 write_ms: u32,
        /// One record per append, as a tap does: average of 20.
        4 append_one_ms: u32,
        /// `Journal::open` (check and count): the first of a boot's two reads.
        5 open_ms: u32,
        /// `Journal::for_each`: the second read at boot, and what an export or
        /// an old-history read spends.
        6 for_each_ms: u32,
        /// Journal files `Sd::list` saw before cleaning up (0 means listing
        /// is broken on this card or VFS).
        7 listed_files: u32,
        /// One person's month, 100 to 130 days back: a profile's old history.
        8 month_ms: u32,
        9 month_entries: u32,
    }
}

/// Writes `n` synthetic entries (4 people, 5 a day, going back in time)
/// to a journal under `sd`, times the operations a real board does, then
/// deletes it. `sd` must be a scratch folder, never the household's.
pub fn journal_bench(sd: std::sync::Arc<crate::sd::Sd>, n: u32) -> io::Result<JournalBench> {
    use crate::journal::Journal;
    use crate::messages::StoredEntry;
    let clean = |sd: &crate::sd::Sd| -> io::Result<()> {
        for name in sd.list("ml/journal")? {
            sd.remove(&format!("ml/journal/{name}"))?;
        }
        Ok(())
    };
    clean(&sd)?;
    let today = crate::tz::Date::parse("2026-10-18").unwrap();
    let entry = |id: u32| StoredEntry {
        id,
        person: 1 + id % 4,
        kind: [
            "meditation",
            "chanting",
            "reading",
            "vegetarian",
            "prostrations",
        ][(id % 5) as usize]
            .into(),
        date: today.add_days(-((id / 20) as i64)).iso(),
        created: 1_792_281_600 - u64::from(id) * 4_000,
        amount: 20,
        note: if id.is_multiple_of(7) {
            "a short note".into()
        } else {
            String::new()
        },
        visibility: "public".into(),
        source: "tap".into(),
        recorded_by: 1 + id % 4,
        rev: 1,
        ..Default::default()
    };
    let mut j = Journal::open(sd.clone())?;
    let started = Instant::now();
    let mut id = 1;
    while id <= n {
        let batch: Vec<_> = (id..=(id + 499).min(n)).map(entry).collect();
        j.append(&batch)?;
        id += 500;
    }
    let write_ms = started.elapsed().as_millis() as u32;
    let started = Instant::now();
    for i in 0..20 {
        j.append(&[entry(n + 1 + i)])?;
    }
    let append_one_ms = (started.elapsed().as_millis() / 20) as u32;
    let started = Instant::now();
    let j = Journal::open(sd.clone())?;
    let open_ms = started.elapsed().as_millis() as u32;
    let started = Instant::now();
    let mut seen = 0u32;
    j.for_each(|_| seen += 1)?;
    let for_each_ms = started.elapsed().as_millis() as u32;
    let started = Instant::now();
    let (lo, hi) = (
        today.add_days(-130).days() as i32,
        today.add_days(-100).days() as i32,
    );
    let mut month_entries = 0u32;
    j.for_each_where(
        |h| h.person == 1 && h.day >= lo && h.day <= hi,
        |_| month_entries += 1,
    )?;
    let month_ms = started.elapsed().as_millis() as u32;
    let listed_files = sd.list("ml/journal")?.len() as u32;
    let bench = JournalBench {
        listed_files,
        month_ms,
        month_entries,
        records: seen,
        bytes: j.info.bytes,
        write_ms,
        append_one_ms,
        open_ms,
        for_each_ms,
    };
    clean(&sd)?;
    if u64::from(seen) != j.info.records {
        return Err(io::Error::other(
            "for_each found a different count than open",
        ));
    }
    Ok(bench)
}
