/// Represents the type of error that occurred while parsing.
#[derive(Debug)]
pub enum Error {
    /// Expected the given character but found something else.
    Expected(char),
    /// Expected value but found EOF instead.
    ExpectedValue,
    /// Exponent in number not followed by a valid digit.
    ExpectedExponentValue,
    /// Unexpected token while parsing.
    UnexpectedToken,
    /// String wasn't properly terminated.
    UnclosedString,
    /// Found raw control characters inside string while parsing.
    ControlCharacter,
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
