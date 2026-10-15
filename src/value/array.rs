use crate::{misc::capacity_overflow, value::builder::ArrayBuilder};
use alloc::alloc::{Layout, alloc, dealloc, handle_alloc_error, realloc};
use core::{
    fmt::{Debug, Formatter, Result},
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    ptr::{NonNull, slice_from_raw_parts_mut},
    slice::{from_raw_parts, from_raw_parts_mut},
};

/// Represents a JSON array.
pub struct Array<T> {
    buf: NonNull<T>,
    len: usize,
    // `cap` is non-zero ONLY when `Self` actually allocated something
    cap: usize,
}

impl<T> Array<T> {
    const ZST: bool = size_of::<T>() == 0;

    /// Creates an empty array.
    #[inline]
    pub const fn new() -> Self {
        Self {
            buf: NonNull::dangling(),
            len: 0,
            cap: 0,
        }
    }
}

impl<T> ArrayBuilder<T> for Array<T> {
    #[inline]
    fn new() -> Self {
        Self::new()
    }

    #[inline]
    fn with_capacity(cap: usize) -> Self {
        let Ok(layout) = Layout::array::<T>(cap) else {
            capacity_overflow()
        };
        let (buf, cap) = match layout.size() {
            0 => (NonNull::dangling(), 0),
            _ => match unsafe { NonNull::new(alloc(layout).cast()) } {
                Some(v) => (v, cap),
                _ => handle_alloc_error(layout),
            },
        };

        Self { buf, cap, len: 0 }
    }

    #[inline]
    fn len(&self) -> usize {
        self.len
    }

    #[inline]
    fn on_value(&mut self, val: T) {
        let new_len = match self.len.overflowing_add(1) {
            (_, true) if Self::ZST => capacity_overflow(),
            (v, _) => v, // won't ever overflow for non zst
        };

        if !Self::ZST && self.cap < new_len {
            if let Some(new_cap) = new_len.checked_mul(2)
                && let Ok(layout) = Layout::array::<T>(new_cap)
            {
                let raw = match self.cap {
                    0 => unsafe { alloc(layout) },
                    _ => unsafe {
                        realloc(
                            self.buf.as_ptr().cast(),
                            Layout::array::<T>(self.cap).unwrap_unchecked(),
                            layout.size(),
                        )
                    },
                };

                match NonNull::new(raw.cast()) {
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

        unsafe { self.buf.add(self.len).write(val) }
        self.len += 1;
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

impl<T, const N: usize> From<[T; N]> for Array<T> {
    fn from(val: [T; N]) -> Self {
        let val = ManuallyDrop::new(val);
        let Ok(layout) = Layout::array::<T>(N) else {
            capacity_overflow()
        };
        let (buf, cap) = match layout.size() {
            0 => (NonNull::dangling(), 0),
            _ => unsafe {
                let Some(buf) = NonNull::new(alloc(layout).cast::<T>()) else {
                    handle_alloc_error(layout)
                };

                buf.as_ptr().copy_from_nonoverlapping(val.as_ptr(), N);
                (buf, N)
            },
        };

        Self { buf, cap, len: N }
    }
}

impl<T> Default for Array<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Clone for Array<T> {
    fn clone(&self) -> Self {
        let layout = unsafe { Layout::array::<T>(self.len).unwrap_unchecked() };
        let (buf, cap) = match layout.size() {
            0 => (NonNull::dangling(), 0),
            _ => match unsafe { NonNull::new(alloc(layout).cast()) } {
                Some(v) => (v, self.len),
                _ => handle_alloc_error(layout),
            },
        };
        let mut new = Self { buf, cap, len: 0 };

        for i in 0..self.len {
            unsafe {
                let tmp = self.buf.add(i).as_ref().clone();
                new.buf.add(i).write(tmp);
            }
            new.len += 1;
        }

        new
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
        unsafe {
            slice_from_raw_parts_mut(self.buf.as_ptr(), self.len).drop_in_place();

            if self.cap != 0 {
                dealloc(
                    self.buf.as_ptr().cast(),
                    Layout::array::<T>(self.cap).unwrap_unchecked(),
                );
            }
        }
    }
}
