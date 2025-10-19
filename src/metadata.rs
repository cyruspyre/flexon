use std::borrow::Cow;

use crate::{find_comment::FindComment, misc::Sealed, span::Span};

/// Metadata about the source code, including its line offsets and comments.
#[derive(Debug)]
pub struct Metadata<'a> {
    pub(crate) lines: Vec<usize>,
    pub(crate) cmnts: Vec<Span<(Cow<'a, str>, bool)>>,
}

impl Metadata<'_> {
    /// Returns the line index to which the given index falls in.
    ///
    /// The parser treats end of file as a new line.
    /// So, If the index is greater than or equal to the source length,
    /// it is treated as if belonging to a virtual line after the last line.
    pub fn line_index(&self, index: usize) -> usize {
        match self.lines.binary_search(&(index + 1)) {
            Ok(v) => v,
            Err(v) => v,
        }
    }

    /// Finds comment for the given line index, if any.
    pub fn find_comment_by_line(&self, index: usize) -> Option<&Span<(Cow<'_, str>, bool)>> {
        match self.lines.get(index) {
            Some(v) => self.find_comment(v.wrapping_sub(1)),
            _ => None,
        }
    }
}

impl Sealed for Metadata<'_> {}

impl FindComment for Metadata<'_> {
    fn find_comment(&self, index: usize) -> Option<&Span<(Cow<'_, str>, bool)>> {
        self.cmnts.find_comment(index)
    }
}
