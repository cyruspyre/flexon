#[derive(Debug)]
pub enum Error {
    Eof,
    Expected,
    Unexpected,
    TrailingDecimal,
    InvalidEscapeSequnce,
    TrailingComma,
}
