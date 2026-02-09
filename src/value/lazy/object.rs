use core::{
    hint::unreachable_unchecked,
    ops::{Index, IndexMut},
};

use crate::{
    Parser,
    value::{
        borrowed::String,
        lazy::{Raw, Value},
    },
};

/// Represents a lazy JSON object.
///
/// This will not parse the elements until it is queried. Parsed elements are cached in memory for subsequent accesses.
#[derive(Debug)]
pub struct Object<'a> {
    pub(super) raw: &'a str,
    buf: Vec<(String<'a>, Value<'a>)>,
}

impl<'a> Object<'a> {
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

    /// Returns a mutable reference to the value associated with the given key, skipping and finding if necessary.
    pub fn get(&mut self, key: &str) -> Option<&mut Value<'a>> {
        for (k, v) in unsafe { &mut *(&mut self.buf as *mut Vec<(String, _)>) } {
            if **k == *key {
                return Some(v);
            }
        }

        let mut tmp = unsafe { Parser::new(self.raw.get_unchecked(1..)) };

        loop {
            match tmp.skip_whitespace() {
                b'"' => unsafe {
                    let new = tmp.string_unchecked2();
                    tmp.skip_whitespace(); // skip ':'
                    let wtf = tmp.skip_whitespace();

                    if &*new == key {
                        self.buf.push((
                            new,
                            Value::Raw(Raw(self.raw.get_unchecked(tmp.idx() + 1..))),
                        ));

                        return Some(&mut self.buf.last_mut().unwrap_unchecked().1);
                    }

                    match wtf {
                        b'"' => tmp.skip_string_unchecked(),
                        b'{' | b'[' => tmp.skip_container_unchecked(),
                        _ => tmp.skip_literal_unchecked(),
                    }
                },
                b',' => continue,
                b'}' => return None,
                _ => unsafe { unreachable_unchecked() },
            }
        }
    }

    /// Returns the actual number of elements by skipping and counting.
    pub fn actual_len(&self) -> usize {
        let mut count = 0;
        let mut tmp = unsafe { Parser::new(self.raw.get_unchecked(1..)) };

        loop {
            match tmp.skip_whitespace() {
                b'"' => {
                    count += 1;

                    tmp.skip_string_unchecked();
                    tmp.skip_whitespace(); // skip ':'
                    tmp.skip_value_unchecked();
                }
                b',' => continue,
                b'}' => return count,
                _ => unsafe { unreachable_unchecked() },
            }
        }
    }

    /// Returns the number of elements that have been parsed so far.
    #[inline]
    pub fn parsed_len(&self) -> usize {
        self.buf.len()
    }
}

impl<'a> Index<&str> for Object<'a> {
    type Output = Value<'a>;

    #[inline]
    fn index(&self, _: &str) -> &Self::Output {
        unimplemented!("lazy objects must be indexed mutably")
    }
}

impl<'a> IndexMut<&str> for Object<'a> {
    #[inline]
    fn index_mut(&mut self, key: &str) -> &mut Self::Output {
        match self.get(key) {
            Some(v) => v,
            _ => panic!("given key does not exist in the object"),
        }
    }
}
