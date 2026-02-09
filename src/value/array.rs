use core::{
    alloc::Layout,
    fmt::{Debug, Formatter, Result},
    ops::{Deref, DerefMut},
    ptr::{dangling_mut, slice_from_raw_parts_mut},
    slice::{from_raw_parts, from_raw_parts_mut},
};
use std::alloc::{alloc, dealloc, realloc};

use crate::value::builder::ArrayBuilder;

/// Represents a JSON array.
pub struct Array<V> {
    buf: *mut V,
    len: usize,
    cap: usize,
}

impl<V> ArrayBuilder<V> for Array<V> {
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
                unsafe { alloc(Layout::array::<V>(cap).unwrap_unchecked()).cast() }
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
    fn on_value(&mut self, val: V) {
        self.len += 1;

        if self.cap < self.len {
            let new_cap = self.len * 2;

            self.buf = unsafe {
                let layout = Layout::array::<V>(new_cap).unwrap_unchecked();
                if self.cap != 0 {
                    realloc(
                        self.buf.cast(),
                        Layout::array::<V>(self.cap).unwrap_unchecked(),
                        layout.size(),
                    )
                } else {
                    alloc(layout)
                }
                .cast()
            };
            self.cap = new_cap;
        }

        unsafe { self.buf.add(self.len - 1).write(val) }
    }

    #[inline]
    fn on_complete(&mut self) {}
}

impl<V> Deref for Array<V> {
    type Target = [V];

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { from_raw_parts(self.buf, self.len) }
    }
}

impl<V> DerefMut for Array<V> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { from_raw_parts_mut(self.buf, self.len) }
    }
}

impl<V: Debug> Debug for Array<V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.deref().fmt(f)
    }
}

impl<V> Drop for Array<V> {
    fn drop(&mut self) {
        if self.cap != 0 {
            unsafe {
                slice_from_raw_parts_mut(self.buf, self.len).drop_in_place();
                dealloc(
                    self.buf.cast(),
                    Layout::array::<V>(self.cap).unwrap_unchecked(),
                );
            }
        }
    }
}
