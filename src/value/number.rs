use std::fmt::Debug;

use crate::span::Span;

#[derive(Clone, Copy)]
pub enum Number {
    Unsigned(u64),
    Signed(i64),
    Float(f64),
}

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

#[cfg(feature = "serde-json")]
impl Into<serde_json::Number> for Number {
    fn into(self) -> serde_json::Number {
        match self {
            Number::Unsigned(value) => value.into(),
            Number::Signed(value) => value.into(),
            Number::Float(value) => {
                serde_json::Number::from_f64(value).expect("value should be finite")
            }
        }
    }
}

#[cfg(feature = "serde-json")]
#[cfg(test)]
mod serde_json_tests {
    use super::*;

    #[test]
    fn unsigned_serde_json_number() {
        let num = Number::Unsigned(42);
        let json_num: serde_json::Number = num.into();
        assert_eq!(json_num.as_u64(), Some(42));
        assert!(json_num.is_u64());
    }

    #[test]
    fn unsigned_max_serde_json_number() {
        let num = Number::Unsigned(u64::MAX);
        let json_num: serde_json::Number = num.into();
        assert_eq!(json_num.as_u64(), Some(u64::MAX));
        assert!(json_num.is_u64());
    }

    #[test]
    fn signed_positive_serde_json_number() {
        let num = Number::Signed(42);
        let json_num: serde_json::Number = num.into();
        assert_eq!(json_num.as_i64(), Some(42));
        assert!(json_num.is_i64());
    }

    #[test]
    fn signed_negative_serde_json_number() {
        let num = Number::Signed(-42);
        let json_num: serde_json::Number = num.into();
        assert_eq!(json_num.as_i64(), Some(-42));
        assert!(json_num.is_i64());
    }

    #[test]
    fn signed_min_serde_json_number() {
        let num = Number::Signed(i64::MIN);
        let json_num: serde_json::Number = num.into();
        assert_eq!(json_num.as_i64(), Some(i64::MIN));
        assert!(json_num.is_i64());
    }

    #[test]
    fn signed_max_serde_json_number() {
        let num = Number::Signed(i64::MAX);
        let json_num: serde_json::Number = num.into();
        assert_eq!(json_num.as_i64(), Some(i64::MAX));
        assert!(json_num.is_i64());
    }

    #[test]
    fn float_positive_serde_json_number() {
        let num = Number::Float(3.14);
        let json_num: serde_json::Number = num.into();
        assert_eq!(json_num.as_f64(), Some(3.14));
        assert!(json_num.is_f64());
    }

    #[test]
    fn float_negative_serde_json_number() {
        let num = Number::Float(-3.14);
        let json_num: serde_json::Number = num.into();
        assert_eq!(json_num.as_f64(), Some(-3.14));
        assert!(json_num.is_f64());
    }

    #[test]
    fn float_zero_serde_json_number() {
        let num = Number::Float(0.0);
        let json_num: serde_json::Number = num.into();
        assert_eq!(json_num.as_f64(), Some(0.0));
        assert!(json_num.is_f64());
    }

    #[test]
    #[should_panic(expected = "value should be finite")]
    fn float_nan_panics() {
        let num = Number::Float(f64::NAN);
        let _: serde_json::Number = num.into();
    }

    #[test]
    #[should_panic(expected = "value should be finite")]
    fn float_infinity_panics() {
        let num = Number::Float(f64::INFINITY);
        let _: serde_json::Number = num.into();
    }

    #[test]
    #[should_panic(expected = "value should be finite")]
    fn float_neg_infinity_panics() {
        let num = Number::Float(f64::NEG_INFINITY);
        let _: serde_json::Number = num.into();
    }
}
