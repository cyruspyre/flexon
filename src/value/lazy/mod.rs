//! Lazy JSON value representation.

mod array;
mod dummy;
mod object;

use core::{
    fmt::{self, Debug, Formatter},
    hint::unreachable_unchecked,
    ops::{Deref, Index, IndexMut},
    str::from_utf8_unchecked,
};

use crate::{
    Error, Parser,
    pointer::JsonPointer,
    source::{NonVolatile, Source},
    value::{Number, borrowed::String, builder::ValueBuilder, lazy::dummy::*},
};

pub use {array::Array, object::Object};

/// Represents a lazy JSON value.
///
/// The initial value will always be [`Value::Raw`] until you call its mutating APIs.
pub enum Value<'a> {
    /// Represents a JSON null value.
    Null,

    /// Represents an unparsed raw JSON value.
    Raw(Raw<'a>),

    /// Represents a JSON array.
    Array(Array<'a>),

    /// Represents a JSON object.
    Object(Object<'a>),

    /// Represents a JSON string.
    String(String<'a>),

    /// Represents a JSON number.
    Number(Number),

    /// Represents a JSON boolean.
    Boolean(bool),
}

/// Represents an unparsed JSON value.
///
/// It may have trailing characters that are irrelevant to its type.
#[repr(transparent)]
pub struct Raw<'a>(&'a str);

impl<'a> Raw<'a> {
    /// Trims the raw JSON value to its end excluding the trailing characters that are irrelevant to its type.
    #[inline]
    pub fn trim_to_value(&self) -> &'a str {
        let mut tmp = Parser::new(self.0);
        tmp.skip_value_unchecked();
        unsafe { self.0.get_unchecked(..=tmp.idx()) }
    }
}

impl<'a> Value<'a> {
    /// Returns unparsed raw JSON if it is still in raw form, `None` otherwise.
    #[inline]
    pub fn as_raw(&'a self) -> Option<Raw<'a>> {
        match self {
            Self::Raw(v) => Some(Raw(v)),
            _ => None,
        }
    }

    /// Returns `()` if it is a null, `None` otherwise.
    #[inline]
    pub fn as_null(&mut self) -> Option<()> {
        match self {
            Self::Null => Some(()),
            Self::Raw(s) if unsafe { *s.as_ptr() == b'n' } => {
                *self = Self::Null;
                Some(())
            }
            _ => None,
        }
    }

    /// Returns `bool` if it is a boolean, `None` otherwise.
    #[inline]
    pub fn as_bool(&mut self) -> Option<bool> {
        match self {
            Self::Boolean(v) => Some(*v),
            Self::Raw(s) if unsafe { matches!(*s.as_ptr(), b't' | b'f') } => unsafe {
                let tmp = *s.as_ptr() == b't';
                *self = Self::Boolean(tmp);
                Some(tmp)
            },
            _ => None,
        }
    }

    /// Returns a mutable reference to [`Array`] if it is an array, `None` otherwise.
    #[inline]
    pub fn as_array(&mut self) -> Option<&mut Array<'a>> {
        Some(match *self {
            Self::Array(ref mut v) => v,
            Self::Raw(Raw(s)) if unsafe { *s.as_ptr() == b'[' } => unsafe {
                *self = Self::Array(Array::new(s));

                match self {
                    Self::Array(v) => v,
                    _ => unreachable_unchecked(),
                }
            },
            _ => return None,
        })
    }

    /// Returns [`Array`] if it is an array, `None` otherwise.
    #[inline]
    pub fn into_array(self) -> Option<Array<'a>> {
        Some(match self {
            Self::Array(v) => v,
            Self::Raw(Raw(s)) if unsafe { *s.as_ptr() == b'[' } => Array::new(s),
            _ => return None,
        })
    }

    /// Returns a mutable reference to [`Object`] if it is an object, `None` otherwise.
    #[inline]
    pub fn as_object(&mut self) -> Option<&mut Object<'a>> {
        Some(match *self {
            Self::Object(ref mut v) => v,
            Self::Raw(Raw(s)) if unsafe { *s.as_ptr() == b'{' } => unsafe {
                *self = Self::Object(Object::new(s));

                match self {
                    Self::Object(v) => v,
                    _ => unreachable_unchecked(),
                }
            },
            _ => return None,
        })
    }

    /// Returns [`Object`] if it is an object, `None` otherwise.
    #[inline]
    pub fn into_object(self) -> Option<Object<'a>> {
        Some(match self {
            Self::Object(v) => v,
            Self::Raw(Raw(s)) if unsafe { *s.as_ptr() == b'{' } => Object::new(s),
            _ => return None,
        })
    }

    /// Returns [`Number`] if it is a number, `None` otherwise.
    #[inline]
    pub fn as_number(&mut self) -> Option<Number> {
        Some(match *self {
            Self::Number(v) => v,
            Self::Raw(Raw(s))
                if unsafe {
                    matches!(
                        *s.as_ptr(),
                        b'-' | b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9'
                    )
                } =>
            unsafe {
                let mut tmp = Parser::new(s);

                tmp.inc(1);
                *self = Self::Number(tmp.number_unchecked());

                match *self {
                    Self::Number(v) => v,
                    _ => unreachable_unchecked(),
                }
            },
            _ => return None,
        })
    }

    /// Returns `i64` if it is an integer and negative, `None` otherwise.
    #[inline]
    pub fn as_i64(&mut self) -> Option<i64> {
        self.as_number()?.as_i64()
    }

    /// Returns `i64` if it is an integer and positive, `None` otherwise.
    #[inline]
    pub fn as_u64(&mut self) -> Option<u64> {
        self.as_number()?.as_u64()
    }

    /// Returns `f64` if it is a floating point number or an integer that is too big, `None` otherwise.
    #[inline]
    pub fn as_f64(&mut self) -> Option<f64> {
        self.as_number()?.as_f64()
    }

    /// Returns string slice if it is a string, `None` otherwise.
    #[inline]
    pub fn as_str(&mut self) -> Option<&str> {
        Some(match *self {
            Self::String(ref mut v) => v,
            Self::Raw(Raw(s)) if unsafe { *s.as_ptr() == b'"' } => unsafe {
                let mut tmp = Parser::new(s);

                tmp.inc(1);
                *self = Self::String(tmp.string_unchecked2());

                match self {
                    Self::String(v) => v,
                    _ => unreachable_unchecked(),
                }
            },
            _ => return None,
        })
    }

    /// Returns `true` if it is still a raw value, `false` otherwise.
    #[inline]
    pub fn is_raw(&self) -> bool {
        match self {
            Self::Raw(_) => true,
            _ => false,
        }
    }

    /// Returns `true` if it is a string, `false` otherwise.
    #[inline]
    pub fn is_str(&self) -> bool {
        match self {
            Self::String(_) => true,
            Self::Raw(s) => unsafe { *s.as_ptr() == b'"' },
            _ => false,
        }
    }

    /// Returns `true` if it is a null, `false` otherwise.
    #[inline]
    pub fn is_null(&self) -> bool {
        match self {
            Self::Null => true,
            Self::Raw(s) => unsafe { *s.as_ptr() == b'n' },
            _ => false,
        }
    }

    /// Returns `true` if it is a boolean, `false` otherwise.
    #[inline]
    pub fn is_bool(&self) -> bool {
        match self {
            Self::Boolean(_) => true,
            Self::Raw(s) => unsafe { matches!(*s.as_ptr(), b't' | b'f') },
            _ => false,
        }
    }

    /// Returns `true` if it is an array, `false` otherwise.
    #[inline]
    pub fn is_array(&self) -> bool {
        match self {
            Self::Array(_) => true,
            Self::Raw(s) => unsafe { *s.as_ptr() == b'[' },
            _ => false,
        }
    }

    /// Returns `true` if it is an object, `false` otherwise.
    #[inline]
    pub fn is_object(&self) -> bool {
        match self {
            Self::Object(_) => true,
            Self::Raw(s) => unsafe { *s.as_ptr() == b'{' },
            _ => false,
        }
    }

    /// Returns `true` if it is a number, `false` otherwise.
    #[inline]
    pub fn is_number(&self) -> bool {
        match self {
            Self::Number(_) => true,
            Self::Raw(s) => unsafe {
                matches!(
                    *s.as_ptr(),
                    b'-' | b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9'
                )
            },
            _ => false,
        }
    }

    /// Looks up a value by the given path.
    ///
    /// This is faster than its mutating APIs if you only need to get a particular value once
    /// as it doesn't cache the values. If the path is empty, then the root value is returned.
    /// The returned value will generally be a raw value unless it is a string in which case
    /// it will return a borrowed string. Numbers, booleans and nulls are returned as is.
    ///
    /// # Example
    /// ```
    /// use flexon::{LazyValue, jsonp};
    ///
    /// let val: LazyValue = flexon::parse(r#"{"foo": ["bar", 123]}"#)?;
    ///
    /// assert!(val.pointer(jsonp!["foo", 1]).unwrap().is_number());
    /// assert!(val.pointer([0]).is_none());
    ///
    /// # Ok::<_, flexon::Error>(())
    /// ```
    pub fn pointer<P>(&'a self, p: P) -> Option<Value<'a>>
    where
        P: IntoIterator,
        P::Item: JsonPointer,
    {
        let src: &'a str = match self {
            Value::Raw(v) => v,
            Value::Object(v) => v.raw,
            Value::Array(array) => array.raw,
            _ => unsafe {
                return Some(match *self {
                    // valid borrowed string even if the original string is now in heap
                    // both of them will continue to live for the same lifetime
                    Value::String(ref v) => Value::String(String::from_slice(v.as_bytes())),
                    Value::Null => Value::Null,
                    Value::Number(v) => Value::Number(v),
                    Value::Boolean(v) => Value::Boolean(v),
                    _ => unreachable_unchecked(),
                });
            },
        };
        let mut iter = p.into_iter();
        let Some(mut pointer) = iter.next() else {
            return Some(Value::Raw(Raw(src)));
        };
        let mut tmp = Parser::new(src);
        let mut char = tmp.skip_whitespace();

        'main: loop {
            if let Some(key) = pointer.as_key()
                && char == b'{'
            {
                loop {
                    match tmp.skip_whitespace() {
                        b'"' => unsafe {
                            let new = tmp.string_unchecked2();
                            tmp.skip_whitespace(); // skip ':'
                            char = tmp.skip_whitespace();

                            if &*new == key {
                                let Some(v) = iter.next() else { break 'main };
                                pointer = v;
                                break;
                            }

                            match char {
                                b'"' => tmp.skip_string_unchecked(),
                                b'{' | b'[' => tmp.skip_container_unchecked(),
                                _ => tmp.skip_literal_unchecked(),
                            }
                        },
                        b',' => continue,
                        b'}' => return None,
                        _ => unsafe { unreachable_unchecked() },
                    }
                }
            } else if let Some(mut idx) = pointer.as_index()
                && char == b'['
            {
                loop {
                    match tmp.skip_whitespace() {
                        b',' => continue,
                        b']' => return None,
                        v if idx == 0 => match iter.next() {
                            Some(new) => {
                                pointer = new;
                                char = v;
                                break;
                            }
                            _ => break 'main,
                        },
                        v => unsafe {
                            idx -= 1;
                            match v {
                                b'"' => tmp.skip_string_unchecked(),
                                b'{' | b'[' => tmp.skip_container_unchecked(),
                                _ => tmp.skip_literal_unchecked(),
                            }
                        },
                    }
                }
            } else {
                return None;
            }
        }

        unsafe { Some(Value::Raw(Raw(src.get_unchecked(tmp.idx()..)))) }
    }
}

impl<'a> Parser<'a, &'a str> {
    #[inline]
    pub(super) fn number_unchecked(&mut self) -> Number {
        let tmp = self.cur();

        let neg = tmp == b'-';
        if neg {
            self.inc(1)
        }

        let start = self.idx();
        let (val, is_int) = unsafe { self.parse_u64() };

        match is_int {
            true => match neg {
                true => Number::from_i64(val.wrapping_neg() as _),
                _ => Number::from_u64(val),
            },
            _ => unsafe {
                Number::from_f64(self.parse_f64(val, neg, start).unwrap_unchecked())
                    .unwrap_unchecked()
            },
        }
    }
}

impl<'a, S: Source<Volatility = NonVolatile>> ValueBuilder<'a, S> for Value<'a> {
    const LAZY: bool = true;
    const CUSTOM_LITERAL: bool = false;

    type Error = Error;
    type Array = _Array;
    type Object = _Object<'a, S, Self>;
    type String = _String;

    #[inline]
    fn literal(_: &[u8]) -> Result<Self, Self::Error> {
        unimplemented!()
    }

    #[inline]
    fn integer(_: u64, _: bool) -> Self {
        unimplemented!()
    }

    #[inline]
    fn float(_: f64) -> Self {
        unimplemented!()
    }

    #[inline]
    fn bool(_: bool) -> Self {
        unimplemented!()
    }

    #[inline]
    fn null() -> Self {
        unimplemented!()
    }

    #[inline]
    fn raw(s: &'a [u8]) -> Self {
        Self::Raw(Raw(unsafe { from_utf8_unchecked(s) }))
    }

    #[inline]
    fn apply_span(&mut self, _: usize, _: usize) {}
}

impl<'a> Into<Value<'a>> for _Array {
    #[inline]
    fn into(self) -> Value<'a> {
        unimplemented!()
    }
}

impl<'a, S: Source<Volatility = NonVolatile>> Into<Value<'a>> for _Object<'a, S, Value<'a>> {
    #[inline]
    fn into(self) -> Value<'a> {
        unimplemented!()
    }
}

impl<'a> Into<Value<'a>> for _String {
    #[inline]
    fn into(self) -> Value<'a> {
        unimplemented!()
    }
}

impl<'a> Index<usize> for Value<'a> {
    type Output = Value<'a>;

    #[inline]
    fn index(&self, _: usize) -> &Self::Output {
        unimplemented!("lazy values must be indexed mutably")
    }
}

impl<'a> IndexMut<usize> for Value<'a> {
    #[inline]
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        match self.as_array() {
            Some(v) => match v.get(idx) {
                Some(v) => v,
                _ => panic!("given index does not exist in the array"),
            },
            _ => panic!("value is not an array"),
        }
    }
}

impl<'a> Index<&str> for Value<'a> {
    type Output = Value<'a>;

    #[inline]
    fn index(&self, _: &str) -> &Self::Output {
        unimplemented!("lazy values must be indexed mutably")
    }
}

impl<'a> IndexMut<&str> for Value<'a> {
    #[inline]
    fn index_mut(&mut self, key: &str) -> &mut Self::Output {
        match self.as_object() {
            Some(v) => match v.get(key) {
                Some(v) => v,
                _ => panic!("given key does not exist in the object"),
            },
            _ => panic!("value is not an object"),
        }
    }
}

impl Deref for Raw<'_> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl Debug for Raw<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Raw").field(&self.trim_to_value()).finish()
    }
}

impl Debug for Value<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Raw(v) => v.fmt(f),
            Self::Null => f.write_str("null"),
            Self::Array(v) => v.fmt(f),
            Self::Boolean(v) => v.fmt(f),
            Self::Number(v) => v.fmt(f),
            Self::String(v) => v.fmt(f),
            Self::Object(v) => v.fmt(f),
        }
    }
}
