use core::{alloc::Layout, hint::unreachable_unchecked, ptr::dangling_mut, slice::from_raw_parts};
use std::alloc::{alloc, dealloc, realloc};

use crate::{
    Error,
    misc::likely,
    source::Source,
    value::{builder::*, misc::string_impl},
};

/// Represents an owned JSON string.
///
/// This will either store the string on the stack if it does not have escape characters
/// and is less than or equal to 30 bytes, or use a heap allocated buffer otherwise.
#[repr(transparent)]
pub struct String(Inner);

enum Inner {
    Stack {
        buf: [u8; 30],
        len: u8,
    },
    Heap {
        buf: *mut u8,
        len: usize,
        cap: usize,
    },
}

impl String {
    /// Returns itself as a string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        let (ptr, len) = match self.0 {
            Inner::Stack { ref buf, len } => (buf.as_ptr(), len as _),
            Inner::Heap { buf, len, .. } => (buf.cast_const(), len),
        };

        unsafe { str::from_utf8_unchecked(from_raw_parts(ptr, len)) }
    }
}

impl<S, E> StringBuilder<'_, S, E> for String
where
    S: Source,
    E: ErrorBuilder,
{
    const REJECT_CTRL_CHAR: bool = true;
    const REJECT_INVALID_ESCAPE: bool = true;

    #[inline]
    fn new() -> Self {
        Self(Inner::Heap {
            buf: dangling_mut(),
            len: 0,
            cap: 0,
        })
    }

    #[inline]
    fn on_escape(&mut self, s: &[u8]) {
        unsafe {
            match &mut self.0 {
                Inner::Heap { buf, len, .. } => {
                    buf.add(*len).copy_from_nonoverlapping(s.as_ptr(), s.len());
                    *len += s.len();
                }
                _ => unreachable_unchecked(),
            }
        }
    }

    #[inline]
    fn on_chunk(&mut self, s: &[u8]) {
        let Inner::Heap { buf, len, cap } = &mut self.0 else {
            unsafe { unreachable_unchecked() }
        };
        let new_len = *len + s.len() + 4;

        if *cap < new_len {
            let tmp = new_len * 5 / 4;

            *buf = unsafe {
                let layout = Layout::array::<u8>(tmp).unwrap_unchecked();
                if *cap != 0 {
                    realloc(
                        *buf,
                        Layout::array::<u8>(*cap).unwrap_unchecked(),
                        layout.size(),
                    )
                } else {
                    alloc(layout)
                }
            };
            *cap = tmp;
        }

        unsafe { buf.add(*len).copy_from_nonoverlapping(s.as_ptr(), s.len()) }
        *len += s.len()
    }

    #[inline]
    fn on_final_chunk(&mut self, s: &[u8]) {
        let Inner::Heap { buf, len, cap } = &mut self.0 else {
            unsafe { unreachable_unchecked() }
        };

        if likely(*len == 0 && s.len() <= 30) {
            return unsafe {
                let mut buf = [0; 30];

                buf.as_mut_ptr()
                    .copy_from_nonoverlapping(s.as_ptr(), s.len());

                self.0 = Inner::Stack {
                    buf,
                    len: s.len() as _,
                }
            };
        }

        let new_len = *len + s.len();

        if *cap < new_len {
            *buf = unsafe {
                let layout = Layout::array::<u8>(new_len).unwrap_unchecked();
                if *cap != 0 {
                    realloc(
                        *buf,
                        Layout::array::<u8>(*cap).unwrap_unchecked(),
                        layout.size(),
                    )
                } else {
                    alloc(layout)
                }
            };
            *cap = new_len;
        }

        unsafe { buf.add(*len).copy_from_nonoverlapping(s.as_ptr(), s.len()) }
        *len = new_len
    }

    #[inline]
    fn apply_span(&mut self, _: usize, _: usize) {}

    #[inline]
    fn on_complete(&mut self, _: &[u8]) -> Result<(), E> {
        Ok(())
    }
}

string_impl!(String);

impl Drop for String {
    fn drop(&mut self) {
        if let Inner::Heap { buf, cap, .. } = self.0
            && cap != 0
        {
            unsafe { dealloc(buf, Layout::array::<u8>(cap).unwrap_unchecked()) }
        }
    }
}
