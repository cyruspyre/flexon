//! Span information for JSON values.

#[cfg(feature = "alloc")]
mod value;

use crate::{
    Error,
    source::Source,
    value::builder::{ErrorBuilder, StringBuilder},
};
use core::{
    fmt::{self, Debug},
    ops::Deref,
};

#[cfg(feature = "alloc")]
pub use value::GenericValue;

/// Represents a borrowed JSON value.
#[cfg(feature = "alloc")]
pub type Value<'a> = Span<GenericValue<crate::value::borrowed::String<'a>>>;

/// Represents an owned JSON value.
#[cfg(feature = "alloc")]
pub type OwnedValue = Span<GenericValue<crate::value::owned::String>>;

/// A spanned value with its starting and ending byte offset.
///
/// # Example
/// ```
/// use flexon::span::Span;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Book {
///     name: Span<String>,
///     pages: Span<u32>,
/// }
///
/// let json = r#"{"name": "idk", "pages": 256}"#;
/// let book: Span<Book> = flexon::from_str(json)?;
/// let span: Span<u32> = book.data().pages;
///
/// assert_eq!(span.start(), 25);
/// assert_eq!(span.end(), 27);
///
/// # Ok::<(), flexon::serde::de::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span<T> {
    data: T,
    start: usize,
    end: usize,
}

impl<T> Span<T> {
    #[inline]
    pub(crate) fn new(data: T) -> Self {
        Self {
            data,
            start: 0,
            end: 0,
        }
    }

    #[inline]
    #[cfg(feature = "serde")]
    pub(crate) fn with(data: T, start: usize, end: usize) -> Self {
        Self { data, start, end }
    }

    /// Returns a reference to its value.
    #[inline]
    pub fn data(&self) -> &T {
        &self.data
    }

    #[inline]
    pub fn data_mut(&mut self) -> &mut T {
        &mut self.data
    }

    /// Returns its starting byte index.
    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns its ending byte index.
    #[inline]
    pub fn end(&self) -> usize {
        self.end
    }
}

impl<T: Deref<Target = str>> Deref for Span<T> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T: Debug> Debug for Span<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}..{}] ", self.start, self.end)?;
        self.data.fmt(f)
    }
}

impl<'a, S, V> StringBuilder<'a, S, Span<Error>> for Span<V>
where
    S: Source,
    V: StringBuilder<'a, S, Span<Error>>,
{
    const REJECT_CTRL_CHAR: bool = V::REJECT_CTRL_CHAR;
    const REJECT_INVALID_ESCAPE: bool = V::REJECT_INVALID_ESCAPE;

    #[inline]
    fn new() -> Self {
        Self::new(V::new())
    }

    #[inline]
    unsafe fn on_escape(&mut self, s: &[u8]) {
        self.data.on_escape(s)
    }

    #[inline]
    fn on_chunk(&mut self, s: &'a [u8]) {
        self.data.on_chunk(s)
    }

    #[inline]
    fn on_final_chunk(&mut self, s: &'a [u8]) {
        self.data.on_final_chunk(s)
    }

    #[inline]
    fn apply_span(&mut self, start: usize, end: usize) {
        self.start = start;
        self.end = end;
    }

    #[inline]
    fn on_complete(&mut self, s: &'a [u8]) -> Result<(), Span<Error>> {
        self.data.on_complete(s)
    }
}

impl<E: ErrorBuilder> ErrorBuilder for Span<E> {
    #[inline]
    fn eof() -> Self {
        Self::new(E::eof())
    }

    #[inline]
    fn expected_colon() -> Self {
        Self::new(E::expected_colon())
    }

    #[inline]
    fn expected_value() -> Self {
        Self::new(E::expected_value())
    }

    #[inline]
    fn trailing_comma() -> Self {
        Self::new(E::trailing_comma())
    }

    #[inline]
    fn unclosed_string() -> Self {
        Self::new(E::unclosed_string())
    }

    #[inline]
    fn invalid_escape() -> Self {
        Self::new(E::invalid_escape())
    }

    #[inline]
    fn control_character() -> Self {
        Self::new(E::control_character())
    }

    #[inline]
    fn invalid_literal() -> Self {
        Self::new(E::invalid_literal())
    }

    #[inline]
    fn trailing_decimal() -> Self {
        Self::new(E::trailing_decimal())
    }

    #[inline]
    fn leading_decimal() -> Self {
        Self::new(E::leading_decimal())
    }

    #[inline]
    fn leading_zero() -> Self {
        Self::new(E::leading_zero())
    }

    #[inline]
    fn number_overflow() -> Self {
        Self::new(E::number_overflow())
    }

    #[inline]
    fn unexpected_token() -> Self {
        Self::new(E::unexpected_token())
    }

    #[inline]
    fn apply_span(&mut self, start: usize, end: usize) {
        self.start = start;
        self.end = end;
    }
}
