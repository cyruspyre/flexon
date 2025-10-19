mod reader;
mod slice;

use std::ops::RangeBounds;

pub use reader::Reader;
pub use slice::Slice;

#[allow(unused_imports)]
use std::borrow::Cow;

/// A trait representing a generic source of bytes for parsing.
pub trait Source {
    /// Indicates whether comments returned by the parser can be borrowed (`true`) or must be owned (`false`).
    ///
    /// - If `true`, comment text is returned as [`Cow::Borrowed`] pointing directly into the source slice,
    ///   avoiding allocations.
    /// - If `false`, comment text is returned as [`Cow::Owned`], since the source may not live long enough
    ///   for borrowing.
    ///
    /// For example:
    /// - [`Slice<'a>`] sets `BORROWED = true` because comments can reference the input slice directly.
    /// - [`Reader<R>`] sets `BORROWED = false` because comments must be copied from the streaming buffer.
    const BORROWED: bool;

    /// Returns the byte at the specified index without performing bounds checking.
    ///
    /// This method is used more frequently by the parser than [`Source::get_checked`].
    /// It assumes that the given index is always valid and will not trigger any reads or errors.
    fn get(&self, index: usize) -> u8;

    /// Returns the byte at the given `index`, returning `0` if out of bounds.
    ///
    /// For streaming sources, this may trigger a read to fetch more data.
    fn get_checked(&mut self, index: usize) -> u8;

    /// Returns a slice of bytes within the specified `range`.
    ///
    /// This method should never panic. The range will always be inclusive or exclusive
    /// and will not be unbounded.
    fn slice<R: RangeBounds<usize>>(&self, range: R) -> &[u8];

    /// Searches for the first occurrence of either of the given bytes starting from `index`.
    ///
    /// Returns the index of the match, or `0` if no match is found.
    /// For streaming sources, this may read additional data as needed.
    fn unbounded_search<const N: usize>(&mut self, needles: [u8; N], index: usize) -> usize;

    /// Marks a "stamp" at the given index, indicating that bytes before this index
    /// may be discarded or ignored.
    ///
    /// This is primarily used to optimize memory usage for sources like `Reader`.
    /// It can be safely ignored for sources that are fully loaded in memory, like `Slice`.
    fn stamp(&mut self, stamp: usize);

    /// Provides a hint that the parser will soon require data up to the given index.
    ///
    /// Streaming sources may use this to prefetch or buffer additional bytes.
    /// Otherwise, the parser may return an EOF error. This can be safely ignored for
    /// sources like `Slice`.
    fn hint(&mut self, index: usize);

    /// Returns the total length of the source.
    ///
    /// For fixed sources like slices, this is the slice length.
    /// For streaming sources, this is the total number of bytes read so far.
    fn len(&self) -> usize;
}

macro_rules! memchr_n {
    ($n:expr, $needles:expr, $haystack:expr) => {
        match $n {
            1 => memchr::memchr($needles[0], $haystack),
            2 => memchr::memchr2($needles[0], $needles[1], $haystack),
            _ => memchr::memchr3($needles[0], $needles[1], $needles[2], $haystack),
        }
    };
}

pub(crate) use memchr_n;
