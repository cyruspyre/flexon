use crate::{span::Span, value::Value};

#[derive(Debug)]
pub struct Array(pub(crate) Vec<Span<Value>>);

impl Array {
    pub fn get(&self, index: usize) -> Option<&Span<Value>> {
        self.0.get(index)
    }
}
