use crate::input::Document;

struct JsonLinesIter<'a> {
    slice: &'a [u8],
    position: usize,
}

/// Iterate documents in a .jsonl file
impl<'a> Iterator for JsonLinesIter<'a> {
    type Item = Document<'static>; // Will always be owned
    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.slice.len() {
            return None;
        }
        let next_newline = self.slice[self.position..]
            .iter()
            .position(|&b| b == b'\n')?;
        let line = &self.slice[self.position..self.position + next_newline];
        // Parse JSON
        todo!()
    }
}

struct JsonLinesSource<R> {
    reader: R,
    position: usize,
}

impl<R> JsonLinesSource<R> where R: std::io::BufRead {
    
}
