use std::fmt::Debug;

use crate::{Wrap, value::Value};

pub struct Object(pub(crate) Vec<(Wrap<String>, Wrap<Value>)>);

impl Object {
    pub fn get_key_value(&self, key: &str) -> Option<&(Wrap<String>, Wrap<Value>)> {
        match self.0.binary_search_by(|v| {
            key.cmp(
                #[cfg(feature = "span")]
                &v.0.data,
                #[cfg(not(feature = "span"))]
                &v.0,
            )
        }) {
            Ok(v) => Some(&self.0[v]),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Wrap<Value>> {
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
