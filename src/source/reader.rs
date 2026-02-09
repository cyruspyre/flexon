use std::{io::Read, slice::from_raw_parts_mut};

use super::{Source, Volatile};

const BUF_SIZE: usize = 800;

/// A minimal buffered wrapper for types implementing [`Read`].
pub struct Reader<R, const UTF8: bool> {
    reader: R,
    buf: Vec<u8>,
    offset: usize,
    cur: usize,
}

impl<R: Read> Reader<R, false> {
    /// Creates a new reader with UTF-8 validation.
    pub fn new(rdr: R) -> Self {
        Self {
            reader: rdr,
            buf: Vec::new(),
            offset: 0,
            cur: 0,
        }
    }
}

impl<R: Read> Reader<R, true> {
    /// Creates a new reader without UTF-8 validation.
    pub unsafe fn new_unchecked(rdr: R) -> Self {
        Self {
            reader: rdr,
            buf: Vec::new(),
            offset: 0,
            cur: 0,
        }
    }
}

impl<const UTF8: bool, R: Read> Source for Reader<R, UTF8> {
    const UTF8: bool = UTF8;
    const INSITU: bool = false;
    const NULL_PADDED: bool = false;

    type Volatility = Volatile;

    fn ptr(&mut self, offset: usize) -> *const u8 {
        self.cur = self.cur.max(offset);
        unsafe { self.buf.as_ptr().add(offset - self.offset) }
    }

    #[inline(always)]
    fn ptr_mut(&mut self, _: usize) -> *mut u8 {
        unimplemented!()
    }

    #[inline]
    fn trim(&mut self, until: usize) {
        if until - self.offset > BUF_SIZE {
            // ehh inefficient
            let offset = until - self.offset;
            let ptr = self.buf.as_mut_ptr();
            let new_len = self.buf.len() - offset;

            unsafe {
                ptr.copy_from(ptr.add(offset), new_len);
                self.buf.set_len(new_len);
                self.offset = until;
            }
        }
    }

    #[inline]
    fn len(&mut self) -> usize {
        if self.buf.len() + self.offset - self.cur <= BUF_SIZE {
            self.buf.reserve(BUF_SIZE);

            let start = self.buf.len();
            let buf = unsafe { from_raw_parts_mut(self.buf.as_mut_ptr().add(start), BUF_SIZE) };
            let count = self.reader.read(buf).unwrap();

            unsafe { self.buf.set_len(start + count) }
        }

        self.offset + self.buf.len()
    }
}
