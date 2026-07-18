//! Single-threaded SentencePiece encode throughput on OWT, mirroring
//! `encode_st` (GPT-2 byte-level) for a like-for-like comparison.
//!
//! Select the tokenizer with SP_TOKENIZER=tinyllama (default, from the
//! local HF cache) or an explicit tokenizer.json path; cap the input with
//! ENCODE_MB like `encode_st`.

use gigatoken_rs::load_tokenizer::hf::load_hf_sentencepiece;
use std::path::PathBuf;
use std::time::Instant;

mod common;

fn main() {
    let which = std::env::var("SP_TOKENIZER").unwrap_or_else(|_| "tinyllama".to_string());
    let tokenizer_path = match which.as_str() {
        "tinyllama" => common::hf_tokenizer_json("TinyLlama/TinyLlama-1.1B-Chat-v1.0"),
        // The sp4096 parameter-golf model exists only as a raw .model in the
        // Ryukijano/parameter-golf-sp4096 dataset repo; the .model ->
        // tokenizer.json conversion lives in Python. Produce the json once:
        //   uv run python -c "from tests.hf_cache import hf_file; from gigatoken._load.sentencepiece import sentencepiece_to_tokenizer_json; import pathlib; pathlib.Path('sp4096_tokenizer.json').write_text(sentencepiece_to_tokenizer_json(hf_file('Ryukijano/parameter-golf-sp4096', 'tokenizers/fineweb_4096_bpe.model', repo_type='dataset').read_bytes()))"
        // then pass SP_TOKENIZER=sp4096_tokenizer.json.
        "sp4096" => panic!(
            "sp4096 needs a converted tokenizer.json; see the comment in benches/encode_st_sp.rs"
        ),
        path => PathBuf::from(path),
    };
    eprintln!("Loading {which} tokenizer from {tokenizer_path:?}...");
    let tokenizer = load_hf_sentencepiece(&tokenizer_path).expect("Could not load tokenizer");

    let input = common::load_owt_input(None);
    let size_gb = input.len() as f64 / 1e9;
    let text = std::str::from_utf8(&input).expect("input must be UTF-8");

    eprintln!("Encoding (single-threaded)...");
    // Count-only callback, mirroring encode_st's measurement of the GPT-2
    // path (the full encode runs; only output materialization is skipped).
    let passes: usize = std::env::var("SP_PASSES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2);
    let mut state = gigatoken_rs::EncodeState::new();
    for pass in 1..=passes {
        let mut total_tokens: usize = 0;
        let start = Instant::now();
        tokenizer.encode_raw_cb(&mut state, text, &mut |tokens: &[_]| {
            total_tokens += tokens.len();
        });
        let elapsed = start.elapsed().as_secs_f64();
        let throughput_gb = size_gb / elapsed;
        eprintln!(
            "pass {pass} (cache {}): {total_tokens} tokens in {elapsed:.2}s — {throughput_gb:.2} GB/s ({:.0} MB/s), {} cached units",
            if pass == 1 { "cold" } else { "warm" },
            throughput_gb * 1000.0,
            state.cache_size()
        );
    }
}
