use crate::{
    Error,
    misc::{capacity_overflow, likely},
    source::Source,
    value::{builder::*, misc::string_impl},
};
use alloc::alloc::{alloc, dealloc, handle_alloc_error, realloc};
use core::{alloc::Layout, hint::unreachable_unchecked, ptr::dangling_mut, slice::from_raw_parts};

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
    unsafe fn on_escape(&mut self, s: &[u8]) {
        match &mut self.0 {
            Inner::Heap { buf, len, .. } => {
                buf.add(*len).copy_from_nonoverlapping(s.as_ptr(), s.len());
                *len += s.len();
            }
            _ => unreachable_unchecked(),
        }
    }

    #[inline]
    unsafe fn on_chunk(&mut self, s: &[u8]) {
        let Inner::Heap { buf, len, cap } = &mut self.0 else {
            unreachable_unchecked()
        };
        let mut new_len = *len + s.len() + 4;

        if *cap < new_len {
            new_len += new_len / 4;
            let Ok(layout) = Layout::array::<u8>(new_len) else {
                capacity_overflow()
            };
            let new_buf = if *cap != 0 {
                realloc(
                    *buf,
                    Layout::array::<u8>(*cap).unwrap_unchecked(),
                    layout.size(),
                )
            } else {
                alloc(layout)
            };

            if new_buf.is_null() {
                handle_alloc_error(layout)
            }

            *buf = new_buf;
            *cap = new_len;
        }

        buf.add(*len).copy_from_nonoverlapping(s.as_ptr(), s.len());
        *len += s.len()
    }

    #[inline]
    unsafe fn on_final_chunk(&mut self, s: &[u8]) {
        let Inner::Heap { buf, len, cap } = &mut self.0 else {
            unreachable_unchecked()
        };

        if likely(*len == 0 && s.len() <= 30) {
            let mut buf = [0; 30];

            buf.as_mut_ptr()
                .copy_from_nonoverlapping(s.as_ptr(), s.len());
            return self.0 = Inner::Stack {
                buf,
                len: s.len() as _,
            };
        }

        let new_len = *len + s.len();

        if *cap < new_len {
            let layout = Layout::array::<u8>(new_len).unwrap_unchecked();
            let new_buf = if *cap != 0 {
                realloc(
                    *buf,
                    Layout::array::<u8>(*cap).unwrap_unchecked(),
                    layout.size(),
                )
            } else {
                alloc(layout)
            };

            if new_buf.is_null() {
                handle_alloc_error(layout)
            }

            *buf = new_buf;
            *cap = new_len;
        }

        buf.add(*len).copy_from_nonoverlapping(s.as_ptr(), s.len());
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

impl From<&str> for String {
    #[inline]
    fn from(value: &str) -> Self {
        let src = value.as_ptr();
        let len = value.len();

        String(unsafe {
            if len <= 30 {
                let mut buf = [0; 30];
                buf.as_mut_ptr().copy_from_nonoverlapping(src, len);
                Inner::Stack { buf, len: len as _ }
            } else {
                let layout = Layout::array::<u8>(len).unwrap_unchecked();
                let buf = alloc(layout);

                if buf.is_null() {
                    handle_alloc_error(layout)
                }

                buf.copy_from_nonoverlapping(src, len);
                Inner::Heap { buf, len, cap: len }
            }
        })
    }
}

#[cfg(feature = "alloc")]
impl From<alloc::string::String> for String {
    #[inline]
    fn from(value: alloc::string::String) -> Self {
        let (buf, len, cap) = value.into_raw_parts();
        Self(Inner::Heap { buf, len, cap })
    }
}

#[cfg(feature = "alloc")]
impl From<String> for alloc::string::String {
    #[inline]
    fn from(value: String) -> Self {
        use alloc::string::String;

        unsafe {
            match value.0 {
                Inner::Stack { buf, len } => {
                    String::from_utf8_unchecked(buf.get_unchecked(..len as usize).into())
                }
                Inner::Heap { buf, len, cap } => String::from_raw_parts(buf, len, cap),
            }
        }
    }
}

impl Clone for String {
    #[inline]
    fn clone(&self) -> Self {
        match self.0 {
            Inner::Stack { buf, len } => Self(Inner::Stack { buf, len }),
            Inner::Heap { buf: src, len, .. } => Self(if len != 0 {
                unsafe {
                    let layout = Layout::array::<u8>(len).unwrap_unchecked();
                    let buf = alloc(layout);

                    if buf.is_null() {
                        handle_alloc_error(layout)
                    }

                    buf.copy_from_nonoverlapping(src, len);
                    Inner::Heap { buf, len, cap: len }
                }
            } else {
                Inner::Heap {
                    buf: dangling_mut(),
                    len: 0,
                    cap: 0,
                }
            }),
        }
    }
}

impl Drop for String {
    fn drop(&mut self) {
        if let Inner::Heap { buf, cap, .. } = self.0
            && cap != 0
        {
            unsafe { dealloc(buf, Layout::array::<u8>(cap).unwrap_unchecked()) }
        }
    }
}
