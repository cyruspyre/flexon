/// Represents the type of error that occurred while parsing.
#[derive(Debug)]
pub enum Error {
    /// Unexpected EOF while parsing.
    Eof,

    /// Expected colon while parsing object.
    ExpectedColon,

    /// Expected value but found EOF instead.
    ExpectedValue,

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

    /// Comma after the last element of an array or an object.
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

impl crate::value::builder::ErrorBuilder for Error {
    #[inline]
    fn eof() -> Self {
        Self::Eof
    }

    #[inline]
    fn expected_colon() -> Self {
        Self::ExpectedColon
    }

    #[inline]
    fn expected_value() -> Self {
        Self::ExpectedValue
    }

    #[inline]
    fn trailing_comma() -> Self {
        Self::TrailingComma
    }

    #[inline]
    fn unclosed_string() -> Self {
        Self::UnclosedString
    }

    #[inline]
    fn invalid_escape() -> Self {
        Self::InvalidEscapeSequnce
    }

    #[inline]
    fn control_character() -> Self {
        Self::ControlCharacter
    }

    #[inline]
    fn invalid_literal() -> Self {
        Self::InvalidLiteral
    }

    #[inline]
    fn trailing_decimal() -> Self {
        Self::TrailingDecimal
    }

    #[inline]
    fn leading_decimal() -> Self {
        Self::LeadingDecimal
    }

    #[inline]
    fn leading_zero() -> Self {
        Self::LeadingZero
    }

    #[inline]
    fn number_overflow() -> Self {
        Self::NumberOverflow
    }

    #[inline]
    fn unexpected_token() -> Self {
        Self::UnexpectedToken
    }

    #[inline]
    fn apply_span(&mut self, _: usize, _: usize) {}
}
