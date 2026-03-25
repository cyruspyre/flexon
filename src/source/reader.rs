use core::{alloc::Layout, ptr::dangling_mut, slice::from_raw_parts_mut};
use std::{
    alloc::{alloc_zeroed, dealloc, realloc},
    io::Read,
};

use super::{Source, Volatile};

/// A minimal buffered wrapper for types implementing [`Read`].
pub struct Reader<const UTF8: bool, R> {
    reader: R,
    buf: *mut u8,
    len: usize,
    cap: usize,
    max_recent: usize,
    offset: usize,
}

impl<R> Reader<false, R> {
    /// Creates a new reader with UTF-8 validation.
    #[inline]
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buf: dangling_mut(),
            len: 0,
            cap: 0,
            max_recent: 0,
            offset: 0,
        }
    }
}

impl<R> Reader<true, R> {
    /// Creates a new reader without UTF-8 validation.
    #[inline]
    pub unsafe fn new_unchecked(reader: R) -> Self {
        Self {
            reader,
            buf: dangling_mut(),
            len: 0,
            cap: 0,
            max_recent: 0,
            offset: 0,
        }
    }
}

impl<const UTF8: bool, R: Read> Source for Reader<UTF8, R> {
    const UTF8: bool = UTF8;
    const INSITU: bool = false;
    const NULL_PADDED: bool = false;

    type Volatility = Volatile;

    fn ptr(&mut self, offset: usize) -> *const u8 {
        self.max_recent = offset.max(self.max_recent);
        unsafe { self.buf.add(offset - self.offset) }
    }

    fn ptr_mut(&mut self, _: usize) -> *mut u8 {
        unimplemented!()
    }

    fn trim(&mut self, until: usize) {
        let remaining = self.len - (until - self.offset);

        if remaining <= 128 {
            unsafe {
                let old = self.len - remaining;

                self.buf.add(old).copy_to(self.buf, remaining);
                self.offset += old;
                self.len = remaining;
                self.max_recent = until;
            }
        }
    }

    fn len(&mut self) -> usize {
        if self.len - (self.max_recent - self.offset) < 64 {
            unsafe {
                if self.cap - self.len < 1024 {
                    if self.cap != 0 {
                        let new = self.cap * 2;
                        let old = Layout::array::<u8>(self.cap).unwrap_unchecked();

                        self.buf = realloc(self.buf, old, new);
                        self.buf.add(self.cap).write_bytes(0, new - self.cap);
                        self.cap = new;
                    } else {
                        self.buf = alloc_zeroed(Layout::array::<u8>(1024).unwrap_unchecked());
                        self.cap = 1024;
                    }
                }

                self.len += self
                    .reader
                    .read(from_raw_parts_mut(
                        self.buf.add(self.len),
                        self.cap - self.len,
                    ))
                    .unwrap();
            }
        }

        self.len + self.offset
    }
}

impl<const UTF8: bool, R> Drop for Reader<UTF8, R> {
    fn drop(&mut self) {
        if self.cap != 0 {
            unsafe { dealloc(self.buf, Layout::array::<u8>(self.cap).unwrap_unchecked()) }
        }
    }
}
