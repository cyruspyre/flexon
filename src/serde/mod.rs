//! serde specific API.

pub mod de;
mod unchecked;
mod value;

#[cfg(feature = "std")]
pub mod format;
#[cfg(feature = "std")]
pub mod ser;
#[cfg(feature = "span")]
mod span;

#[doc(inline)]
pub use de::{
    from_mut_slice, from_mut_slice_unchecked, from_mut_str, from_slice, from_slice_unchecked,
    from_str, get_from, get_from_unchecked, get_with_parser, get_with_parser_unchecked,
};

#[doc(inline)]
#[cfg(feature = "alloc")]
pub use de::{from_mut_null_padded, from_null_padded};

#[doc(inline)]
#[cfg(feature = "std")]
pub use {
    de::{from_reader, from_reader_unchecked},
    ser::{to_string, to_string_pretty, to_vec, to_vec_pretty, to_writer, to_writer_pretty},
};
