mod index;
mod number;
mod object;
#[cfg(feature = "serde-json")]
mod serde_json;

use std::fmt::Debug;

pub use index::Index;
pub use number::Number;
pub use object::Object;

use crate::wrap;

#[cfg(feature = "span")]
use crate::Span;

/// Represents a JSON value.
pub enum Value {
    /// Represents `null` in JSON.
    Null,

    /// Represents a JSON array.
    Array(Vec<wrap!(Value)>),

    /// Represents a JSON boolean.
    Boolean(bool),

    /// Represents a JSON number.
    Number(Number),

    /// Represents a JSON string.
    String(String),

    /// Represents a JSON object.
    Object(Object),
}

#[cfg(feature = "span")]
impl Span<Value> {
    /// Returns `()` if the value is `Null`, `None` otherwise.
    pub fn as_null(&self) -> Option<Span<()>> {
        match self.data {
            Value::Null => Some(Span {
                data: (),
                start: self.start,
                end: self.end,
            }),
            _ => None,
        }
    }

    /// Returns `&str` if the value is a `String`, `None` otherwise.
    pub fn as_str(&self) -> Option<Span<&str>> {
        match &self.data {
            Value::String(v) => Some(Span {
                data: v,
                start: self.start,
                end: self.end,
            }),
            _ => None,
        }
    }

    /// Returns `bool` if the value is a `Boolean`, `None` otherwise.
    pub fn as_bool(&self) -> Option<Span<bool>> {
        match self.data {
            Value::Boolean(data) => Some(Span {
                data,
                start: self.start,
                end: self.end,
            }),
            _ => None,
        }
    }

    /// Returns `u64` if the value is a positive integer, `None` otherwise.
    pub fn as_u64(&self) -> Option<Span<u64>> {
        self.as_number().and_then(|v| v.as_u64())
    }

    /// Returns `i64` if the value is a negative integer, `None` otherwise.
    pub fn as_i64(&self) -> Option<Span<i64>> {
        self.as_number().and_then(|v| v.as_i64())
    }

    /// Returns `f64` if the value is a float, `None` otherwise.
    pub fn as_f64(&self) -> Option<Span<f64>> {
        self.as_number().and_then(|v| v.as_f64())
    }

    /// Returns `Number` if the value is a `Number`, `None` otherwise.
    pub fn as_number(&self) -> Option<Span<Number>> {
        match self.data {
            Value::Number(data) => Some(Span {
                data,
                start: self.start,
                end: self.end,
            }),
            _ => None,
        }
    }

    /// Returns a reference to the `Object` if the value is an `Object`, `None` otherwise.
    pub fn as_object(&self) -> Option<Span<&Object>> {
        match &self.data {
            Value::Object(data) => Some(Span {
                data,
                start: self.start,
                end: self.end,
            }),
            _ => None,
        }
    }

    /// Returns a slice if the value is an `Array`, `None` otherwise.
    pub fn as_array(&self) -> Option<Span<&[Span<Value>]>> {
        match &self.data {
            Value::Array(data) => Some(Span {
                data,
                start: self.start,
                end: self.end,
            }),
            _ => None,
        }
    }
}

impl Value {
    /// Returns `()` if the value is `Null`, `None` otherwise.
    pub fn as_null(&self) -> Option<()> {
        match self {
            Value::Null => Some(()),
            _ => None,
        }
    }

    /// Returns `&str` if the value is a `String`, `None` otherwise.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(v) => Some(v),
            _ => None,
        }
    }

    /// Returns `bool` if the value is a `Boolean`, `None` otherwise.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Boolean(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns `u64` if the value is a positive integer, `None` otherwise.
    pub fn as_u64(&self) -> Option<u64> {
        self.as_number().and_then(|v| v.as_u64())
    }

    /// Returns `i64` if the value is a negative integer, `None` otherwise.
    pub fn as_i64(&self) -> Option<i64> {
        self.as_number().and_then(|v| v.as_i64())
    }

    /// Returns `f64` if the value is a float, `None` otherwise.
    pub fn as_f64(&self) -> Option<f64> {
        self.as_number().and_then(|v| v.as_f64())
    }

    /// Returns `Number` if the value is a `Number`, `None` otherwise.
    pub fn as_number(&self) -> Option<Number> {
        match self {
            Value::Number(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns a reference to the `Object` if the value is an `Object`, `None` otherwise.
    pub fn as_object(&self) -> Option<&Object> {
        match self {
            Value::Object(v) => Some(v),
            _ => None,
        }
    }

    /// Returns a slice if the value is an `Array`, `None` otherwise.
    pub fn as_array(&self) -> Option<&[wrap!(Value)]> {
        match self {
            Value::Array(v) => Some(v),
            _ => None,
        }
    }

    /// Returns `true` if the value is `Null`.
    pub fn is_null(&self) -> bool {
        self.as_null().is_some()
    }

    /// Returns `true` if the value is a `String`.
    pub fn is_str(&self) -> bool {
        self.as_str().is_some()
    }

    /// Returns `true` if the value is a `Boolean`.
    pub fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }

    /// Returns `true` if the value is a positive integer.
    pub fn is_u64(&self) -> bool {
        self.as_u64().is_some()
    }

    /// Returns `true` if the value is a negative integer.
    pub fn is_i64(&self) -> bool {
        self.as_i64().is_some()
    }

    /// Returns `true` if the value is a float.
    pub fn is_f64(&self) -> bool {
        self.as_f64().is_some()
    }

    /// Returns `true` if the value is a `Number`.
    pub fn is_number(&self) -> bool {
        self.as_number().is_some()
    }

    /// Returns `true` if the value is an `Object`.
    pub fn is_object(&self) -> bool {
        self.as_object().is_some()
    }

    /// Returns `true` if the value is an `Array`.
    pub fn is_array(&self) -> bool {
        self.as_array().is_some()
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl From<Vec<Span<Value>>> for Value {
    fn from(value: Vec<Span<Value>>) -> Self {
        Self::Array(value)
    }
}

impl From<Vec<(Span<String>, Span<Value>)>> for Value {
    fn from(mut value: Vec<(Span<std::string::String>, Span<Value>)>) -> Self {
        value.sort_unstable_by(|a, b| a.0.data.cmp(&b.0.data));
        Self::Object(Object(value))
    }
}
