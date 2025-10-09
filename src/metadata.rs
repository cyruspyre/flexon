use crate::span::Span;

#[derive(Debug)]
pub struct Metadata<'a> {
    pub(crate) lines: Vec<usize>,
    pub(crate) cmnts: Vec<Span<(bool, &'a str)>>,
}

impl Metadata<'_> {
    pub fn line_index(&self, pos: usize) -> usize {
        match self.lines.binary_search(&(pos + 1)) {
            Ok(v) => v,
            Err(v) => v,
        }
    }

    pub fn find_comment(&self, pos: usize) -> Option<Span<(bool, &'_ str)>> {
        let idx = match self.cmnts.binary_search_by(|v| v.start.cmp(&pos)) {
            Ok(v) => v,
            Err(v) => v.wrapping_sub(1),
        };
        let Some(cmnt) = self.cmnts.get(idx) else {
            return None;
        };

        if pos < cmnt.start || pos > cmnt.end {
            return None;
        }

        Some(*cmnt)
    }

    pub fn find_comment_by_line(&self, idx: usize) -> Option<Span<(bool, &'_ str)>> {
        match self.lines.get(idx) {
            Some(v) => self.find_comment(v.wrapping_sub(1)),
            _ => None,
        }
    }
}
