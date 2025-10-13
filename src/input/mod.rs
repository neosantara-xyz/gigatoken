// Different ways to construct (parallel) document iterators from file or Python input
use memmap2::Mmap;
use rayon::prelude::*;
use std::{borrow::Cow, error::Error, path::Path};
mod bytes;
mod jsonl;
mod py;

#[derive(Debug, Clone)]
pub(crate) struct Document<'a>(Cow<'a, [u8]>);

impl<'a> std::ops::Deref for Document<'a> {
    type Target = Cow<'a, [u8]>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub(crate) enum DocReader {
    FromFiles,
    PythonBytes,
}

pub(crate) enum InputData {
    MmapFile(Mmap),
    PythonBytes,
}

pub(crate) fn read_file(path: impl AsRef<Path>) -> Result<Mmap, String> {
    let path = path.as_ref();
    match path.extension().and_then(|e| e.to_str()) {
        Some("jsonl") => todo!(),
        Some("parquet") => todo!(),
        Some("txt") | None => {
            // TODO: Make this a one-time warning
            eprintln!("Path {path:?} is being treated as a UTF-8 blob.")
        }
        Some(x) => {
            eprintln!(
                "Path {path:?} has extension {x} which was not recognized. Falling back to reading it as a blob of UTF-8."
            )
        }
    }
    let file = std::fs::File::open(path).map_err(|e| format!("{e}"))?;
    unsafe { Mmap::map(&file) }.map_err(|e| format!("{e}"))
}

/// Memmap each file
pub fn iterate_files(
    path_iterator: impl ParallelIterator<Item = impl AsRef<Path>>,
) -> impl ParallelIterator<Item = Result<impl AsRef<[u8]>, String>> {
    path_iterator
        .map(|path| {
            let file = std::fs::File::open(path.as_ref())?;
            unsafe { Mmap::map(&file) }
        })
        .map(|r| r.map_err(|e| format!("Failed to open file: {e}")))
}
