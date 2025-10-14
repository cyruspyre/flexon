/// Represents the type of error that occurred while parsing.
#[derive(Debug)]
pub enum Error {
    /// EOF while parsing.
    Eof,
    /// Expected the given character but found something else.
    Expected(char),
    /// Exponent in number not followed by a valid digit.
    ExpectedExponentValue,
    /// Unexpected token while parsing.
    UnexpectedToken,
    /// Invalid escape sequence in string.
    InvalidEscapeSequnce,
    /// Comma after the last value of an array or an object.
    TrailingComma,
    /// Number starting with a decimal point.
    LeadingDecimal,
    /// Number ending with a decimal point.
    TrailingDecimal,
    /// Negative sign of a number not followed by a digit.
    MissingDigitAfterNegative,
    /// Number is bigger than it can represent.
    NumberOverflow,
}
