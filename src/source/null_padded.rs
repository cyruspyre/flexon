use crate::{
    misc::capacity_overflow,
    source::{NonVolatile, Source},
};
use alloc::alloc::{alloc, dealloc, handle_alloc_error, realloc};
use core::{
    alloc::Layout, ops::Deref, ptr::NonNull, slice::from_raw_parts, str::from_utf8_unchecked,
};
use std::io::{self, IoSlice, Write};

/// Null padded buffer.
pub struct NullPadded<const UTF8: bool> {
    buf: NonNull<u8>,
    len: usize,
    cap: usize,
}

impl<const UTF8: bool> NullPadded<UTF8> {
    /// Creates a new null padded buffer. This will not perform allocation.
    #[inline]
    pub fn new() -> Self {
        // this will be never mutated.
        // just a placeholder in case empty buffer is passed.
        static NULL: [u8; 64] = [0; 64];

        Self {
            buf: unsafe { NonNull::new_unchecked(NULL.as_ptr().cast_mut()) },
            len: 0,
            cap: 0,
        }
    }

    unsafe fn from_raw(ptr: *const u8, len: usize) -> Self {
        if len == 0 {
            return Self::new();
        }

        let Ok(layout) = Layout::array::<u8>(len + 64) else {
            capacity_overflow()
        };
        let buf = match NonNull::new(alloc(layout)) {
            Some(v) => v,
            _ => handle_alloc_error(layout),
        };

        buf.as_ptr().copy_from_nonoverlapping(ptr, len);
        buf.add(len).write_bytes(0, 64);

        Self { buf, len, cap: len }
    }

    unsafe fn write_raw(&mut self, ptr: *const u8, len: usize) {
        let new_len = self.len + len;

        if self.cap < new_len {
            // 3n / 2 + 64 = n + n / 2 + 64 < usize::MAX (won't overflow)
            let new_cap = new_len + new_len / 2;
            let Ok(layout) = Layout::array::<u8>(new_cap + 64) else {
                capacity_overflow()
            };
            let raw = match self.cap {
                0 => alloc(layout),
                _ => realloc(
                    self.buf.as_ptr(),
                    Layout::array::<u8>(self.cap + 64).unwrap_unchecked(),
                    layout.size(),
                ),
            };

            self.buf = match NonNull::new(raw) {
                Some(new_buf) => new_buf,
                _ => handle_alloc_error(layout),
            };
            self.buf.add(new_len).write_bytes(0, new_cap - new_len + 64);
            self.cap = new_cap;
        }

        self.buf
            .as_ptr()
            .add(self.len)
            .copy_from_nonoverlapping(ptr, len);
        self.len = new_len;
    }

    /// Returns the number of bytes currently stored in the buffer, excluding padding.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }
}

impl NullPadded<true> {
    /// Creates a new null padded buffer from the given string slice.
    #[inline]
    pub fn from_str(v: &str) -> Self {
        unsafe { Self::from_raw(v.as_ptr(), v.len()) }
    }

    /// Appends the given string slice into the buffer.
    #[inline]
    pub fn write_str(&mut self, v: &str) {
        unsafe { self.write_raw(v.as_ptr(), v.len()) }
    }
}

impl NullPadded<false> {
    /// Creates a new null padded buffer from the given byte slice.
    #[inline]
    pub fn from_bytes(v: &[u8]) -> Self {
        unsafe { Self::from_raw(v.as_ptr(), v.len()) }
    }

    /// Appends the given byte slice into the buffer.
    #[inline]
    pub fn write_bytes(&mut self, v: &[u8]) {
        unsafe { self.write_raw(v.as_ptr(), v.len()) }
    }
}

impl Write for NullPadded<false> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_bytes(buf);
        Ok(buf.len())
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        let mut count = 0;

        for v in bufs {
            self.write_bytes(v);
            count += v.len();
        }

        Ok(count)
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.write_bytes(buf);
        Ok(())
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Deref for NullPadded<true> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { from_utf8_unchecked(from_raw_parts(self.buf.as_ptr(), self.len)) }
    }
}

impl Deref for NullPadded<false> {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { from_raw_parts(self.buf.as_ptr(), self.len) }
    }
}

impl<const UTF8: bool> Source for &mut NullPadded<UTF8> {
    const UTF8: bool = UTF8;
    const INSITU: bool = true;
    const NULL_PADDED: bool = true;

    type Volatility = NonVolatile;

    #[inline(always)]
    fn ptr(&mut self, offset: usize) -> *const u8 {
        unsafe { self.buf.add(offset).as_ptr() }
    }

    #[inline(always)]
    fn ptr_mut(&mut self, offset: usize) -> *mut u8 {
        unsafe { self.buf.add(offset).as_ptr() }
    }

    #[inline(always)]
    fn trim(&mut self, _: usize) {}

    #[inline(always)]
    fn len(&mut self) -> usize {
        self.len
    }
}

impl<const UTF8: bool> Source for &NullPadded<UTF8> {
    const UTF8: bool = UTF8;
    const INSITU: bool = false;
    const NULL_PADDED: bool = true;

    type Volatility = NonVolatile;

    #[inline(always)]
    fn ptr(&mut self, offset: usize) -> *const u8 {
        unsafe { self.buf.add(offset).as_ptr() }
    }

    #[inline(always)]
    fn ptr_mut(&mut self, _: usize) -> *mut u8 {
        unimplemented!()
    }

    #[inline(always)]
    fn trim(&mut self, _: usize) {}

    #[inline(always)]
    fn len(&mut self) -> usize {
        self.len
    }
}

impl<const UTF8: bool> Drop for NullPadded<UTF8> {
    fn drop(&mut self) {
        if self.cap != 0 {
            unsafe {
                dealloc(
                    self.buf.as_ptr(),
                    Layout::array::<u8>(self.cap + 64).unwrap_unchecked(),
                )
            }
        }
    }
}
