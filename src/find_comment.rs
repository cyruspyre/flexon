use std::borrow::Cow;

use crate::{Span, misc::Sealed};

/// A trait for finding comment by byte index.
///
/// This is a sealed trait, only to be implemented by `flexon`.
pub trait FindComment: Sealed {
    /// Finds the comment that contains the given index, if any.
    fn find_comment(&self, index: usize) -> Option<&Span<(Cow<'_, str>, bool)>>;
}

impl Sealed for Vec<Span<(Cow<'_, str>, bool)>> {}

impl FindComment for Vec<Span<(Cow<'_, str>, bool)>> {
    fn find_comment(&self, index: usize) -> Option<&Span<(Cow<'_, str>, bool)>> {
        let idx = match self.binary_search_by(|v| v.start.cmp(&index)) {
            Ok(v) => v,
            Err(v) => v.wrapping_sub(1),
        };
        let Some(cmnt) = self.get(idx) else {
            return None;
        };

        if index < cmnt.start || index > cmnt.end {
            return None;
        }

        Some(cmnt)
    }
}
