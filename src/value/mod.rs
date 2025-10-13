mod index;
mod number;
mod object;

use std::fmt::Debug;

use crate::span::Span;

pub use index::Index;
pub use number::Number;
pub use object::Object;

pub enum Value {
    Null,
    Array(Vec<Span<Value>>),
    Boolean(bool),
    Number(Number),
    String(String),
    Object(Object),
}

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

    pub fn as_array(&self) -> Option<&[Span<Value>]> {
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

#[cfg(feature = "serde-json")]
impl Into<serde_json::Value> for Value {
    fn into(self) -> serde_json::Value {
        match self {
            Value::Null => serde_json::Value::Null,
            Value::Array(values) => serde_json::Value::Array(
                values.into_iter().map(|value| value.data.into()).collect(),
            ),
            Value::Boolean(value) => value.into(),
            Value::Number(value) => serde_json::Value::Number(value.into()),
            Value::String(value) => value.into(),
            Value::Object(object) => serde_json::Value::Object(
                object
                    .0
                    .into_iter()
                    .map(|(k, v)| (k.data, v.data.into()))
                    .collect(),
            ),
        }
    }
}

#[cfg(feature = "serde-json")]
#[cfg(test)]
mod serde_json_tests {
    use super::*;

    #[test]
    fn null_to_serde_json_value() {
        let value = Value::Null;
        let json_value: serde_json::Value = value.into();
        assert_eq!(json_value, serde_json::Value::Null);
        assert!(json_value.is_null());
    }

    #[test]
    fn boolean_true_to_serde_json_value() {
        let value = Value::Boolean(true);
        let json_value: serde_json::Value = value.into();
        assert_eq!(json_value, serde_json::Value::Bool(true));
        assert_eq!(json_value.as_bool(), Some(true));
    }

    #[test]
    fn boolean_false_to_serde_json_value() {
        let value = Value::Boolean(false);
        let json_value: serde_json::Value = value.into();
        assert_eq!(json_value, serde_json::Value::Bool(false));
        assert_eq!(json_value.as_bool(), Some(false));
    }

    #[test]
    fn string_to_serde_json_value() {
        let value = Value::String("hello world".to_string());
        let json_value: serde_json::Value = value.into();
        assert_eq!(
            json_value,
            serde_json::Value::String("hello world".to_string())
        );
        assert_eq!(json_value.as_str(), Some("hello world"));
    }

    #[test]
    fn empty_string_to_serde_json_value() {
        let value = Value::String(String::new());
        let json_value: serde_json::Value = value.into();
        assert_eq!(json_value, serde_json::Value::String(String::new()));
        assert_eq!(json_value.as_str(), Some(""));
    }

    #[test]
    fn number_unsigned_to_serde_json_value() {
        let value = Value::Number(Number::Unsigned(42));
        let json_value: serde_json::Value = value.into();
        assert_eq!(json_value.as_u64(), Some(42));
    }

    #[test]
    fn number_signed_to_serde_json_value() {
        let value = Value::Number(Number::Signed(-42));
        let json_value: serde_json::Value = value.into();
        assert_eq!(json_value.as_i64(), Some(-42));
    }

    #[test]
    fn number_float_to_serde_json_value() {
        let value = Value::Number(Number::Float(3.14));
        let json_value: serde_json::Value = value.into();
        assert_eq!(json_value.as_f64(), Some(3.14));
    }

    #[test]
    fn empty_array_to_serde_json_value() {
        let value = Value::Array(vec![]);
        let json_value: serde_json::Value = value.into();
        assert!(json_value.is_array());
        assert_eq!(json_value.as_array().unwrap().len(), 0);
    }

    #[test]
    fn array_with_values_to_serde_json_value() {
        let value = Value::Array(vec![
            Span {
                data: Value::Number(Number::Unsigned(1)),
                start: 0,
                end: 1,
            },
            Span {
                data: Value::String("test".to_string()),
                start: 2,
                end: 8,
            },
            Span {
                data: Value::Boolean(true),
                start: 9,
                end: 13,
            },
        ]);
        let json_value: serde_json::Value = value.into();
        assert!(json_value.is_array());
        let arr = json_value.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_u64(), Some(1));
        assert_eq!(arr[1].as_str(), Some("test"));
        assert_eq!(arr[2].as_bool(), Some(true));
    }

    #[test]
    fn nested_array_to_serde_json_value() {
        let value = Value::Array(vec![
            Span {
                data: Value::Array(vec![
                    Span {
                        data: Value::Number(Number::Unsigned(1)),
                        start: 0,
                        end: 1,
                    },
                    Span {
                        data: Value::Number(Number::Unsigned(2)),
                        start: 2,
                        end: 3,
                    },
                ]),
                start: 0,
                end: 4,
            },
            Span {
                data: Value::Array(vec![Span {
                    data: Value::Number(Number::Unsigned(3)),
                    start: 5,
                    end: 6,
                }]),
                start: 5,
                end: 7,
            },
        ]);
        let json_value: serde_json::Value = value.into();
        assert!(json_value.is_array());
        let arr = json_value.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].as_array().unwrap().len(), 2);
        assert_eq!(arr[1].as_array().unwrap().len(), 1);
    }

    #[test]
    fn empty_object_to_serde_json_value() {
        let value = Value::Object(Object(vec![]));
        let json_value: serde_json::Value = value.into();
        assert!(json_value.is_object());
        assert_eq!(json_value.as_object().unwrap().len(), 0);
    }

    #[test]
    fn object_with_fields_to_serde_json_value() {
        let value = Value::Object(Object(vec![
            (
                Span {
                    data: "name".to_string(),
                    start: 0,
                    end: 6,
                },
                Span {
                    data: Value::String("Alice".to_string()),
                    start: 7,
                    end: 14,
                },
            ),
            (
                Span {
                    data: "age".to_string(),
                    start: 15,
                    end: 20,
                },
                Span {
                    data: Value::Number(Number::Unsigned(30)),
                    start: 21,
                    end: 23,
                },
            ),
            (
                Span {
                    data: "active".to_string(),
                    start: 24,
                    end: 32,
                },
                Span {
                    data: Value::Boolean(true),
                    start: 33,
                    end: 37,
                },
            ),
        ]));
        let json_value: serde_json::Value = value.into();
        assert!(json_value.is_object());
        let obj = json_value.as_object().unwrap();
        assert_eq!(obj.len(), 3);
        assert_eq!(obj.get("name").unwrap().as_str(), Some("Alice"));
        assert_eq!(obj.get("age").unwrap().as_u64(), Some(30));
        assert_eq!(obj.get("active").unwrap().as_bool(), Some(true));
    }

    #[test]
    fn nested_object_to_serde_json_value() {
        let value = Value::Object(Object(vec![(
            Span {
                data: "user".to_string(),
                start: 0,
                end: 6,
            },
            Span {
                data: Value::Object(Object(vec![(
                    Span {
                        data: "name".to_string(),
                        start: 7,
                        end: 13,
                    },
                    Span {
                        data: Value::String("Bob".to_string()),
                        start: 14,
                        end: 19,
                    },
                )])),
                start: 7,
                end: 20,
            },
        )]));
        let json_value: serde_json::Value = value.into();
        assert!(json_value.is_object());
        let obj = json_value.as_object().unwrap();
        assert_eq!(obj.len(), 1);
        let nested = obj.get("user").unwrap().as_object().unwrap();
        assert_eq!(nested.get("name").unwrap().as_str(), Some("Bob"));
    }

    #[test]
    fn complex_nested_structure_to_serde_json_value() {
        let value = Value::Object(Object(vec![(
            Span {
                data: "users".to_string(),
                start: 0,
                end: 7,
            },
            Span {
                data: Value::Array(vec![Span {
                    data: Value::Object(Object(vec![
                        (
                            Span {
                                data: "id".to_string(),
                                start: 8,
                                end: 12,
                            },
                            Span {
                                data: Value::Number(Number::Unsigned(1)),
                                start: 13,
                                end: 14,
                            },
                        ),
                        (
                            Span {
                                data: "tags".to_string(),
                                start: 15,
                                end: 21,
                            },
                            Span {
                                data: Value::Array(vec![
                                    Span {
                                        data: Value::String("admin".to_string()),
                                        start: 22,
                                        end: 29,
                                    },
                                    Span {
                                        data: Value::String("active".to_string()),
                                        start: 30,
                                        end: 38,
                                    },
                                ]),
                                start: 22,
                                end: 39,
                            },
                        ),
                    ])),
                    start: 8,
                    end: 40,
                }]),
                start: 8,
                end: 41,
            },
        )]));
        let json_value: serde_json::Value = value.into();
        let users = json_value
            .as_object()
            .unwrap()
            .get("users")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(users.len(), 1);
        let user = users[0].as_object().unwrap();
        assert_eq!(user.get("id").unwrap().as_u64(), Some(1));
        let tags = user.get("tags").unwrap().as_array().unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].as_str(), Some("admin"));
        assert_eq!(tags[1].as_str(), Some("active"));
    }
}
