pub use crate::{error::Error, parser::Parser, span::Span};
#[cfg(all(feature = "comment", not(feature = "line-count")))]
pub use find_comment::FindComment;

use crate::value::Value;

#[cfg(feature = "line-count")]
mod metadata;

#[cfg(feature = "comment")]
mod find_comment;

mod error;
mod misc;
mod parser;
mod span;
pub mod value;

pub fn parse_with(
    src: &str,
    comma: bool,
    trailing_comma: bool,
) -> Result<Span<Value>, Span<Error>> {
    Parser::new(src, comma, trailing_comma).parse().map(|v| {
        #[cfg(feature = "comment")]
        return v.0;

        #[cfg(not(feature = "comment"))]
        v
    })
}

pub fn parse(src: &str) -> Result<Span<Value>, Span<Error>> {
    parse_with(src, true, false)
}
