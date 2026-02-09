use std::alloc::{Layout, alloc, dealloc};

use super::{NonVolatile, Source};

/// Null padded buffer.
pub struct NullPadded {
    buf: *mut u8,
    len: usize,
}

impl NullPadded {
    /// Creates a new null padded buffer. This will not perform allocation.
    #[inline]
    pub fn new() -> Self {
        // this will be never mutated.
        // just a placeholder in case empty buffer is passed.
        static mut NULL: [u8; 64] = [0; 64];

        Self {
            #[allow(static_mut_refs)]
            buf: unsafe { NULL.as_mut_ptr() },
            len: 0,
        }
    }

    /// Creates a new null padded buffer from the given string slice. This will perform allocation.
    #[inline]
    pub fn from_str(s: &str) -> Self {
        let len = s.len() + 64;
        let buf = unsafe { alloc(Layout::array::<u8>(len).unwrap_unchecked()) };

        unsafe {
            buf.copy_from_nonoverlapping(s.as_ptr(), s.len());
            buf.add(s.len()).write_bytes(0, 64);
        }

        Self { buf, len }
    }

    /// Writes the given string slice into the buffer.
    ///
    /// This will perform allocation only if the buffer is too small for the
    /// string slice with extra 64 bytes padding.
    pub fn write_str(&mut self, s: &str) {
        let new_len = s.len() + 64;

        if self.len < new_len {
            unsafe {
                if self.len != 0 {
                    dealloc(self.buf, Layout::array::<u8>(self.len).unwrap_unchecked())
                }

                self.buf = alloc(Layout::array::<u8>(new_len).unwrap_unchecked());
                self.len = new_len;
            }
        }

        unsafe {
            self.buf.copy_from_nonoverlapping(s.as_ptr(), s.len());
            self.buf.add(s.len()).write_bytes(0, 64);
        }
    }
}

impl Source for &mut NullPadded {
    const UTF8: bool = true;
    const INSITU: bool = true;
    const NULL_PADDED: bool = true;

    type Volatility = NonVolatile;

    #[inline(always)]
    fn ptr(&mut self, offset: usize) -> *const u8 {
        unsafe { self.buf.add(offset) }
    }

    #[inline(always)]
    fn ptr_mut(&mut self, offset: usize) -> *mut u8 {
        unsafe { self.buf.add(offset) }
    }

    #[inline(always)]
    fn trim(&mut self, _: usize) {}

    #[inline(always)]
    fn len(&mut self) -> usize {
        self.len
    }
}

impl Source for &NullPadded {
    const UTF8: bool = true;
    const INSITU: bool = false;
    const NULL_PADDED: bool = true;

    type Volatility = NonVolatile;

    #[inline(always)]
    fn ptr(&mut self, offset: usize) -> *const u8 {
        unsafe { self.buf.add(offset) }
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

impl Drop for NullPadded {
    fn drop(&mut self) {
        if self.len != 0 {
            unsafe { dealloc(self.buf, Layout::array::<u8>(self.len).unwrap_unchecked()) }
        }
    }
}
