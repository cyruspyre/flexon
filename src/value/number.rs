use std::fmt::Debug;

#[cfg(feature = "span")]
use crate::span::Span;

/// Represents a JSON number, stored as either an unsigned integer (`u64`),
/// signed integer (`i64`), or a float (`f64`).
#[derive(Clone, Copy)]
pub enum Number {
    Unsigned(u64),
    Signed(i64),
    Float(f64),
}

#[cfg(feature = "span")]
impl Span<Number> {
    /// Returns the number as `u64`, or `None` if it is not a positive integer.
    pub fn as_u64(&self) -> Option<Span<u64>> {
        match self.data {
            Number::Unsigned(data) => Some(Span {
                data,
                start: self.start,
                end: self.end,
            }),
            _ => None,
        }
    }

    /// Returns the number as `i64`, or `None` if it is not a negative integer.
    pub fn as_i64(&self) -> Option<Span<i64>> {
        match self.data {
            Number::Signed(data) => Some(Span {
                data,
                start: self.start,
                end: self.end,
            }),
            _ => None,
        }
    }

    /// Returns the number as `f64`, or `None` if it is not a float.
    pub fn as_f64(&self) -> Option<Span<f64>> {
        match self.data {
            Number::Float(data) => Some(Span {
                data,
                start: self.start,
                end: self.end,
            }),
            _ => None,
        }
    }
}

impl Number {
    /// Returns the number as `u64`, or `None` if it is not a positive integer.
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Number::Unsigned(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns the number as `i64`, or `None` if it is not a negative integer.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Number::Signed(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns the number as `f64`, or `None` if it is not a float.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Number::Float(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns `true` if the number is an unsigned integer or a positive integer.
    pub fn is_u64(&self) -> bool {
        self.as_u64().is_some()
    }

    /// Returns `true` if the number is a signed integer or a negative integer.
    pub fn is_i64(&self) -> bool {
        self.as_i64().is_some()
    }

    /// Returns `true` if the number is a float.
    pub fn is_f64(&self) -> bool {
        self.as_f64().is_some()
    }
}

impl Debug for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            return match self {
                Self::Unsigned(v) => v.fmt(f),
                Self::Signed(v) => v.fmt(f),
                Self::Float(v) => v.fmt(f),
            };
        }

        match self {
            Self::Unsigned(v) => f.debug_tuple("Unsigned").field(v).finish(),
            Self::Signed(v) => f.debug_tuple("Signed").field(v).finish(),
            Self::Float(v) => f.debug_tuple("Float").field(v).finish(),
        }
    }
}
