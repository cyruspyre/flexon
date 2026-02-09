use core::{
    alloc::Layout,
    fmt::{Debug, Formatter, Result},
    ops::{Deref, Index, IndexMut},
    ptr::{dangling_mut, slice_from_raw_parts_mut},
    slice::from_raw_parts_mut,
};
use std::alloc::{alloc, dealloc, realloc};

use crate::{source::Source, value::builder::*};

/// Represents a JSON object.
///
/// As of right now it stores elements as an array of key-value pairs. The original JSON source order is preserved.
pub struct Object<K, V> {
    buf: *mut (K, V),
    len: usize,
    cap: usize,
}

impl<K, V> Object<K, V> {
    #[inline]
    fn as_slice(&self) -> &mut [(K, V)] {
        unsafe { from_raw_parts_mut(self.buf, self.len) }
    }

    /// Returns the number of elements.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
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
    pub fn get_mut(&self, key: &str) -> Option<&mut V> {
        for (k, v) in self.as_slice() {
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
        for (k, v) in self.as_slice() {
            if &**k == key {
                return Some((k, v));
            }
        }

        None
    }
}

impl<'a, S, K, V, E> ObjectBuilder<'a, S, E> for Object<K, V>
where
    S: Source,
    K: StringBuilder<'a, S, E>,
    V: ValueBuilder<'a, S, Error = E>,
    E: ErrorBuilder,
{
    type Key = K;
    type Value = V;

    #[inline]
    fn new() -> Self {
        Self {
            buf: dangling_mut(),
            len: 0,
            cap: 0,
        }
    }

    #[inline]
    fn with_capacity(cap: usize) -> Self {
        Self {
            buf: if cap != 0 {
                unsafe { alloc(Layout::array::<(K, V)>(cap).unwrap_unchecked()).cast() }
            } else {
                dangling_mut()
            },
            len: 0,
            cap,
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.len
    }

    #[inline]
    fn on_value(&mut self, key: K, val: V) {
        self.len += 1;

        if self.cap < self.len {
            let new_cap = self.len * 2;

            self.buf = unsafe {
                let new_layout = Layout::array::<(K, V)>(new_cap).unwrap_unchecked();

                if self.cap != 0 {
                    realloc(
                        self.buf.cast(),
                        Layout::array::<(K, V)>(self.cap).unwrap_unchecked(),
                        new_layout.size(),
                    )
                } else {
                    alloc(new_layout)
                }
                .cast()
            };
            self.cap = new_cap;
        }

        unsafe { self.buf.add(self.len - 1).write((key, val)) }
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

impl<K, V> Drop for Object<K, V> {
    fn drop(&mut self) {
        if self.cap != 0 {
            unsafe {
                slice_from_raw_parts_mut(self.buf, self.len).drop_in_place();
                dealloc(
                    self.buf.cast(),
                    Layout::array::<(K, V)>(self.cap).unwrap_unchecked(),
                );
            }
        }
    }
}
