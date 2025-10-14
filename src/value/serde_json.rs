#[cfg(feature = "span")]
use crate::Span;

use crate::{Value, value::Number};

impl Into<serde_json::Value> for Value {
    fn into(self) -> serde_json::Value {
        match self {
            Value::Null => serde_json::Value::Null,
            Value::Array(values) => {
                serde_json::Value::Array(values.into_iter().map(|value| value.into()).collect())
            }
            Value::Boolean(value) => value.into(),
            Value::Number(value) => serde_json::Value::Number(value.into()),
            Value::String(value) => value.into(),
            Value::Object(object) => serde_json::Value::Object(
                object
                    .0
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            #[cfg(feature = "span")]
                            k.data,
                            #[cfg(not(feature = "span"))]
                            k,
                            v.into(),
                        )
                    })
                    .collect(),
            ),
        }
    }
}

#[cfg(feature = "span")]
impl Into<serde_json::Value> for Span<Value> {
    fn into(self) -> serde_json::Value {
        self.data.into()
    }
}

impl Into<serde_json::Number> for Number {
    fn into(self) -> serde_json::Number {
        match self {
            Number::Unsigned(value) => value.into(),
            Number::Signed(value) => value.into(),
            Number::Float(value) => {
                serde_json::Number::from_f64(value).expect("floating point number should be finite")
            }
        }
    }
}

#[cfg(feature = "span")]
impl Into<serde_json::Number> for Span<Number> {
    fn into(self) -> serde_json::Number {
        self.data.into()
    }
}
