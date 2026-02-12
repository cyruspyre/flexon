use core::{alloc::Layout, hint::unreachable_unchecked, ptr::dangling_mut, slice::from_raw_parts};
use std::alloc::{alloc, dealloc, realloc};

use crate::{
    Parser,
    config::Config,
    misc::ESC_LUT,
    pointer::JsonPointer,
    source::{Source, Volatility},
    value::{borrowed::String, builder::ErrorBuilder},
};

impl<'a, S: Source, C: Config> Parser<'a, S, C> {
    #[inline(always)]
    pub(crate) fn skip_to<E, P>(&mut self, p: P) -> Result<u8, E>
    where
        E: ErrorBuilder,
        P: IntoIterator,
        P::Item: JsonPointer,
    {
        let mut char = self.skip_whitespace();

        'main: for pointer in p {
            #[allow(unused_mut)]
            let mut err = if let Some(key) = pointer.as_key()
                && char == b'{'
            {
                #[cfg(feature = "span")]
                let start = self.idx();
                char = self.skip_whitespace();

                if char == b'}' {
                    let mut tmp = E::expected_value();

                    #[cfg(feature = "span")]
                    tmp.apply_span(start, self.idx());
                    return Err(tmp);
                }

                loop {
                    if char != b'"' {
                        break E::unexpected_token();
                    }

                    let new = unsafe { self.string2()? };
                    if self.skip_whitespace() != b':' {
                        break E::expected_colon();
                    }

                    char = self.skip_whitespace();
                    if &*new == key {
                        continue 'main;
                    }

                    match char {
                        b'"' => self.skip_string(),
                        b'{' => self.skip_object(),
                        b'[' => self.skip_array(),
                        _ => unsafe { self.skip_literal() },
                    }?;

                    char = self.skip_whitespace();
                    let comma = char == b',';
                    if comma {
                        char = self.skip_whitespace();
                    }

                    if char == b'}' {
                        if !comma || self.cfg.trailing_comma() {
                            let mut tmp = E::expected_value();

                            #[cfg(feature = "span")]
                            tmp.apply_span(start, self.idx());
                            return Err(tmp);
                        } else {
                            #[cfg(feature = "span")]
                            self.dec();
                            break E::trailing_comma();
                        }
                    }

                    if comma || self.cfg.comma() {
                        continue;
                    }

                    break match char {
                        0 => E::eof(),
                        _ => E::unexpected_token(),
                    };
                }
            } else if let Some(mut idx) = pointer.as_index()
                && char == b'['
            {
                #[cfg(feature = "span")]
                let start = self.idx();
                char = self.skip_whitespace();

                if char == b']' {
                    let mut tmp = E::expected_value();

                    #[cfg(feature = "span")]
                    tmp.apply_span(start, self.idx());
                    return Err(tmp);
                }

                loop {
                    if idx == 0 {
                        continue 'main;
                    }

                    idx -= 1;
                    match char {
                        b'"' => self.skip_string(),
                        b'{' => self.skip_object(),
                        b'[' => self.skip_array(),
                        _ => unsafe { self.skip_literal() },
                    }?;

                    char = self.skip_whitespace();
                    let comma = char == b',';
                    if comma {
                        char = self.skip_whitespace();
                    }

                    if char == b']' {
                        if !comma || self.cfg.trailing_comma() {
                            let mut tmp = E::expected_value();

                            #[cfg(feature = "span")]
                            tmp.apply_span(start, self.idx());
                            return Err(tmp);
                        } else {
                            #[cfg(feature = "span")]
                            self.dec();
                            break E::trailing_comma();
                        }
                    }

                    if comma || self.cfg.comma() {
                        continue;
                    }

                    break match char {
                        0 => E::eof(),
                        _ => E::unexpected_token(),
                    };
                }
            } else {
                match char {
                    0 => E::eof(),
                    _ => E::unexpected_token(),
                }
            };

            #[cfg(feature = "span")]
            err.apply_span(self.idx(), self.idx());
            return Err(err);
        }

        Ok(char)
    }

    #[inline(always)]
    pub(crate) unsafe fn skip_to_unchecked<P>(&mut self, p: P) -> u8
    where
        P: IntoIterator,
        P::Item: JsonPointer,
    {
        let mut char = self.skip_whitespace();

        'main: for pointer in p {
            if let Some(key) = pointer.as_key() {
                loop {
                    self.skip_whitespace(); // skip '"'
                    let new = self.string_unchecked2();
                    self.skip_whitespace(); // skip ':'
                    char = self.skip_whitespace();

                    if &*new == key {
                        continue 'main;
                    }

                    match char {
                        b'"' => self.skip_string_unchecked(),
                        b'{' | b'[' => self.skip_container_unchecked(),
                        _ => self.skip_literal_unchecked(),
                    }
                    self.skip_whitespace(); // skip ','
                }
            }

            let Some(mut idx) = pointer.as_index() else {
                unreachable_unchecked()
            };

            loop {
                char = self.skip_whitespace();
                if idx == 0 {
                    continue 'main;
                }

                idx -= 1;
                match char {
                    b'"' => self.skip_string_unchecked(),
                    b'{' | b'[' => self.skip_container_unchecked(),
                    _ => self.skip_literal_unchecked(),
                }
                self.skip_whitespace(); // skip ','
            }
        }

        char
    }

    unsafe fn string2<E: ErrorBuilder>(&mut self) -> Result<String<'a>, E> {
        let mut offset = self.idx() + 1;
        let mut buf = dangling_mut();
        let mut cap = 0;
        let mut len = 0;

        'main: {
            let err = loop {
                if self.simd_str() {
                    continue;
                }

                self.inc(1);
                if !S::NULL_PADDED && self.idx() >= self.src.len() {
                    break E::unclosed_string();
                }

                break match self.cur() {
                    b'"' => break 'main,
                    b'\\' => unsafe {
                        let count = self.idx() - offset;
                        let new_len = len + count + 4;

                        if cap < new_len {
                            let tmp = new_len * 5 / 4;
                            let layout = Layout::array::<u8>(tmp).unwrap_unchecked();

                            buf = if cap != 0 {
                                realloc(
                                    buf,
                                    Layout::array::<u8>(cap).unwrap_unchecked(),
                                    layout.size(),
                                )
                            } else {
                                alloc(layout)
                            };
                            cap = tmp;
                        }

                        buf.add(len)
                            .copy_from_nonoverlapping(self.src.ptr(offset), count);
                        self.inc(1);

                        len += count;
                        offset = self.idx() + 1;

                        if !S::NULL_PADDED && self.idx() == self.src.len() {
                            break E::unclosed_string();
                        }

                        let tmp = self.cur();
                        let buf = buf.add(len);
                        let esc = ESC_LUT[tmp as usize];

                        if esc != 0 {
                            buf.write(esc);
                            len += 1;
                            continue;
                        }

                        if tmp == b'u'
                            && let Some(v) = self.unicode_escape(&mut [0; 4])
                        {
                            buf.copy_from_nonoverlapping(v.as_ptr(), v.len());
                            offset = self.idx() + 1;
                            len += v.len();
                            continue;
                        }
                        E::invalid_escape()
                    },
                    c if !c.is_ascii_control() => continue,
                    _ => E::control_character(),
                };
            };

            return unsafe {
                if cap != 0 {
                    dealloc(buf, Layout::array::<u8>(cap).unwrap_unchecked())
                }

                Err(err)
            };
        }

        if !S::Volatility::IS_VOLATILE && len == 0 {
            return unsafe {
                // utf-8 validation is unnecessary here its going to match against string slice
                Ok(String::from_slice(from_raw_parts(
                    self.src.ptr(offset),
                    self.idx() - offset,
                )))
            };
        }

        let count = self.idx() - offset;
        let new_len = len + count;

        if cap < new_len {
            buf = unsafe {
                let layout = Layout::array::<u8>(new_len).unwrap_unchecked();

                if !(S::Volatility::IS_VOLATILE && cap == 0) {
                    realloc(
                        buf,
                        Layout::array::<u8>(cap).unwrap_unchecked(),
                        layout.size(),
                    )
                } else {
                    alloc(layout)
                }
            };
            cap = new_len;
        }

        unsafe {
            buf.add(len)
                .copy_from_nonoverlapping(self.src.ptr(offset), count);
            Ok(String::from_raw_parts(buf, new_len, cap))
        }
    }

    /// This function is used for matching against object key when using
    /// `parse_at_unchecked` and materializing lazy value.
    /// Validation is not required.
    pub(crate) fn string_unchecked2(&mut self) -> String<'a> {
        let mut offset = self.idx() + 1;
        let mut buf = dangling_mut();
        let mut cap = 0;
        let mut len = 0;

        loop {
            if self.simd_str_unchecked() {
                continue;
            }

            self.inc(1);
            match self.cur() {
                b'"' => break,
                b'\\' => unsafe {
                    let count = self.idx() - offset;
                    let new_len = len + count + 4;

                    if cap < new_len {
                        let tmp = new_len * 5 / 4;
                        let layout = Layout::array::<u8>(tmp).unwrap_unchecked();

                        buf = if cap != 0 {
                            realloc(
                                buf,
                                Layout::array::<u8>(cap).unwrap_unchecked(),
                                layout.size(),
                            )
                        } else {
                            alloc(layout)
                        };
                        cap = tmp;
                    }

                    buf.add(len)
                        .copy_from_nonoverlapping(self.src.ptr(offset), count);
                    self.inc(1);

                    len += count;
                    offset = self.idx() + 1;

                    let tmp = self.cur();
                    let buf = buf.add(len);
                    let esc = ESC_LUT[tmp as usize];

                    if esc != 0 {
                        buf.write(esc);
                        len += 1;
                        continue;
                    }

                    let tmp = &mut [0; 4];
                    let esc = self.unicode_escape(tmp).unwrap_unchecked();

                    buf.copy_from_nonoverlapping(esc.as_ptr(), esc.len());
                    offset = self.idx() + 1;
                    len += esc.len();
                    continue;
                },
                _ => continue,
            };
        }

        if !S::Volatility::IS_VOLATILE && len == 0 {
            return unsafe {
                String::from_slice(from_raw_parts(self.src.ptr(offset), self.idx() - offset))
            };
        }

        unsafe {
            let count = self.idx() - offset;
            let new_len = len + count;

            if cap < new_len {
                let layout = Layout::array::<u8>(new_len).unwrap_unchecked();

                buf = if !(S::Volatility::IS_VOLATILE && cap == 0) {
                    realloc(
                        buf,
                        Layout::array::<u8>(cap).unwrap_unchecked(),
                        layout.size(),
                    )
                } else {
                    alloc(layout)
                };
                cap = new_len;
            }

            buf.add(len)
                .copy_from_nonoverlapping(self.src.ptr(offset), count);
            String::from_raw_parts(buf, new_len, cap)
        }
    }
}
