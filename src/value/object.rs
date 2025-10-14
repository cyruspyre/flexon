use std::fmt::Debug;

use crate::{value::Value, wrap};

#[cfg(feature = "span")]
use crate::Span;

/// A JSON object, represented as a sorted list of key-value pairs which use
/// binary search for lookups.
pub struct Object(pub(crate) Vec<(wrap!(String), wrap!(Value))>);

impl Object {
    /// Returns the key-value pair corresponding to the given key, if it exists.
    pub fn get_key_value(&self, key: &str) -> Option<&(wrap!(String), wrap!(Value))> {
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

    /// Returns a reference to the value corresponding to the key.
    pub fn get(&self, key: &str) -> Option<&wrap!(Value)> {
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
