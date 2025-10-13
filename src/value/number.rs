use std::fmt::Debug;

#[cfg(feature = "span")]
use crate::span::Span;

#[derive(Clone, Copy)]
pub enum Number {
    Unsigned(u64),
    Signed(i64),
    Float(f64),
}

#[cfg(feature = "span")]
impl Span<Number> {
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
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Number::Unsigned(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Number::Signed(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Number::Float(v) => Some(*v),
            _ => None,
        }
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
            Self::Unsigned(arg0) => f.debug_tuple("Unsigned").field(arg0).finish(),
            Self::Signed(arg0) => f.debug_tuple("Signed").field(arg0).finish(),
            Self::Float(arg0) => f.debug_tuple("Float").field(arg0).finish(),
        }
    }
}
