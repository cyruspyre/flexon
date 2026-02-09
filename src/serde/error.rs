//! serde specific errors.

use core::fmt::{Display, Formatter, Result};

use serde::de;

/// Represents error occurred while parsing.
#[derive(Debug)]
pub struct Error(Box<Kind>);

/// Represents the type of error.
#[derive(Debug, PartialEq)]
pub enum Kind {
    /// Serde specific error.
    Message(Box<str>),

    /// Unexpected EOF while parsing.
    Eof,
    /// Expected colon while parsing object.
    ExpectedColon,
    /// Unexpected token while parsing.
    UnexpectedToken,
    /// String wasn't properly terminated.
    UnclosedString,
    /// Found raw control characters inside string while parsing.
    ControlCharacter,
    /// Invalid escape sequence in string.
    InvalidEscapeSequnce,
    /// Invalid JSON literal.
    InvalidLiteral,
    /// Comma after the last value of an array or an object.
    TrailingComma,
    /// Number starting with a decimal point.
    LeadingDecimal,
    /// Number ending with a decimal point.
    TrailingDecimal,
    /// Number starting with zero.
    LeadingZero,
    /// Number is bigger than it can represent.
    NumberOverflow,
}

impl Error {
    /// Returns the error kind.
    pub fn kind(&self) -> &Kind {
        &self.0
    }
}

impl Into<Error> for Kind {
    #[cold]
    fn into(self) -> Error {
        Error(Box::new(self))
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str(match &*self.0 {
            Kind::Message(data) => data,
            Kind::Eof => "eof while parsing",
            Kind::ExpectedColon => "expected colon",
            Kind::UnexpectedToken => "unexpected token",
            Kind::UnclosedString => "unclosed string",
            Kind::ControlCharacter => "control character inside string",
            Kind::InvalidEscapeSequnce => "invalid escape sequence",
            Kind::InvalidLiteral => "invalid literal",
            Kind::TrailingComma => "trailing comma",
            Kind::LeadingDecimal => "leading decimal in number",
            Kind::TrailingDecimal => "trailing decimal in number",
            Kind::LeadingZero => "leading zero in number",
            Kind::NumberOverflow => "number too large",
        })
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error(Box::new(Kind::Message(msg.to_string().into_boxed_str())))
    }
}

impl core::error::Error for Error {}
