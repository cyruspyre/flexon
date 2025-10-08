pub use crate::{error::Error, parser::Parser};

use crate::{span::Span, value::Value};

mod error;
mod metadata;
mod misc;
mod parser;
mod span;
pub mod value;

pub fn parse_with(src: &str, comma: bool, trailing_comma: bool) -> Result<Span<Value>, Error> {
    Parser::new(src, comma, trailing_comma).parse().map(|v| v.0)
}

pub fn parse(src: &str) -> Result<Span<Value>, Error> {
    parse_with(src, true, false)
}
