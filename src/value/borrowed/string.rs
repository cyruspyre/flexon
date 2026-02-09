use core::{
    alloc::Layout, hint::unreachable_unchecked, ptr::dangling_mut, slice::from_raw_parts,
    str::from_utf8_unchecked,
};
use std::alloc::{alloc, dealloc, realloc};

use crate::{
    Error,
    misc::likely,
    source::{NonVolatile, Source},
    value::{builder::*, misc::string_impl},
};

/// Represents a borrowed JSON string.
///
/// This will either borrow the string if it has no escape
/// characters, or store it in a heap allocated buffer otherwise.
#[repr(transparent)]
pub struct String<'a>(Inner<'a>);

enum Inner<'a> {
    Ref(&'a [u8]),
    Heap {
        buf: *mut u8,
        len: usize,
        cap: usize,
    },
}

impl<'a> String<'a> {
    #[inline]
    pub(crate) fn from_slice(s: &'a [u8]) -> String<'a> {
        String(Inner::Ref(s))
    }

    #[inline]
    pub(crate) fn from_raw_parts(buf: *mut u8, len: usize, cap: usize) -> Self {
        Self(Inner::Heap { buf, len, cap })
    }

    /// Returns itself as a string slice.
    #[inline]
    pub fn as_str(&'a self) -> &'a str {
        unsafe {
            from_utf8_unchecked(match self.0 {
                Inner::Ref(v) => v,
                Inner::Heap { buf, len, .. } => from_raw_parts(buf, len),
            })
        }
    }
}

impl<'a, S, E> StringBuilder<'a, S, E> for String<'a>
where
    S: Source<Volatility = NonVolatile>,
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
    fn on_chunk(&mut self, s: &'a [u8]) {
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
    fn on_final_chunk(&mut self, s: &'a [u8]) {
        let Inner::Heap { buf, len, cap } = &mut self.0 else {
            unsafe { unreachable_unchecked() }
        };

        if likely(*len == 0) {
            return self.0 = Inner::Ref(s);
        }

        let new_len = *len + s.len();

        if *cap < new_len {
            *buf = unsafe {
                let layout = Layout::array::<u8>(new_len).unwrap_unchecked();
                if *cap != 0 {
                    realloc(
                        *buf,
                        Layout::array::<u8>(*cap).unwrap_unchecked(),
                        Layout::array::<u8>(new_len).unwrap_unchecked().size(),
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
    fn on_complete(&mut self, _: &'a [u8]) -> Result<(), E> {
        Ok(())
    }
}

string_impl!(String<'a>, 'a);

impl Drop for String<'_> {
    fn drop(&mut self) {
        if let Inner::Heap { buf, cap, .. } = self.0
            && cap != 0
        {
            unsafe { dealloc(buf, Layout::array::<u8>(cap).unwrap_unchecked()) }
        }
    }
}
