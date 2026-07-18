//! Whole-file parallel encode benchmark. The entire input is ONE document
//! handed to the library's parallel encode path (`encode_docs_ragged`) —
//! the same chunking policy, pretoken-safe splitting, and persistent worker
//! pool as `BPETokenizer.encode_batch` / `encode_files` — so this measures
//! gigatoken's own parallelism, not a bench-local split.
//!
//! Run with: cargo bench --bench encode                 (full OWT)
//!           ENCODE_MB=500 cargo bench --bench encode
//!           TOKENIZER_JSON=/path/to/tokenizer.json cargo bench --bench encode

use gigatoken_rs::load_tokenizer::hf::load_hf_bpe;
use gigatoken_rs::{WorkerPool, encode_docs_ragged};
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

mod common;
fn main() {
    common::allow_thp();
    // TOKENIZER_JSON overrides the default (GPT-2, from the local HF cache
    // or the committed fixture) with an explicit tokenizer.json path.
    let tokenizer_path = std::env::var("TOKENIZER_JSON")
        .map(PathBuf::from)
        .unwrap_or_else(|_| common::gpt2_tokenizer_json());
    eprintln!("Loading tokenizer from {tokenizer_path:?}...");
    let tokenizer = load_hf_bpe(&tokenizer_path).expect("Could not load tokenizer");

    let input = common::load_owt_input(None);
    let size_gb = input.len() as f64 / 1e9;

    eprintln!(
        "Encoding (1 document, {} threads)...",
        rayon::current_num_threads()
    );
    let workers = WorkerPool::new();
    let start = Instant::now();
    let (ids, lens) = encode_docs_ragged(&workers, &tokenizer, &[&input]);
    black_box((&ids, lens));
    let elapsed = start.elapsed().as_secs_f64();
    let throughput_gb = size_gb / elapsed;

    eprintln!(
        "{} tokens in {elapsed:.2}s — {throughput_gb:.2} GB/s ({:.0} MB/s)",
        ids.len(),
        throughput_gb * 1000.0
    );
}
