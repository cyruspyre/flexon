//! # flexon
//!
//! A JSON parser with span tracking and optional comment support.
//!
//! ## Example
//!
//! ```rust
//! use flexon::parse;
//!
//! let src = r#"{"blah": "was it really necessary?"}"#;
//! let val = &parse(src).unwrap()["blah"];
//!
//! println!("{val:#?} at {}..={}", val.start(), val.end());
//! ```
//!
//! For more examples and performance benchmarks, see the [README](https://crates.io/crates/flexon).
//!
//! ## Features
//!
//! - **`comment`** - Enable comment parsing. The performance overhead is significant
//! - **`line-count`** - Include line information and allow searching comments by line index. Performance overhead is somewhat minimal
//! - **`prealloc`** *(default)* - Pre-allocate memory when parsing for faster performance, with possible memory trade-offs
//! - **`span`** - Include span information on the parsed JSON data. Performance overhead is minimal and memory usage will increase roughly by 33%

pub use crate::{error::Error, parser::Parser, value::Value};

#[cfg(all(feature = "comment", feature = "span"))]
pub use find_comment::FindComment;

#[cfg(feature = "span")]
pub use crate::span::Span;

#[cfg(feature = "line-count")]
mod metadata;

#[cfg(all(feature = "comment", feature = "span"))]
mod find_comment;

#[cfg(feature = "span")]
mod span;

mod error;
mod misc;
mod parser;
pub mod value;

#[cfg(feature = "span")]
type Wrap<T> = Span<T>;

#[cfg(not(feature = "span"))]
type Wrap<T> = T;

#[cfg(feature = "span")]
macro_rules! wrap {
    ($t:ty) => {
        Span<$t>
    };
}

#[cfg(not(feature = "span"))]
macro_rules! wrap {
    ($t:ty) => {
        $t
    };
}

pub(crate) use wrap;

/// Parses JSON with custom parsing options.
///
/// # Arguments
///
/// - `src`: The JSON source to parse.
/// - `comma`: Whether to require commas while parsing.
/// - `trailing_comma`: Whether trailing commas are allowed (has no effect when commas are optional).
///
/// # Example
///
/// ```rust
/// use flexon::parse_with;
///
/// let src = r#"{"numbers": [1, 2,]}"#;
/// let value = parse_with(src, true, true).unwrap();
/// ```
pub fn parse_with(
    src: &str,
    comma: bool,
    trailing_comma: bool,
) -> Result<wrap!(Value), wrap!(Error)> {
    Parser::new(src, comma, trailing_comma).parse().map(|v| {
        #[cfg(feature = "comment")]
        return v.0;

        #[cfg(not(feature = "comment"))]
        v
    })
}

/// Parses JSON with default parsing options.
///
/// This is equivalent to calling `flexon::parse_with(src, true, false)`.
///
/// # Example
///
/// ```rust
/// use flexon::parse;
///
/// let src = r#"{"message": "Hello, world!"}"#;
/// let value = parse(src).unwrap();
/// ```
pub fn parse(src: &str) -> Result<wrap!(Value), wrap!(Error)> {
    parse_with(src, true, false)
}
