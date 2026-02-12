use core::{
    alloc::Layout,
    fmt::{Debug, Display, Formatter, Result},
    marker::PhantomData,
    slice::from_raw_parts,
    str::from_utf8_unchecked,
};
use std::alloc::{alloc, dealloc};

/// Represents JSON comments in JSONC.
pub struct Comment<'a> {
    owned: bool,
    multi: bool,
    buf: *mut u8,
    len: usize,
    #[cfg(feature = "span")]
    span: [usize; 2],
    __: PhantomData<&'a ()>,
}

impl<'a> Comment<'a> {
    #[inline]
    pub(crate) fn new(
        src: *mut u8,
        len: usize,
        multi: bool,
        owned: bool,
        #[cfg(feature = "span")] span: [usize; 2],
    ) -> Self {
        Self {
            buf: match owned {
                true => unsafe {
                    // no need to check for `len != 0` as owned is true
                    // only when the source is volatile and the comment is non empty.
                    let tmp = alloc(Layout::array::<u8>(len).unwrap_unchecked());
                    tmp.copy_from_nonoverlapping(src, len);
                    tmp
                },
                _ => src,
            },
            len,
            owned,
            multi,
            #[cfg(feature = "span")]
            span,
            __: PhantomData,
        }
    }

    /// Returns the byte length of the comment.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the comment is a multiline comment or single-line.
    #[inline]
    pub fn is_multiline(&self) -> bool {
        self.multi
    }

    /// Returns the comment as a string slice.
    #[inline]
    pub fn as_str(&'a self) -> &'a str {
        unsafe { from_utf8_unchecked(from_raw_parts(self.buf, self.len)) }
    }

    /// Returns the starting byte offset of the comment.
    #[inline]
    #[cfg(feature = "span")]
    pub fn start(&'a self) -> usize {
        self.span[0]
    }

    /// Returns the ending byte offset of the comment.
    ///
    /// In case of single-line comment, it does not include `\n`.
    #[inline]
    #[cfg(feature = "span")]
    pub fn end(&'a self) -> usize {
        self.span[1]
    }

    /// Consumes the comment and converts it into [`String`].
    pub fn into_string(self) -> String {
        unsafe {
            // there might be cases like `/**/` where the comment is empty.
            // so we check length here to avoid allocating 0 bytes.
            let buf = if self.owned || self.len == 0 {
                self.buf
            } else {
                let tmp = alloc(Layout::array::<u8>(self.len).unwrap_unchecked());
                tmp.copy_from_nonoverlapping(self.buf, self.len);
                tmp
            };

            String::from_raw_parts(buf, self.len, self.len)
        }
    }
}

impl Debug for Comment<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Debug::fmt(self.as_str(), f)
    }
}

impl Display for Comment<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Display::fmt(self.as_str(), f)
    }
}

impl Drop for Comment<'_> {
    fn drop(&mut self) {
        if self.owned {
            // no need to check for `len != 0` as owned is true
            // only when the source is volatile and the comment is non empty.
            unsafe { dealloc(self.buf, Layout::array::<u8>(self.len).unwrap_unchecked()) }
        }
    }
}
