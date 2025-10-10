use crate::{find_comment::FindComment, span::Span};

#[derive(Debug)]
pub struct Metadata<'a> {
    pub(crate) lines: Vec<usize>,
    pub(crate) cmnts: Vec<Span<(&'a str, bool)>>,
}

impl Metadata<'_> {
    pub fn line_index(&self, index: usize) -> usize {
        match self.lines.binary_search(&(index + 1)) {
            Ok(v) => v,
            Err(v) => v,
        }
    }

    pub fn find_comment(&self, index: usize) -> Option<Span<(&'_ str, bool)>> {
        self.cmnts.find_comment(index)
    }

    pub fn find_comment_by_line(&self, index: usize) -> Option<Span<(&'_ str, bool)>> {
        match self.lines.get(index) {
            Some(v) => self.find_comment(v.wrapping_sub(1)),
            _ => None,
        }
    }
}
