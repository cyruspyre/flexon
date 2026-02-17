use core::{hint::select_unpredictable, slice::from_raw_parts, str::from_utf8_unchecked};

use crate::{Parser, config::Config, misc::*, source::Source, value::builder::ErrorBuilder};

impl<'a, S: Source, C: Config> Parser<'a, S, C> {
    #[inline]
    pub(crate) fn skip_value<E: ErrorBuilder>(&mut self) -> Result<(), E> {
        match self.skip_whitespace() {
            b'"' => self.skip_string(),
            b'{' => self.skip_object(),
            b'[' => self.skip_array(),
            0 => Err(E::expected_value()),
            _ => unsafe { self.skip_literal() },
        }
    }

    #[inline]
    pub(crate) fn skip_value_unchecked(&mut self) {
        match self.skip_whitespace() {
            b'"' => self.skip_string_unchecked(),
            b'{' | b'[' => unsafe { self.skip_container_unchecked() },
            _ => self.skip_literal_unchecked(),
        }
    }

    pub(super) fn skip_object<E: ErrorBuilder>(&mut self) -> Result<(), E> {
        let mut tmp = self.skip_whitespace();
        if tmp == b'}' {
            return Ok(());
        }

        let err = loop {
            if tmp != b'"' {
                break E::unexpected_token();
            }

            self.skip_string()?;
            if self.skip_whitespace() != b':' {
                break E::expected_colon();
            }

            self.skip_value()?;
            tmp = self.skip_whitespace();
            let comma = tmp == b',';
            if comma {
                tmp = self.skip_whitespace();
            }

            if tmp == b'}' {
                if !comma || self.cfg.trailing_comma() {
                    return Ok(());
                }

                break E::trailing_comma();
            }

            if comma || self.cfg.comma() {
                continue;
            }

            break match tmp {
                0 => E::eof(),
                _ => E::unexpected_token(),
            };
        };

        cold_path();
        Err(err)
    }

    pub(super) fn skip_array<E: ErrorBuilder>(&mut self) -> Result<(), E> {
        let mut tmp = self.skip_whitespace();

        if tmp == b']' {
            return Ok(());
        }

        let err = loop {
            match tmp {
                b'"' => self.skip_string(),
                b'{' => self.skip_object(),
                b'[' => self.skip_array(),
                0 => return Err(E::eof()),
                _ => unsafe { self.skip_literal() },
            }?;
            tmp = self.skip_whitespace();
            let comma = tmp == b',';

            if comma {
                tmp = self.skip_whitespace();
            }

            if tmp == b']' {
                if !comma || self.cfg.trailing_comma() {
                    return Ok(());
                }

                break E::trailing_comma();
            }

            if comma || self.cfg.comma() {
                continue;
            }

            break match tmp {
                0 => E::eof(),
                _ => E::unexpected_token(),
            };
        };

        cold_path();
        Err(err)
    }

    pub(super) fn skip_string<E: ErrorBuilder>(&mut self) -> Result<(), E> {
        let start = self.idx() + 1;
        let err = loop {
            if self.simd_str() {
                continue;
            }

            self.inc(1);
            if !S::NULL_PADDED && self.idx() >= self.src.len() {
                break E::unclosed_string();
            }

            break match self.cur() {
                b'"' => unsafe {
                    return match S::UTF8
                        || simdutf8::basic::from_utf8(from_raw_parts(
                            self.src.ptr(start),
                            self.idx() - start,
                        ))
                        .is_ok()
                    {
                        true => Ok(()),
                        _ => Err(E::unexpected_token()),
                    };
                },
                b'\\' => unsafe {
                    self.inc(1);
                    if !S::NULL_PADDED && self.idx() == self.src.len() {
                        break E::unclosed_string();
                    }

                    let tmp = self.cur();
                    let esc = ESC_LUT[tmp as usize];

                    if esc != 0 {
                        continue;
                    }

                    if tmp == b'u' && self.skip_unicode_escape() {
                        continue;
                    }

                    E::invalid_escape()
                },
                0x20.. => continue,
                0 if S::NULL_PADDED => E::eof(),
                _ => E::control_character(),
            };
        };

        return Err(self.close_string(err));
    }

    // typically this function rarely gets called so not worth complicating
    pub(crate) fn skip_string_unchecked(&mut self) {
        loop {
            if self.simd_str_unchecked() {
                continue;
            }

            self.inc(1);
            match self.cur() {
                b'"' => return,
                b'\\' => self.inc(1),
                _ => continue,
            }
        }
    }

    #[inline(never)]
    unsafe fn skip_unicode_escape<'esc>(&mut self) -> bool {
        self.inc(4);
        if !S::NULL_PADDED && self.idx() >= self.src.len() {
            match S::NULL_PADDED {
                true => self.cur.ptr = self.cur.ptr.sub(4),
                _ => self.cur.idx = self.cur.idx.wrapping_sub(4),
            }
            return false;
        }

        let mut codepoint = match u16::from_str_radix(
            from_utf8_unchecked(from_raw_parts(self.cur_ptr().sub(3), 4)),
            16,
        ) {
            Ok(v) => v as u32,
            _ => return false,
        };

        if (0xD800..=0xDBFF).contains(&codepoint) {
            self.inc(6);
            if !S::NULL_PADDED && self.idx() >= self.src.len()
                || from_raw_parts(self.cur_ptr().sub(5), 2) != br"\u"
            {
                match S::NULL_PADDED {
                    true => self.cur.ptr = self.cur.ptr.sub(6),
                    _ => self.cur.idx = self.cur.idx.wrapping_sub(6),
                }
                return false;
            }

            let low = match u16::from_str_radix(
                from_utf8_unchecked(from_raw_parts(self.cur_ptr().sub(3), 4)),
                16,
            ) {
                Ok(v) => v as u32,
                _ => return false,
            };

            if !(0xDC00..=0xDFFF).contains(&low) {
                return false;
            }

            codepoint = 0x10000 + (((codepoint - 0xD800) << 10) | (low - 0xDC00));
        }

        let mut buf = [0; 4];
        char::from_u32(codepoint)
            .map(|v| v.encode_utf8(&mut buf).as_bytes())
            .is_some()
    }

    #[inline]
    pub(super) unsafe fn skip_literal<E: ErrorBuilder>(&mut self) -> Result<(), E> {
        let tmp = self.cur();

        if NUM_LUT[tmp as usize] {
            let neg = tmp == b'-';
            if neg {
                self.inc(1)
            }

            if unlikely(
                !S::NULL_PADDED && self.idx() == self.src.len() || !NUM_LUT[self.cur() as usize],
            ) {
                return Err(E::invalid_literal());
            }

            if self.cur() == b'0'
                && (S::NULL_PADDED || self.idx() + 1 != self.src.len())
                && matches!(*self.cur_ptr().add(1), b'0'..=b'9')
            {
                return Err(E::leading_zero());
            }

            let start = self.idx();
            let (val, is_int) = self.parse_u64();

            if is_int {
                self.dec();
                if !neg || val < 9223372036854775809 {
                    return Ok(());
                }
            }

            if start == self.idx() {
                return Err(E::leading_decimal());
            }

            if let Some(val) = self.parse_f64(val, neg, start) {
                self.dec();
                return match val.is_finite() {
                    true => Ok(()),
                    _ => Err(E::number_overflow()),
                };
            }

            return Err(select_unpredictable(
                *self.cur_ptr().sub(1) == b'.',
                E::trailing_decimal(),
                E::invalid_literal(),
            ));
        }

        self.inc(3);
        let err = 'err: {
            if S::NULL_PADDED || self.idx() < self.src.len() {
                let ptr = self.cur_ptr().sub(3);

                match ptr.cast::<u32>().read_unaligned() {
                    0x6c6c756e | 0x65757274 => {}
                    0x736c6166
                        if (S::NULL_PADDED || self.idx() + 1 != self.src.len())
                            && *ptr.add(4) == b'e' =>
                    {
                        self.inc(1)
                    }
                    _ => break 'err E::invalid_literal(),
                }

                return Ok(());
            } else {
                break 'err E::invalid_literal();
            };
        };

        return Err(err);
    }

    pub(crate) fn skip_literal_unchecked(&mut self) {
        loop {
            self.inc(1);
            if (!S::NULL_PADDED && self.idx() >= self.src.len()) || NON_LIT_LUT[self.cur() as usize]
            {
                self.dec();
                return;
            }

            if self.simd_lit() {
                return;
            }
        }
    }
}
