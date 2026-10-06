//! Source and global line index shared by all reparsing within one document.
use super::model::BsolSpan;
use std::ops::Deref;

pub(super) struct ParseSource<'a> {
    source: &'a str,
    line_starts: Vec<usize>,
}
impl<'a> ParseSource<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(
            source
                .bytes()
                .enumerate()
                .filter_map(|(offset, byte)| (byte == b'\n').then_some(offset + 1)),
        );
        Self {
            source,
            line_starts,
        }
    }
    pub fn span(&self, start: usize, end: usize) -> BsolSpan {
        let mut start = start.min(self.source.len());
        while !self.source.is_char_boundary(start) {
            start -= 1;
        }
        let mut end = end.max(start.saturating_add(1)).min(self.source.len());
        while !self.source.is_char_boundary(end) {
            end += 1;
        }
        BsolSpan {
            start,
            end,
            line: self.line_starts.partition_point(|offset| *offset <= start),
        }
    }
}
impl Deref for ParseSource<'_> {
    type Target = str;
    fn deref(&self) -> &str {
        self.source
    }
}
