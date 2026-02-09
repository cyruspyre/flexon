//! Span information for JSON values.

use core::{
    fmt::{self, Debug, Formatter},
    ops::{Deref, Index},
};

use crate::{
    Error,
    pointer::JsonPointer,
    source::Source,
    value::{Array, Number, Object, builder::*},
};

/// A wrapper with the values starting and ending byte offset.
pub struct Span<T> {
    data: T,
    start: usize,
    end: usize,
}

#[doc(hidden)]
pub enum Value<S> {
    /// Represents a JSON null value.
    Null,

    /// Represents a JSON array.
    Array(Array<Span<Self>>),

    /// Represents a JSON object.
    Object(Object<Span<S>, Span<Self>>),

    /// Represents a JSON string.
    String(S),

    /// Represents a JSON number.
    Number(Number),

    /// Represents a JSON boolean.
    Boolean(bool),
}

mod owned {
    use crate::{
        span::{Span, Value},
        value::owned::String,
    };

    /// Represents an owned JSON value.
    pub type OwnedValue = Span<Value<String>>;
}

mod borrowed {
    use crate::{
        span::{Span, Value},
        value::borrowed::String,
    };

    /// Represents a borrowed JSON value.
    pub type BorrowedValue<'a> = Span<Value<String<'a>>>;
}

pub use borrowed::BorrowedValue;
pub use owned::OwnedValue;

impl<T> Span<T> {
    #[inline]
    fn new(data: T) -> Self {
        Self {
            data,
            start: 0,
            end: 0,
        }
    }

    /// Returns a reference to its value.
    #[inline]
    pub fn data(&self) -> &T {
        &self.data
    }

    #[inline]
    pub fn data_mut(&mut self) -> &mut T {
        &mut self.data
    }

    /// Returns its starting byte index.
    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns its ending byte index.
    #[inline]
    pub fn end(&self) -> usize {
        self.end
    }
}

impl<S> Value<S> {
    /// Returns `()` if it is a null, `None` otherwise.
    #[inline]
    pub fn as_null(&self) -> Option<()> {
        match self {
            Self::Null => Some(()),
            _ => None,
        }
    }

    /// Returns `bool` if it is a boolean, `None` otherwise.
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns a reference to [`Array`] if it is an array, `None` otherwise.
    #[inline]
    pub fn as_array(&self) -> Option<&Array<Span<Self>>> {
        match self {
            Self::Array(v) => Some(v),
            _ => None,
        }
    }

    /// Returns a reference to [`Object`] if it is an object, `None` otherwise.
    #[inline]
    pub fn as_object(&self) -> Option<&Object<Span<S>, Span<Value<S>>>> {
        match self {
            Self::Object(v) => Some(v),
            _ => None,
        }
    }

    /// Returns [`Number`] if it is a number, `None` otherwise.
    #[inline]
    pub fn as_number(&self) -> Option<Number> {
        match *self {
            Self::Number(v) => Some(v),
            _ => None,
        }
    }

    /// Returns `i64` if it is an integer and negative, `None` otherwise.
    #[inline]
    pub fn as_i64(&self) -> Option<i64> {
        self.as_number()?.as_i64()
    }

    /// Returns `i64` if it is an integer and positive, `None` otherwise.
    #[inline]
    pub fn as_u64(&self) -> Option<u64> {
        self.as_number()?.as_u64()
    }

    /// Returns `f64` if it is a floating point number or an integer that is too big, `None` otherwise.
    #[inline]
    pub fn as_f64(&self) -> Option<f64> {
        self.as_number()?.as_f64()
    }

    /// Returns `true` if it is a null, `false` otherwise.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.as_null().is_some()
    }

    /// Returns `true` if it is a boolean, `false` otherwise.
    #[inline]
    pub fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }

    /// Returns `true` if it is an array, `false` otherwise.
    #[inline]
    pub fn is_array(&self) -> bool {
        self.as_array().is_some()
    }

    /// Returns `true` if it is an object, `false` otherwise.
    #[inline]
    pub fn is_object(&self) -> bool {
        self.as_object().is_some()
    }

    /// Returns `true` if it is a number, `false` otherwise.
    #[inline]
    pub fn is_number(&self) -> bool {
        self.as_number().is_some()
    }

    /// Returns `true` if it is an integer and negative, `false` otherwise.
    #[inline]
    pub fn is_i64(&self) -> bool {
        self.as_i64().is_some()
    }

    /// Returns `true` if it is an integer and positive, `false` otherwise.
    #[inline]
    pub fn is_u64(&self) -> bool {
        self.as_u64().is_some()
    }

    /// Returns `true` if it is a floating point number or an integer that is too big, `false` otherwise.
    #[inline]
    pub fn is_f64(&self) -> bool {
        self.as_f64().is_some()
    }
}

impl<S: Deref<Target = str>> Value<S> {
    /// Returns string slice if it is a string, `None` otherwise.
    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(v) => Some(v),
            _ => None,
        }
    }

    /// Returns `true` if it is a string, `false` otherwise.
    #[inline]
    pub fn is_str(&self) -> bool {
        self.as_str().is_some()
    }

    /// Looks up a value by the given path and returns a reference.
    ///
    /// If the path is empty, then the root value is returned.
    ///
    /// # Example
    /// ```
    /// use flexon::{Value, jsonp};
    ///
    /// let val: Value = flexon::parse(r#"{"foo": ["bar", 123]}"#)?;
    ///
    /// assert!(val.pointer(jsonp!["foo", 1]).unwrap().is_number());
    /// assert!(val.pointer([0]).is_none());
    ///
    /// # Ok::<_, flexon::Error>(())
    /// ```
    pub fn pointer<P>(&self, p: P) -> Option<&Value<S>>
    where
        P: IntoIterator,
        P::Item: JsonPointer,
    {
        let mut tmp = self;

        for pointer in p {
            tmp = match tmp {
                Value::Object(obj) => obj.get(pointer.as_key()?),
                Value::Array(arr) => arr.get(pointer.as_index()?),
                _ => None,
            }?
            .data()
        }

        Some(tmp)
    }

    /// Looks up a value by the given path and returns a mutable reference.
    pub fn pointer_mut<P>(&mut self, p: P) -> Option<&mut Value<S>>
    where
        P: IntoIterator,
        P::Item: JsonPointer,
    {
        let mut tmp = self;

        for pointer in p {
            tmp = match tmp {
                Value::Object(obj) => obj.get_mut(pointer.as_key()?),
                Value::Array(arr) => arr.get_mut(pointer.as_index()?),
                _ => None,
            }?
            .data_mut()
        }

        Some(tmp)
    }
}

impl<S: Deref<Target = str>> Span<Value<S>> {
    /// Looks up a value by the given path and returns a reference.
    ///
    /// If the path is empty, then the root value is returned.
    ///
    /// # Example
    /// ```
    /// use flexon::{Value, jsonp};
    ///
    /// let val: Value = flexon::parse(r#"{"foo": ["bar", 123]}"#)?;
    ///
    /// assert!(val.pointer(jsonp!["foo", 1]).unwrap().is_number());
    /// assert!(val.pointer([0]).is_none());
    ///
    /// # Ok::<_, flexon::Error>(())
    /// ```
    pub fn pointer<P>(&self, p: P) -> Option<&Span<Value<S>>>
    where
        P: IntoIterator,
        P::Item: JsonPointer,
    {
        let mut tmp = self;

        for pointer in p {
            tmp = match tmp.data() {
                Value::Object(obj) => obj.get(pointer.as_key()?),
                Value::Array(arr) => arr.get(pointer.as_index()?),
                _ => None,
            }?
        }

        Some(tmp)
    }

    /// Looks up a value by the given path and returns a mutable reference.
    pub fn pointer_mut<P>(&mut self, p: P) -> Option<&mut Span<Value<S>>>
    where
        P: IntoIterator,
        P::Item: JsonPointer,
    {
        let mut tmp = self;

        for pointer in p {
            tmp = match tmp.data_mut() {
                Value::Object(obj) => obj.get_mut(pointer.as_key()?),
                Value::Array(arr) => arr.get_mut(pointer.as_index()?),
                _ => None,
            }?
        }

        Some(tmp)
    }
}

impl<S> Index<usize> for Value<S> {
    type Output = Value<S>;

    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        match self.as_array() {
            Some(v) => match v.get(idx) {
                Some(v) => v.data(),
                _ => panic!("given index does not exist in the array"),
            },
            _ => panic!("value is not an array"),
        }
    }
}

impl<S: Deref<Target = str>> Index<&str> for Value<S> {
    type Output = Value<S>;

    #[inline]
    fn index(&self, key: &str) -> &Self::Output {
        match self.as_object() {
            Some(v) => match v.get(key) {
                Some(v) => v.data(),
                _ => panic!("given key does not exist in the object"),
            },
            _ => panic!("value is not an object"),
        }
    }
}

impl<T: Deref<Target = str>> Deref for Span<T> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T: Debug> Debug for Span<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}..{}] ", self.start, self.end)?;
        self.data.fmt(f)
    }
}

impl<'a, S, V> ValueBuilder<'a, S> for Span<Value<V>>
where
    S: Source,
    V: StringBuilder<'a, S, Span<Error>>,
{
    const LAZY: bool = false;
    const CUSTOM_LITERAL: bool = false;

    type Error = Span<Error>;
    type Array = Array<Span<Value<V>>>;
    type Object = Object<Span<V>, Span<Value<V>>>;
    type String = Span<V>;

    #[inline]
    fn literal(_: &[u8]) -> Result<Self, Self::Error> {
        unimplemented!()
    }

    #[inline]
    fn integer(val: u64, neg: bool) -> Self {
        Self::new(Value::Number(match neg {
            true => Number::from_i64(val as _),
            _ => Number::from_u64(val),
        }))
    }

    #[inline]
    fn float(val: f64) -> Self {
        unsafe { Self::new(Value::Number(Number::from_f64(val).unwrap_unchecked())) }
    }

    #[inline]
    fn bool(val: bool) -> Self {
        Self::new(Value::Boolean(val))
    }

    #[inline]
    fn null() -> Self {
        Self::new(Value::Null)
    }

    #[inline]
    fn raw(_: &'a [u8]) -> Self {
        unimplemented!()
    }

    #[inline]
    fn apply_span(&mut self, start: usize, end: usize) {
        self.start = start;
        self.end = end;
    }
}

impl<'a, S, V> StringBuilder<'a, S, Span<Error>> for Span<V>
where
    S: Source,
    V: StringBuilder<'a, S, Span<Error>>,
{
    const REJECT_CTRL_CHAR: bool = V::REJECT_CTRL_CHAR;
    const REJECT_INVALID_ESCAPE: bool = V::REJECT_INVALID_ESCAPE;

    #[inline]
    fn new() -> Self {
        Self::new(V::new())
    }

    #[inline]
    fn on_escape(&mut self, s: &[u8]) {
        self.data.on_escape(s)
    }

    #[inline]
    fn on_chunk(&mut self, s: &'a [u8]) {
        self.data.on_chunk(s)
    }

    #[inline]
    fn on_final_chunk(&mut self, s: &'a [u8]) {
        self.data.on_final_chunk(s)
    }

    #[inline]
    fn apply_span(&mut self, start: usize, end: usize) {
        self.start = start;
        self.end = end;
    }

    #[inline]
    fn on_complete(&mut self, s: &'a [u8]) -> Result<(), Span<Error>> {
        self.data.on_complete(s)
    }
}

impl<E: ErrorBuilder> ErrorBuilder for Span<E> {
    #[inline]
    fn eof() -> Self {
        Self::new(E::eof())
    }

    #[inline]
    fn expected_colon() -> Self {
        Self::new(E::expected_colon())
    }

    #[inline]
    fn expected_value() -> Self {
        Self::new(E::expected_value())
    }

    #[inline]
    fn trailing_comma() -> Self {
        Self::new(E::trailing_comma())
    }

    #[inline]
    fn unclosed_string() -> Self {
        Self::new(E::unclosed_string())
    }

    #[inline]
    fn invalid_escape() -> Self {
        Self::new(E::invalid_escape())
    }

    #[inline]
    fn control_character() -> Self {
        Self::new(E::control_character())
    }

    #[inline]
    fn invalid_literal() -> Self {
        Self::new(E::invalid_literal())
    }

    #[inline]
    fn trailing_decimal() -> Self {
        Self::new(E::trailing_decimal())
    }

    #[inline]
    fn leading_decimal() -> Self {
        Self::new(E::leading_decimal())
    }

    #[inline]
    fn leading_zero() -> Self {
        Self::new(E::leading_zero())
    }

    #[inline]
    fn number_overflow() -> Self {
        Self::new(E::number_overflow())
    }

    #[inline]
    fn unexpected_token() -> Self {
        Self::new(E::unexpected_token())
    }

    #[inline]
    fn apply_span(&mut self, start: usize, end: usize) {
        self.start = start;
        self.end = end;
    }
}

impl<'a, S> From<Array<Span<Value<S>>>> for Span<Value<S>> {
    #[inline]
    fn from(value: Array<Span<Value<S>>>) -> Self {
        Self::new(Value::Array(value))
    }
}

impl<'a, S> From<Object<Span<S>, Span<Value<S>>>> for Span<Value<S>> {
    #[inline]
    fn from(value: Object<Span<S>, Span<Value<S>>>) -> Self {
        Self::new(Value::Object(value))
    }
}

impl<'a, S> From<Span<S>> for Span<Value<S>> {
    #[inline]
    fn from(value: Span<S>) -> Self {
        Span {
            data: Value::String(value.data),
            start: value.start,
            end: value.end,
        }
    }
}

impl<'a, S: Debug> Debug for Value<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("null"),
            Self::Array(v) => v.fmt(f),
            Self::Boolean(v) => v.fmt(f),
            Self::Number(v) => v.fmt(f),
            Self::String(v) => v.fmt(f),
            Self::Object(v) => v.fmt(f),
        }
    }
}
