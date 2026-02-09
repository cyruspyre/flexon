use core::ops::{Index, IndexMut};

use crate::{
    Parser,
    value::lazy::{Raw, Value},
};

/// Represents a lazy JSON array.
///
/// This will not parse the elements until it is queried. Parsed elements are cached in memory for subsequent accesses.
#[derive(Debug)]
pub struct Array<'a> {
    pub(super) raw: &'a str,
    buf: Vec<(usize, Value<'a>)>,
}

impl<'a> Array<'a> {
    #[inline]
    pub(super) fn new(s: &'a str) -> Self {
        Self {
            raw: s,
            buf: Vec::new(),
        }
    }

    /// Returns the raw JSON object.
    #[inline]
    pub fn raw(&self) -> Raw<'a> {
        Raw(self.raw)
    }

    /// Returns a mutable reference to the value at the given index, skipping and finding if necessary.
    pub fn get(&mut self, mut idx: usize) -> Option<&mut Value<'a>> {
        for (i, v) in unsafe { &mut *(&mut self.buf as *mut Vec<_>) } {
            if *i == idx {
                return Some(v);
            }
        }

        let mut tmp = unsafe { Parser::new(self.raw.get_unchecked(1..)) };

        loop {
            match tmp.skip_whitespace() {
                b',' => continue,
                b']' => return None,
                _ if idx == 0 => unsafe {
                    self.buf.push((
                        idx,
                        Value::Raw(Raw(self.raw.get_unchecked(tmp.idx() + 1..))),
                    ));

                    return Some(&mut self.buf.last_mut().unwrap_unchecked().1);
                },
                v => unsafe {
                    idx -= 1;
                    match v {
                        b'"' => tmp.skip_string_unchecked(),
                        b'{' | b'[' => tmp.skip_container_unchecked(),
                        _ => tmp.skip_literal_unchecked(),
                    }
                },
            }
        }
    }

    /// Returns the actual number of elements by skipping and counting.
    pub fn actual_len(&self) -> usize {
        let mut count = 0;
        let mut tmp = unsafe { Parser::new(self.raw.get_unchecked(1..)) };

        loop {
            match tmp.skip_whitespace() {
                b',' => continue,
                b']' => return count,
                v => unsafe {
                    count += 1;
                    match v {
                        b'"' => tmp.skip_string_unchecked(),
                        b'{' | b'[' => tmp.skip_container_unchecked(),
                        _ => tmp.skip_literal_unchecked(),
                    }
                },
            }
        }
    }

    /// Returns the number of elements that have been parsed so far.
    #[inline]
    pub fn parsed_len(&self) -> usize {
        self.buf.len()
    }
}

impl<'a> Index<usize> for Array<'a> {
    type Output = Value<'a>;

    #[inline]
    fn index(&self, _: usize) -> &Self::Output {
        unimplemented!("lazy arrays must be indexed mutably")
    }
}

impl<'a> IndexMut<usize> for Array<'a> {
    #[inline]
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        match self.get(idx) {
            Some(v) => v,
            _ => panic!("given index does not exist in the array"),
        }
    }
}
