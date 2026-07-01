#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "nightly", feature(likely_unlikely))]
#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(unsafe_op_in_unsafe_fn)]
#![deny(clippy::std_instead_of_core)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod config;
mod error;
mod fast_float;
mod misc;
mod parser;
mod simd;
pub mod source;
mod utf8;
pub mod value;

#[cfg(feature = "comment")]
mod comment;
pub mod pointer;
#[cfg(feature = "serde")]
pub mod serde;
#[cfg(feature = "span")]
pub mod span;

use crate::{pointer::JsonPointer, source::Source, value::builder::ValueBuilder};

#[doc(inline)]
pub use {error::Error, parser::Parser};

#[doc(inline)]
#[cfg(feature = "alloc")]
pub use value::{LazyValue, OwnedValue, Value};

#[doc(inline)]
#[cfg(feature = "serde")]
pub use serde::de::{
    from_mut_slice, from_mut_slice_unchecked, from_mut_str, from_slice, from_slice_unchecked,
    from_str,
};

#[doc(inline)]
#[cfg(all(feature = "serde", feature = "alloc"))]
pub use serde::de::{from_mut_null_padded, from_null_padded, get_from, get_from_unchecked};

#[doc(inline)]
#[cfg(all(feature = "serde", feature = "std"))]
pub use serde::{
    de::{from_reader, from_reader_unchecked},
    ser::{to_string, to_string_pretty, to_vec, to_vec_pretty, to_writer, to_writer_pretty},
};

#[cfg(feature = "comment")]
pub use comment::Comment;

/// Parses a JSON source into the specified type.
///
/// This is a convenience function that creates a parser with default configuration
/// and immediately parses the input. It is equivalent to `Parser::new(src).parse()`.
///
/// # Example
/// ```
/// use flexon::{Value, parse};
///
/// let json = r#"{"width": 20, "height": 50}"#;
/// let value: Value<'_> = parse(json)?;
/// # Ok::<(), flexon::Error>(())
/// ```
#[inline]
pub fn parse<'a, S: Source + 'a, V: ValueBuilder<'a, S>>(s: S) -> Result<V, V::Error> {
    Parser::new(s).parse()
}

/// Skips to the given path and parses JSON into the specified type.
///
/// This is a convenience function that creates a parser with default configuration
/// and immediately parses the input. It is equivalent to `Parser::new(src).parse_at(path)`.
///
/// # Example
/// ```
/// use flexon::{Value, parse_at};
///
/// let json = r#"{"width": 20, "height": 50}"#;
/// let value: Value<'_> = parse_at(json, ["height"])?;
/// # Ok::<(), flexon::Error>(())
/// ```
#[inline]
pub fn parse_at<'a, S, V, P>(s: S, p: P) -> Result<V, V::Error>
where
    S: Source + 'a,
    V: ValueBuilder<'a, S>,
    P: IntoIterator,
    P::Item: JsonPointer,
{
    Parser::new(s).parse_at(p)
}
