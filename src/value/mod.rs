mod array;
mod number;
mod object;

use crate::span::Span;

pub use array::Array;
pub use number::Number;
pub use object::Object;

#[derive(Debug)]
pub enum Value {
    Null,
    Array(Array),
    Boolean(bool),
    Number(Number),
    String(String),
    Object(Object),
}

impl From<Vec<Span<Value>>> for Value {
    fn from(value: Vec<Span<Value>>) -> Self {
        Self::Array(Array(value))
    }
}

impl From<Vec<(Span<String>, Span<Value>)>> for Value {
    fn from(mut value: Vec<(Span<std::string::String>, Span<Value>)>) -> Self {
        value.sort_unstable_by(|a, b| a.0.data.cmp(&b.0.data));
        Self::Object(Object(value))
    }
}
