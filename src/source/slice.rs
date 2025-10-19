use std::ops::{Bound, RangeBounds};

use crate::source::{Source, memchr_n};

/// A [`Source`] implementation that wraps a byte slice.
///
/// This is the preferred source for parsing JSON.
pub struct Slice<'a>(&'a [u8]);

impl<'a> Source for Slice<'_> {
    const BORROWED: bool = true;

    #[inline(always)]
    fn get(&self, index: usize) -> u8 {
        self.0[index]
    }

    #[inline(always)]
    fn get_checked(&mut self, index: usize) -> u8 {
        match self.0.get(index) {
            Some(v) => *v,
            _ => 0,
        }
    }

    #[inline]
    fn slice<R: RangeBounds<usize>>(&self, range: R) -> &[u8] {
        let start = match range.start_bound() {
            Bound::Included(&i) => i,
            _ => unreachable!(),
        };
        let end = match range.end_bound() {
            Bound::Included(&i) => i + 1,
            Bound::Excluded(&i) => i,
            _ => unreachable!(),
        };

        &self.0[start..end]
    }

    #[inline(always)]
    fn unbounded_search<const N: usize>(&mut self, needles: [u8; N], index: usize) -> usize {
        match memchr_n!(N, needles, &self.0[index..]) {
            Some(v) => index + v,
            _ => 0,
        }
    }

    #[inline(always)]
    fn stamp(&mut self, _: usize) {}

    #[inline(always)]
    fn hint(&mut self, _: usize) {}

    #[inline(always)]
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a, S: AsRef<str> + ?Sized> From<&'a S> for Slice<'a> {
    fn from(value: &'a S) -> Self {
        Slice(value.as_ref().as_bytes())
    }
}
