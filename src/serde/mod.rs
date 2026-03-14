//! serde specific API.

#[cfg(feature = "span")]
mod span;

mod borrowed;
pub mod de;
pub mod format;
mod number;
pub mod ser;
mod unchecked;

#[doc(inline)]
pub use {
    de::{
        from_mut_null_padded, from_mut_slice, from_mut_slice_unchecked, from_mut_str,
        from_null_padded, from_reader, from_reader_unchecked, from_slice, from_slice_unchecked,
        from_str, get_from, get_from_unchecked, get_with_parser, get_with_parser_unchecked,
    },
    ser::{to_string, to_string_pretty, to_vec, to_vec_pretty, to_writer, to_writer_pretty},
};
