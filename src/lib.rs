pub use crate::{error::Error, parser::Parser};

use crate::{span::Span, value::Value};

#[cfg(feature = "metadata")]
mod metadata;

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
