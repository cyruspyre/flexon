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

pub fn parse_with(
    src: &str,
    comma: bool,
    trailing_comma: bool,
) -> Result<Wrap<Value>, Wrap<Error>> {
    Parser::new(src, comma, trailing_comma).parse().map(|v| {
        #[cfg(feature = "comment")]
        return v.0;

        #[cfg(not(feature = "comment"))]
        v
    })
}

pub fn parse(src: &str) -> Result<Wrap<Value>, Wrap<Error>> {
    parse_with(src, true, false)
}
