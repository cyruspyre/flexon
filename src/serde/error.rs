//! serde specific errors.

use core::fmt::{Display, Formatter, Result};

use serde::de;

/// Represents error occurred while parsing.
#[derive(Debug)]
pub struct Error {
    pub(super) kind: Box<Kind>,
    #[cfg(feature = "span")]
    pub(crate) span: [usize; 2],
}

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
    #[inline]
    pub fn kind(&self) -> &Kind {
        &self.kind
    }

    /// Returns the starting byte offset of the error.
    #[inline]
    #[cfg(feature = "span")]
    pub fn start(&self) -> usize {
        self.span[0]
    }

    /// Returns the ending byte offset of the error.
    #[inline]
    #[cfg(feature = "span")]
    pub fn end(&self) -> usize {
        self.span[1]
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str(match &*self.kind {
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
        Error {
            kind: Box::new(Kind::Message(msg.to_string().into_boxed_str())),
            #[cfg(feature = "span")]
            span: [0; 2],
        }
    }
}

impl core::error::Error for Error {}
