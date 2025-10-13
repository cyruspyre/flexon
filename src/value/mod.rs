mod index;
mod number;
mod object;

use std::fmt::Debug;

pub use index::Index;
pub use number::Number;
pub use object::Object;

use crate::Wrap;

#[cfg(feature = "span")]
use crate::Span;

pub enum Value {
    Null,
    Array(Vec<Wrap<Value>>),
    Boolean(bool),
    Number(Number),
    String(String),
    Object(Object),
}

#[cfg(feature = "span")]
impl Span<Value> {
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

    pub fn as_u64(&self) -> Option<Span<u64>> {
        self.as_number().and_then(|v| v.as_u64())
    }

    pub fn as_i64(&self) -> Option<Span<i64>> {
        self.as_number().and_then(|v| v.as_i64())
    }

    pub fn as_f64(&self) -> Option<Span<f64>> {
        self.as_number().and_then(|v| v.as_f64())
    }

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
    pub fn as_null(&self) -> Option<()> {
        match self {
            Value::Null => Some(()),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Boolean(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.as_number().and_then(|v| v.as_u64())
    }

    pub fn as_i64(&self) -> Option<i64> {
        self.as_number().and_then(|v| v.as_i64())
    }

    pub fn as_f64(&self) -> Option<f64> {
        self.as_number().and_then(|v| v.as_f64())
    }

    pub fn as_number(&self) -> Option<&Number> {
        match self {
            Value::Number(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&Object> {
        match self {
            Value::Object(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Wrap<Value>]> {
        match self {
            Value::Array(v) => Some(v),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        self.as_null().is_some()
    }

    pub fn is_str(&self) -> bool {
        self.as_str().is_some()
    }

    pub fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }

    pub fn is_u64(&self) -> bool {
        self.as_u64().is_some()
    }

    pub fn is_i64(&self) -> bool {
        self.as_i64().is_some()
    }

    pub fn is_f64(&self) -> bool {
        self.as_f64().is_some()
    }

    pub fn is_number(&self) -> bool {
        self.as_number().is_some()
    }

    pub fn is_object(&self) -> bool {
        self.as_object().is_some()
    }

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
