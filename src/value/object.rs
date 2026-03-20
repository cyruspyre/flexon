use core::{
    fmt::{Debug, Formatter, Result},
    ops::{Deref, Index, IndexMut},
};

use crate::value::{Array, builder::*};

/// Represents a JSON object.
///
/// As of right now it stores elements as an array of key-value pairs. The original JSON source order is preserved.
#[derive(Clone)]
pub struct Object<K, V>(Array<(K, V)>);

impl<K, V> Object<K, V> {
    /// Returns the object as a slice of key-value pairs.
    #[inline]
    pub fn as_slice(&self) -> &[(K, V)] {
        &self.0
    }

    /// Returns the object as a mutable slice of key-value pairs.
    #[inline]
    pub fn as_slice_mut(&mut self) -> &mut [(K, V)] {
        &mut self.0
    }

    /// Returns the number of elements.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl<K: Deref<Target = str>, V> Object<K, V> {
    /// Returns a reference to the value associated with the given key, `None` otherwise.
    pub fn get(&self, key: &str) -> Option<&V> {
        for (k, v) in self.as_slice() {
            if &**k == key {
                return Some(v);
            }
        }

        None
    }

    /// Returns a mutable reference to the value associated with the given key, `None` otherwise.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        for (k, v) in self.as_slice_mut() {
            if &**k == key {
                return Some(v);
            }
        }

        None
    }

    /// Returns a reference to the key-value pair associated with the given key, `None` otherwise.
    pub fn get_key_value(&self, key: &str) -> Option<&(K, V)> {
        for v in self.as_slice() {
            if &*v.0 == key {
                return Some(v);
            }
        }

        None
    }

    /// Returns a mutable reference to the key-value pair associated with the given key, `None` otherwise.
    pub fn get_key_value_mut(&mut self, key: &str) -> Option<(&mut K, &mut V)> {
        for (k, v) in self.as_slice_mut() {
            if &**k == key {
                return Some((k, v));
            }
        }

        None
    }
}

impl<K, V> ObjectBuilder<K, V> for Object<K, V> {
    #[inline]
    fn new() -> Self {
        Self(Array::new())
    }

    #[inline]
    fn with_capacity(cap: usize) -> Self {
        Self(Array::with_capacity(cap))
    }

    #[inline]
    fn len(&self) -> usize {
        self.len()
    }

    #[inline]
    fn on_value(&mut self, key: K, val: V) {
        self.0.on_value((key, val))
    }

    #[inline]
    fn on_complete(&mut self) {}
}

impl<K: Deref<Target = str>, V> Index<&str> for Object<K, V> {
    type Output = V;

    #[inline]
    fn index(&self, key: &str) -> &Self::Output {
        match self.get(key) {
            Some(v) => v,
            None => panic!("given key does not exist in the object"),
        }
    }
}

impl<K: Deref<Target = str>, V> IndexMut<&str> for Object<K, V> {
    #[inline]
    fn index_mut(&mut self, key: &str) -> &mut Self::Output {
        match self.get_mut(key) {
            Some(v) => v,
            None => panic!("given key does not exist in the object"),
        }
    }
}

impl<K: Debug, V: Debug> Debug for Object<K, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_map()
            .entries(self.as_slice().iter().map(|(k, v)| (k, v)))
            .finish()
    }
}
