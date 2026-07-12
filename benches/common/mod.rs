//! Shared input loading for the bench targets. Lives in a subdirectory so
//! cargo does not treat it as a bench target; each bench pulls it in with
//! `mod common;`.

use std::time::Instant;

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
            let file =
                std::fs::File::open(&owt_path).expect("Could not open ~/data/owt_train.txt");
            let mut data = Vec::with_capacity(max_bytes);
            file.take(max_bytes as u64)
                .read_to_end(&mut data)
                .expect("read failed");
            data
        }
        None => std::fs::read(&owt_path).expect("Could not read ~/data/owt_train.txt"),
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
