use std::ops;

use crate::{misc::Sealed, span::Span, value::Value};

pub trait Index<I>: Sealed {
    fn get(&self, index: I) -> Option<&Span<Value>>;
}

impl Index<usize> for Value {
    fn get(&self, index: usize) -> Option<&Span<Value>> {
        match self {
            Value::Array(v) => v.get(index),
            _ => None,
        }
    }
}

impl Index<&str> for Value {
    fn get(&self, index: &str) -> Option<&Span<Value>> {
        match self {
            Value::Object(v) => v.get(index),
            _ => None,
        }
    }
}

impl Sealed for Value {}

fn name(val: &Value) -> &str {
    match val {
        Value::Null => "null",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::String(_) => "string",
        Value::Number(_) => "number",
        Value::Boolean(_) => "boolean",
    }
}

impl ops::Index<usize> for Value {
    type Output = Span<Value>;

    fn index(&self, index: usize) -> &Self::Output {
        let Value::Array(arr) = self else {
            let one = name(self);
            let two = match self {
                Value::Object(_) => " by number",
                _ => "",
            };

            panic!("{one} cannot be indexed{two}")
        };

        &arr[index]
    }
}

impl ops::Index<&str> for Value {
    type Output = Span<Value>;

    fn index(&self, index: &str) -> &Self::Output {
        let Value::Object(obj) = self else {
            let one = name(self);
            let two = match self {
                Value::Array(_) => " by string",
                _ => "",
            };

            panic!("{one} cannot be indexed{two}")
        };

        match obj.get(index) {
            Some(v) => v,
            _ => panic!("`{index}` does not exist in the object"),
        }
    }
}
