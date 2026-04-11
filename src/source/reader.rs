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
    recent: usize,
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
            recent: 0,
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
            recent: 0,
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
        self.recent = offset;
        unsafe { self.buf.add(offset - self.offset) }
    }

    fn ptr_mut(&mut self, _: usize) -> *mut u8 {
        unimplemented!()
    }

    fn trim(&mut self, until: usize) {
        #[inline(never)]
        unsafe fn shift<const V: bool, R: Read>(
            this: &mut Reader<V, R>,
            remaining: usize,
            until: usize,
        ) {
            let old = this.len - remaining;

            this.buf.add(old).copy_to(this.buf, remaining);
            this.offset += old;
            this.len = remaining;
            this.recent = until;
        }

        let remaining = self.len - (until - self.offset);
        if remaining <= 128 {
            unsafe { shift(self, remaining, until) }
        }
    }

    fn len(&mut self) -> usize {
        #[inline(never)]
        unsafe fn load<const V: bool, R: Read>(this: &mut Reader<V, R>) {
            if this.cap - this.len < 1024 {
                if this.cap != 0 {
                    let new = this.cap * 2;
                    let old = Layout::array::<u8>(this.cap).unwrap_unchecked();

                    this.buf = realloc(this.buf, old, new);
                    this.buf.add(this.cap).write_bytes(0, new - this.cap);
                    this.cap = new;
                } else {
                    this.buf = alloc_zeroed(Layout::array::<u8>(1024).unwrap_unchecked());
                    this.cap = 1024;
                }
            }

            this.len += this
                .reader
                .read(from_raw_parts_mut(
                    this.buf.add(this.len),
                    this.cap - this.len,
                ))
                .unwrap();
        }

        if self.len - (self.recent - self.offset) < 64 {
            unsafe { load(self) }
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
