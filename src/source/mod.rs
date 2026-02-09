//! JSON source types and traits for parsing.

mod null_padded;
mod reader;

use crate::misc::Sealed;

pub use {null_padded::NullPadded, reader::Reader};

/// Trait representing a source of JSON data for parsing.
///
/// This trait abstracts over different input types, allowing the parser to work
/// with strings, byte slices, and streaming readers uniformly.
pub trait Source {
    /// Whether the source is guaranteed to be valid UTF-8.
    ///
    /// When `false`, the parser will validate UTF-8 encoding for JSON string values.
    ///
    /// Note: Validation is limited to strings only, as invalid UTF-8 elsewhere in the
    /// JSON will be rejected as an unexpected token during parsing.
    const UTF8: bool;

    /// Whether the source allows in-situ (in-place) parsing.
    ///
    /// In-situ parsing modifies the source data directly to parse strings. Requires the source to be non volatile as well.
    const INSITU: bool;

    /// Whether the source is 64 null bytes padded at the end. Allows the parser to skip bounds checking.
    ///
    /// # Safety
    ///
    /// The source must be non volatile. As the parser will store and use the pointer as index.
    const NULL_PADDED: bool;

    /// The volatility of the source.
    type Volatility: Volatility;

    /// Returns a pointer at the given offset.
    ///
    /// # Safety
    ///
    /// The parser ensures the offset is within bounds by checking `len()` beforehand.
    /// Implementations must not panic.
    fn ptr(&mut self, offset: usize) -> *const u8;

    /// Returns a mutable pointer to the byte at the given offset.
    ///
    /// This method is used for in-situ parsing. Implementations may return a null pointer
    /// or use `unimplemented!()`/`unreachable()` if `INSITU` is `false`. The parser only calls this
    /// method when in-situ parsing conditions are met.
    ///
    /// # Safety
    ///
    /// The parser ensures the offset is within bounds by checking `len()` beforehand.
    /// Implementations must not panic.
    fn ptr_mut(&mut self, offset: usize) -> *mut u8;

    /// Signal to discard data up to the given offset (exclusive).
    ///
    /// This method is only called for volatile sources to free memory as data is consumed.
    fn trim(&mut self, until: usize);

    /// Returns the current length of available source data in bytes.
    ///
    /// The parser requires at least 64 bytes of data to be available during parsing,
    /// otherwise it may give false EOF-related errors.
    fn len(&mut self) -> usize;
}

/// Marker trait indicating whether a source's data is stable or not.
///
/// Sources can be either volatile or non-volatile:
/// - **Non-volatile**: The entire input is available upfront (e.g., string slices)
/// - **Volatile**: The input arrives incrementally and may need trimming (e.g., readers)
pub trait Volatility: Sealed {
    #[doc(hidden)]
    const IS_VOLATILE: bool;
}

/// Marker type for sources whose data may change or be discarded during parsing.
///
/// Volatile sources typically represent streaming input where data arrives
/// incrementally and already-parsed portions may be trimmed to conserve memory.
pub struct Volatile;

impl Sealed for Volatile {}
impl Volatility for Volatile {
    const IS_VOLATILE: bool = true;
}

/// Marker type for sources whose data remains stable throughout parsing.
///
/// Non-volatile sources have all data available upfront and it remains
/// accessible for the entire duration of parsing.
pub struct NonVolatile;

impl Sealed for NonVolatile {}
impl Volatility for NonVolatile {
    const IS_VOLATILE: bool = false;
}

impl Source for &str {
    const UTF8: bool = true;
    const INSITU: bool = false;
    const NULL_PADDED: bool = false;

    type Volatility = NonVolatile;

    #[inline(always)]
    fn ptr(&mut self, offset: usize) -> *const u8 {
        unsafe { self.as_ptr().add(offset) }
    }

    #[inline(always)]
    fn ptr_mut(&mut self, _: usize) -> *mut u8 {
        unimplemented!()
    }

    #[inline(always)]
    fn trim(&mut self, _: usize) {}

    #[inline(always)]
    fn len(&mut self) -> usize {
        (**self).len()
    }
}

impl Source for &mut str {
    const UTF8: bool = true;
    const INSITU: bool = true;
    const NULL_PADDED: bool = false;

    type Volatility = NonVolatile;

    #[inline(always)]
    fn ptr(&mut self, offset: usize) -> *const u8 {
        unsafe { self.as_ptr().add(offset) }
    }

    #[inline(always)]
    fn ptr_mut(&mut self, offset: usize) -> *mut u8 {
        unsafe { self.as_mut_ptr().add(offset) }
    }

    #[inline(always)]
    fn trim(&mut self, _: usize) {}

    #[inline(always)]
    fn len(&mut self) -> usize {
        (**self).len()
    }
}
