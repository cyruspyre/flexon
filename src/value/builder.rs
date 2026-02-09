//! Value builder traits for JSON parsing.

use crate::source::Source;

/// Trait for building JSON values during parsing.
pub trait ValueBuilder<'a, S: Source>: Sized {
    /// Whether the value that it is building is lazy.
    ///
    /// If `true` the parser won't call any of the building methods except for [`ValueBuilder::raw`] at the end of parsing.
    /// This allows the parser to skip *efficiently* while also validating the JSON. As of right now, it will ignore the
    /// following flags [`CUSTOM_LITERAL`](Self::CUSTOM_LITERAL), [`REJECT_CTRL_CHAR`](StringBuilder::REJECT_CTRL_CHAR)
    /// and [`REJECT_INVALID_ESCAPE`](StringBuilder::REJECT_INVALID_ESCAPE). [`ValueBuilder::apply_span`] won't be called either.
    ///
    /// Requires the source to be non volatile.
    const LAZY: bool;

    /// Whether the value is to parse literals by itself.
    ///
    /// If `true` the parser will not parse the literals but will call [`ValueBuilder::literal`] to parse them.
    /// This excludes string literals.
    const CUSTOM_LITERAL: bool;

    /// The error type used by this builder.
    type Error: ErrorBuilder;

    /// The array type used by this builder.
    type Array: ArrayBuilder<Self> + Into<Self>;

    /// The object type used by this builder.
    type Object: ObjectBuilder<'a, S, Self::Error> + Into<Self>;

    /// The string type used by this builder.
    type String: StringBuilder<'a, S, Self::Error> + Into<Self>;

    /// Creates a value by the given byte slice.
    ///
    /// Called when [`CUSTOM_LITERAL`](Self::CUSTOM_LITERAL) is `true`. A literal can be made of anything that
    /// isn't either of the following sequentially:
    ///
    /// `'{'`, `'}'`, `'['`, `']'`, `'"'`, `':'`, `','`, `' '`, `'\t'`, `'\n'`, `'\r'`, `'\0'`
    ///
    /// As an additional note, the literals may also include non UTF-8 bytes.
    /// UTF-8 validation is performed only at string types.
    fn literal(s: &'a [u8]) -> Result<Self, Self::Error>;

    /// Creates a value by the given integer value.
    fn integer(val: u64, neg: bool) -> Self;

    /// Creates a value by the given floating point value.
    fn float(val: f64) -> Self;

    /// Creates a value by the given boolean value.
    fn bool(val: bool) -> Self;

    /// Creates a value in the context of null value in JSON.
    fn null() -> Self;

    /// Creates a value by the given raw JSON byte slice.
    ///
    /// Only called once at the end when [`LAZY`](Self::LAZY) is `true`. All the other building methods won't be called.
    fn raw(s: &'a [u8]) -> Self;

    /// Applies span information. The given offsets will be byte offsets.
    fn apply_span(&mut self, start: usize, end: usize);
}

/// Trait for building JSON array during parsing.
pub trait ArrayBuilder<V> {
    /// Create a new array builder.
    fn new() -> Self;

    /// Create a new array builder with the specified capacity hint.
    fn with_capacity(cap: usize) -> Self;

    /// Returns the number of elements in the array.
    ///
    /// Only called at the end of array parsing.
    fn len(&self) -> usize;

    /// Adds a value to the array.
    fn on_value(&mut self, val: V);

    /// Called when array parsing completes, e.g., for sorting.
    fn on_complete(&mut self);
}

/// Trait for building JSON object during parsing.
pub trait ObjectBuilder<'a, S: Source, E: ErrorBuilder> {
    /// The key type used by this builder.
    type Key: StringBuilder<'a, S, E>;

    /// The value type used by this builder.
    type Value: ValueBuilder<'a, S, Error = E>;

    /// Create a new object builder.
    fn new() -> Self;

    /// Create a new object builder with the specified capacity hint.
    fn with_capacity(cap: usize) -> Self;

    /// Returns the number of elements in the object.
    ///
    /// Only called at the end of object parsing.
    fn len(&self) -> usize;

    /// Adds a key-value to the object.
    fn on_value(&mut self, key: Self::Key, val: Self::Value);

    /// Called when object parsing completes, e.g., for sorting.
    fn on_complete(&mut self);
}

/// Trait for building JSON string during parsing.
pub trait StringBuilder<'a, S: Source, E: ErrorBuilder> {
    /// Whether to reject unescaped ascii control characters.
    const REJECT_CTRL_CHAR: bool;

    /// Whether to reject invalid escape sequences.
    const REJECT_INVALID_ESCAPE: bool;

    /// Create a new string builder.
    fn new() -> Self;

    /// Called when the parser encounters escape sequence. The given byte slice will be the parsed escape
    /// sequence and the length will be less than or equal to 4.
    fn on_escape(&mut self, s: &[u8]);

    /// Called before calling [`StringBuilder::on_escape`].
    ///
    /// This is the byte slice before escape sequence excluding `'\'` at the end.
    fn on_chunk(&mut self, s: &'a [u8]);

    /// Called at the final remaining byte slice.
    fn on_final_chunk(&mut self, s: &'a [u8]);

    /// Applies span information. The given offsets will be byte offsets.
    fn apply_span(&mut self, start: usize, end: usize);

    /// This function will always be called at the end of parsing string.
    /// The given byte slice is the whole string excluding the surrounding quotes.
    fn on_complete(&mut self, s: &'a [u8]) -> Result<(), E>;
}

/// Trait for building errors during parsing.
pub trait ErrorBuilder {
    /// Unexpected EOF while parsing.
    fn eof() -> Self;

    /// Expected colon while parsing object.
    fn expected_colon() -> Self;

    /// Expected value but found EOF instead.
    fn expected_value() -> Self;

    /// Comma after the last element of an array or an object.
    fn trailing_comma() -> Self;

    /// String wasn't properly terminated.
    fn unclosed_string() -> Self;

    /// Invalid escape sequence in string.
    fn invalid_escape() -> Self;

    /// Found raw control characters inside string while parsing.
    fn control_character() -> Self;

    /// Invalid JSON literal.
    fn invalid_literal() -> Self;

    /// Number ending with a decimal point.
    fn trailing_decimal() -> Self;

    /// Number starting with a decimal point.
    fn leading_decimal() -> Self;

    /// Number starting with zero.
    fn leading_zero() -> Self;

    /// Number is bigger than it can represent.
    fn number_overflow() -> Self;

    /// Unexpected token while parsing.
    fn unexpected_token() -> Self;

    /// Applies span information. The given offsets will be byte offsets.
    fn apply_span(&mut self, start: usize, end: usize);
}
