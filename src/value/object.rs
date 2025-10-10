use std::fmt::Debug;

use crate::{span::Span, value::Value};

pub struct Object(pub(crate) Vec<(Span<String>, Span<Value>)>);

impl Object {
    pub fn get_key_value(&self, key: &str) -> Option<&(Span<String>, Span<Value>)> {
        match self.0.binary_search_by(|v| key.cmp(&v.0.data)) {
            Ok(v) => Some(&self.0[v]),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Span<Value>> {
        self.get_key_value(key).map(|v| &v.1)
    }
}

impl Debug for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(self.0.iter().map(|(k, v)| (k, v)))
            .finish()
    }
}
