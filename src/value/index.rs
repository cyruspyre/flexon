use std::ops;

use crate::{Wrap, misc::Sealed, value::Value, wrap};

#[cfg(feature = "span")]
use crate::Span;

/// A trait for indexing into a `flexon::Value`.
///
/// Implemented for types that can be indexed, i.e., `Value::Object` or `Value::Array`.
///
/// This is a sealed trait, only to be implemented by `flexon`.
pub trait Index<I>: Sealed {
    /// Returns a reference to the value at the given index, or `None` otherwise.
    ///
    /// The index can be a string or an `usize`. If `self` cannot be indexed with
    /// the given type, this method also returns `None`.
    fn get(&self, index: I) -> Option<&wrap!(Value)>;
}

impl Index<usize> for Value {
    fn get(&self, index: usize) -> Option<&wrap!(Value)> {
        match self {
            Value::Array(v) => v.get(index),
            _ => None,
        }
    }
}

impl Index<&str> for Value {
    fn get(&self, index: &str) -> Option<&wrap!(Value)> {
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
    type Output = Wrap<Value>;

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
    type Output = Wrap<Value>;

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
