use core::fmt::{Debug, Formatter, Result};

/// Represents a JSON number.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Number(Kind);

#[derive(Clone, Copy)]
pub enum Kind {
    Unsigned(u64),
    Signed(i64),
    Float(f64),
}

impl Number {
    /// Creates JSON number from `u64`.
    #[inline(always)]
    pub fn from_u64(val: u64) -> Self {
        Self(Kind::Unsigned(val))
    }

    /// Creates JSON number from `i64`.
    #[inline(always)]
    pub fn from_i64(val: i64) -> Self {
        Self(Kind::Signed(val))
    }

    /// Creates JSON number from `f64`.
    ///
    /// Returns `None` if the number is infinite or NaN.
    #[inline]
    pub fn from_f64(val: f64) -> Option<Self> {
        match val.is_finite() {
            true => Some(Self(Kind::Float(val))),
            _ => None,
        }
    }

    /// Returns the number as `u64`, or `None` if it is not a positive integer.
    pub fn as_u64(&self) -> Option<u64> {
        match self.0 {
            Kind::Unsigned(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the number as `i64`, or `None` if it is not a negative integer.
    pub fn as_i64(&self) -> Option<i64> {
        match self.0 {
            Kind::Signed(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the number as `f64`, or `None` if it is not a float.
    pub fn as_f64(&self) -> Option<f64> {
        match self.0 {
            Kind::Float(v) => Some(v),
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
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self.0 {
            Kind::Unsigned(v) => v.fmt(f),
            Kind::Signed(v) => v.fmt(f),
            Kind::Float(v) => v.fmt(f),
        }
    }
}
