use std::{
    io::Read,
    ops::{Bound, RangeBounds},
};

use crate::source::memchr_n;
#[allow(unused_imports)]
use crate::source::{Slice, Source};

/// A streaming source for the JSON parser over any [`Read`] type.
///
/// This is useful for large JSON inputs or files that cannot fit entirely in memory.
/// Prefer using [`Slice`] when possible, as reading from memory is faster
/// than reading from streams.
pub struct Reader<R: Read> {
    reader: R,
    buf: Vec<u8>,
    stamp: usize,
    len: usize,
}

impl<R: Read> Reader<R> {
    fn read(&mut self) -> usize {
        let mut buf = [0; 1024];
        let n = self.reader.read(&mut buf).unwrap();

        if n == 0 {
            return 0;
        }

        self.len += n;
        self.buf.extend_from_slice(&buf[..n]);

        n
    }
}

impl<'a, T: Read> Source for Reader<T> {
    const BORROWED: bool = false;

    #[inline]
    fn get(&self, index: usize) -> u8 {
        self.buf[index - self.stamp]
    }

    fn get_checked(&mut self, index: usize) -> u8 {
        if index == self.len {
            self.read();
        }

        match self.buf.get(index - self.stamp) {
            Some(v) => *v,
            _ => 0,
        }
    }

    fn slice<R: RangeBounds<usize>>(&self, range: R) -> &[u8] {
        let start = match range.start_bound() {
            Bound::Included(i) => i - self.stamp,
            _ => unreachable!(),
        };
        let end = match range.end_bound() {
            Bound::Included(i) => i + 1,
            Bound::Excluded(i) => *i,
            _ => unreachable!(),
        };

        &self.buf[start..end - self.stamp]
    }

    fn unbounded_search<const N: usize>(&mut self, needles: [u8; N], index: usize) -> usize {
        let mut local = index - self.stamp;

        loop {
            let tmp = memchr_n!(N, needles, &self.buf[local..]);

            if let Some(v) = tmp {
                return local + v + self.stamp;
            }

            local = self.buf.len();

            if self.read() == 0 {
                return 0;
            }
        }
    }

    fn stamp(&mut self, stamp: usize) {
        let tmp = stamp - self.stamp;

        self.buf.copy_within(tmp.., 0);
        unsafe { self.buf.set_len(self.buf.len() - tmp) };

        self.stamp = stamp;
    }

    fn hint(&mut self, index: usize) {
        if index >= self.len {
            self.read();
        }
    }

    fn len(&self) -> usize {
        self.len
    }
}

impl<R: Read> From<R> for Reader<R> {
    fn from(value: R) -> Self {
        Reader {
            reader: value,
            buf: Vec::new(),
            stamp: 0,
            len: 0,
        }
    }
}
