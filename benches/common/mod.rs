//! Shared input loading for the bench targets. Lives in a subdirectory so
//! cargo does not treat it as a bench target; each bench pulls it in with
//! `mod common;`.

use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Sequential phase recorder for profiling runs.
///
/// Each `phase()` call closes the previous phase and opens the next;
/// `finish()` closes the last one and, when `PHASE_FILE` is set, writes a
/// JSON sidecar with epoch-ns phase boundaries plus free-form `meta`
/// key/values. The trace analyzers (`profiling/analyze.py`,
/// `profiling/pmu_summary.py`) align those epoch timestamps with the
/// trace's own recorded start time, so samples and PMU windows are cut by
/// measured phase — no stack heuristics, no hand-guessed `--window`.
/// Epoch clocks on both sides make the alignment exact to a few ms, far
/// below the seconds-scale phases.
pub struct Phases {
    entries: Vec<(String, u128, u128)>, // (name, start epoch ns, end epoch ns)
    open: Option<(String, u128)>,
    meta: Vec<(String, String)>,
}

fn epoch_ns() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos()
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

impl Phases {
    pub fn new() -> Self {
        Phases {
            entries: Vec::new(),
            open: None,
            meta: Vec::new(),
        }
    }

    /// Close the current phase (if any) and start `name` now.
    pub fn phase(&mut self, name: impl Into<String>) {
        let now = epoch_ns();
        if let Some((prev, start)) = self.open.take() {
            self.entries.push((prev, start, now));
        }
        self.open = Some((name.into(), now));
    }

    /// Attach provenance (tokenizer path, input size, per-pass results, ...)
    /// to the sidecar; analyzers print it verbatim in their report header.
    pub fn meta(&mut self, key: impl Into<String>, value: impl std::fmt::Display) {
        self.meta.push((key.into(), value.to_string()));
    }

    /// Close the last phase and write the sidecar to `$PHASE_FILE` (no-op
    /// when the variable is unset, so plain bench runs write nothing).
    pub fn finish(mut self) {
        // Closes the last real phase; the "<end>" phase itself stays open
        // and is never emitted.
        self.phase("<end>");
        let Some(path) = std::env::var_os("PHASE_FILE") else {
            return;
        };
        let mut out = String::from("{\n  \"phases\": [\n");
        for (i, (name, start, end)) in self.entries.iter().enumerate() {
            let sep = if i + 1 == self.entries.len() { "" } else { "," };
            out.push_str(&format!(
                "    {{\"name\": \"{}\", \"start_epoch_ns\": {start}, \"end_epoch_ns\": {end}}}{sep}\n",
                json_escape(name)
            ));
        }
        out.push_str("  ],\n  \"meta\": {\n");
        for (i, (k, v)) in self.meta.iter().enumerate() {
            let sep = if i + 1 == self.meta.len() { "" } else { "," };
            out.push_str(&format!(
                "    \"{}\": \"{}\"{sep}\n",
                json_escape(k),
                json_escape(v)
            ));
        }
        out.push_str("  }\n}\n");
        if let Err(e) = std::fs::write(&path, out) {
            eprintln!("warn: could not write PHASE_FILE {path:?}: {e}");
        } else {
            eprintln!("phases: {path:?}");
        }
    }
}

/// HF hub cache dir: HF_HUB_CACHE, else $HF_HOME/hub, else
/// $XDG_CACHE_HOME/huggingface/hub, else ~/.cache/huggingface/hub.
/// Mirrors `src/test_hub.rs` / `tests/hf_cache.py` (benches are a separate
/// crate and cannot reach the `#[cfg(test)]` module in the lib).
fn hub_cache_dir() -> PathBuf {
    let env = |key: &str| {
        std::env::var(key)
            .ok()
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    if let Some(hub_cache) = env("HF_HUB_CACHE") {
        return hub_cache;
    }
    let hf_home = env("HF_HOME").unwrap_or_else(|| {
        env("XDG_CACHE_HOME")
            .unwrap_or_else(|| std::env::home_dir().expect("home dir").join(".cache"))
            .join("huggingface")
    });
    hf_home.join("hub")
}

/// `filename` from a model repo's `main` snapshot in the local HF cache,
/// or None when the repo, ref, or file is not cached.
pub fn cached_hub_file(repo_id: &str, filename: &str) -> Option<PathBuf> {
    let repo_dir = hub_cache_dir().join(format!("models--{}", repo_id.replace('/', "--")));
    let commit = std::fs::read_to_string(repo_dir.join("refs/main")).ok()?;
    let path = repo_dir
        .join("snapshots")
        .join(commit.trim())
        .join(filename);
    path.is_file().then_some(path)
}

/// A model repo's tokenizer.json from the local HF cache. Benches never
/// download; a miss aborts with the command that populates the cache.
pub fn hf_tokenizer_json(repo_id: &str) -> PathBuf {
    cached_hub_file(repo_id, "tokenizer.json").unwrap_or_else(|| {
        panic!(
            "tokenizer.json for {repo_id} not in the HF cache; \
             fetch it with: uv run hf download {repo_id} tokenizer.json"
        )
    })
}

/// GPT-2's tokenizer.json: the HF cache copy when present, else the
/// committed test fixture (a verbatim copy of the openai-community/gpt2
/// file), so the default bench runs on a machine with no HF cache.
pub fn gpt2_tokenizer_json() -> PathBuf {
    cached_hub_file("openai-community/gpt2", "tokenizer.json").unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gpt2_tokenizer.json")
    })
}

/// Re-enable transparent huge pages for this process. The encode paths
/// madvise their big tables and buffers to 2 MiB pages (they far exceed
/// 4 KiB dTLB coverage, and Zen drops software prefetches that miss the
/// TLB), but some session managers launch children with
/// PR_SET_THP_DISABLE, which silently vetoes MADV_HUGEPAGE; clear it so
/// the bench measures the tokenizer, not the launcher's memory policy.
/// No-op off Linux.
pub fn allow_thp() {
    #[cfg(target_os = "linux")]
    // SAFETY: prctl(PR_SET_THP_DISABLE, 0) only clears a per-process flag.
    unsafe {
        libc::prctl(libc::PR_SET_THP_DISABLE, 0, 0, 0, 0);
    }
}

/// Hint 2 MiB pages for a buffer's reserved capacity, BEFORE it is first
/// written (the ordering is what makes the hint effective: pages fault in
/// huge only if the madvise precedes the first touch). Multi-GB bench
/// buffers otherwise saturate the dTLB alongside the encode's own tables.
/// No-op off Linux.
#[allow(unused_variables, clippy::missing_safety_doc)]
pub fn madvise_hugepage_capacity<T>(v: &mut Vec<T>) {
    #[cfg(target_os = "linux")]
    if v.capacity() > 0 {
        // Align the start inward: malloc's mmap chunks carry a 16-byte
        // header, and an unaligned madvise start is EINVAL (silent no-op).
        const PAGE: usize = 4096;
        let addr = v.as_mut_ptr() as usize;
        let start = (addr + PAGE - 1) & !(PAGE - 1);
        let end = addr + v.capacity() * std::mem::size_of::<T>();
        if end > start {
            // SAFETY: the range is one live allocation; the hint neither
            // reads nor writes it.
            unsafe {
                libc::madvise(start as *mut libc::c_void, end - start, libc::MADV_HUGEPAGE);
            }
        }
    }
}

/// Load the benchmark input from `~/data/owt_train.txt`, truncated to a
/// UTF-8 character boundary.
///
/// ENCODE_MB caps the input for fast profiling iterations (only that many
/// bytes are read from disk, so the read does not dominate a profile of the
/// encode loop). When it is unset, `default_mb` applies; `None` reads the
/// whole file.
pub fn load_owt_input(default_mb: Option<usize>) -> Vec<u8> {
    let owt_path = std::env::home_dir().unwrap().join("data/owt_train.txt");
    eprintln!("Reading {owt_path:?}...");
    let t0 = Instant::now();

    let cap_mb = std::env::var("ENCODE_MB")
        .ok()
        .map(|mb| {
            mb.trim()
                .parse::<usize>()
                .expect("ENCODE_MB must be an integer")
        })
        .or(default_mb);
    let mut data = match cap_mb {
        Some(mb) => {
            use std::io::Read;
            let max_bytes = mb * 1_000_000;
            let file = std::fs::File::open(&owt_path).expect("Could not open ~/data/owt_train.txt");
            let mut data = Vec::with_capacity(max_bytes);
            madvise_hugepage_capacity(&mut data);
            file.take(max_bytes as u64)
                .read_to_end(&mut data)
                .expect("read failed");
            data
        }
        None => {
            let len = std::fs::metadata(&owt_path)
                .expect("Could not stat ~/data/owt_train.txt")
                .len() as usize;
            use std::io::Read;
            let mut data = Vec::with_capacity(len + 1);
            madvise_hugepage_capacity(&mut data);
            std::fs::File::open(&owt_path)
                .expect("Could not open ~/data/owt_train.txt")
                .read_to_end(&mut data)
                .expect("read failed");
            data
        }
    };
    // Back up to a UTF-8 character boundary (a byte cap can split a
    // multibyte character).
    if let Err(e) = std::str::from_utf8(&data) {
        data.truncate(e.valid_up_to());
    }
    eprintln!(
        "Read {:.2} GB in {:.1}s",
        data.len() as f64 / 1e9,
        t0.elapsed().as_secs_f64()
    );
    data
}
