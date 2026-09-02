use crate::{
    misc::capacity_overflow,
    source::{NonVolatile, Source},
};
use alloc::alloc::{alloc, dealloc, handle_alloc_error};
use core::{alloc::Layout, ptr::NonNull};

/// Null padded buffer.
pub struct NullPadded {
    buf: NonNull<u8>,
    len: usize,
    cap: usize,
}

impl NullPadded {
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

    /// Creates a new null padded buffer from the given string slice. This will perform allocation.
    pub fn from_str(s: &str) -> Self {
        unsafe {
            let cap = s.len() + 64;
            let buf = Self::alloc(cap);

            buf.as_ptr().copy_from_nonoverlapping(s.as_ptr(), s.len());
            buf.add(s.len()).write_bytes(0, 64);

            Self {
                buf,
                cap,
                len: s.len(),
            }
        }
    }

    /// Writes the given string slice into the buffer.
    ///
    /// This will perform allocation only if the heap allocated buffer
    /// is too small for the string slice with extra 64 bytes padding.
    pub fn write_str(&mut self, s: &str) {
        let needed = s.len() + 64;
        if self.cap < needed {
            unsafe {
                self.dealloc();
                self.buf = Self::alloc(needed);
                self.cap = needed;
            }
        }

        self.len = s.len();
        unsafe {
            self.buf
                .as_ptr()
                .copy_from_nonoverlapping(s.as_ptr(), s.len());
            self.buf.add(s.len()).write_bytes(0, 64);
        }
    }

    unsafe fn alloc(n: usize) -> NonNull<u8> {
        let Ok(layout) = Layout::array::<u8>(n) else {
            capacity_overflow()
        };

        match NonNull::new(alloc(layout)) {
            Some(v) => v,
            _ => handle_alloc_error(layout),
        }
    }

    unsafe fn dealloc(&mut self) {
        if self.cap != 0 {
            dealloc(
                self.buf.as_ptr(),
                Layout::array::<u8>(self.cap).unwrap_unchecked(),
            )
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

impl Source for &NullPadded {
    const UTF8: bool = true;
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

impl Drop for NullPadded {
    fn drop(&mut self) {
        unsafe { self.dealloc() }
    }
}
