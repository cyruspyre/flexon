use crate::{misc::capacity_overflow, value::builder::ArrayBuilder};
use alloc::alloc::{alloc, dealloc, handle_alloc_error, realloc};
use core::{
    alloc::Layout,
    fmt::{Debug, Formatter, Result},
    ops::{Deref, DerefMut},
    ptr::{NonNull, slice_from_raw_parts_mut},
    slice::{from_raw_parts, from_raw_parts_mut},
};

/// Represents a JSON array.
pub struct Array<T> {
    buf: NonNull<T>,
    len: usize,
    cap: usize,
}

impl<T> ArrayBuilder<T> for Array<T> {
    #[inline]
    fn new() -> Self {
        Self {
            buf: NonNull::dangling(),
            len: 0,
            cap: 0,
        }
    }

    #[inline]
    fn with_capacity(cap: usize) -> Self {
        Self {
            buf: if cap != 0 {
                let Ok(layout) = Layout::array::<T>(cap) else {
                    capacity_overflow()
                };

                match unsafe { NonNull::new(alloc(layout).cast()) } {
                    Some(v) => v,
                    _ => handle_alloc_error(layout),
                }
            } else {
                NonNull::dangling()
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
    fn on_value(&mut self, val: T) {
        self.len += 1;

        if self.cap < self.len {
            if let Some(new_cap) = self.len.checked_mul(2)
                && let Ok(layout) = Layout::array::<T>(new_cap)
            {
                let new_buf = unsafe {
                    if self.cap != 0 {
                        realloc(
                            self.buf.as_ptr().cast(),
                            Layout::array::<T>(self.cap).unwrap_unchecked(),
                            layout.size(),
                        )
                    } else {
                        alloc(layout)
                    }
                };

                match NonNull::new(new_buf.cast()) {
                    Some(v) => {
                        self.cap = new_cap;
                        self.buf = v;
                    }
                    _ => handle_alloc_error(layout),
                }
            } else {
                capacity_overflow()
            }
        }

        unsafe { self.buf.add(self.len).sub(1).write(val) }
    }

    #[inline]
    fn on_complete(&mut self) {}
}

impl<T: PartialEq> PartialEq for Array<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.deref() == other.deref()
    }
}

impl<T: Eq> Eq for Array<T> {}

impl<T: Clone> Clone for Array<T> {
    fn clone(&self) -> Self {
        if self.len != 0 {
            unsafe {
                let layout = Layout::array::<T>(self.len).unwrap_unchecked();
                let Some(buf) = NonNull::new(alloc(layout).cast()) else {
                    handle_alloc_error(layout)
                };

                for i in 0..self.len {
                    buf.add(i).write(self.buf.add(i).as_ref().clone())
                }

                Self {
                    buf,
                    len: self.len,
                    cap: self.len,
                }
            }
        } else {
            Self {
                buf: NonNull::dangling(),
                len: 0,
                cap: 0,
            }
        }
    }
}

impl<T> Deref for Array<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { from_raw_parts(self.buf.as_ptr(), self.len) }
    }
}

impl<T> DerefMut for Array<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { from_raw_parts_mut(self.buf.as_ptr(), self.len) }
    }
}

impl<T: Debug> Debug for Array<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.deref().fmt(f)
    }
}

impl<T> Drop for Array<T> {
    fn drop(&mut self) {
        if self.cap != 0 {
            unsafe {
                slice_from_raw_parts_mut(self.buf.as_ptr(), self.len).drop_in_place();
                dealloc(
                    self.buf.as_ptr().cast(),
                    Layout::array::<T>(self.cap).unwrap_unchecked(),
                );
            }
        }
    }
}
