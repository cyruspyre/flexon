use crate::{Parser, config::Config, misc::*, source::Source, value::builder::ErrorBuilder};
use core::{hint::cold_path, slice::from_raw_parts, str::from_utf8_unchecked};

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
    #[cfg(feature = "alloc")]
    pub(crate) fn skip_value_unchecked(&mut self) {
        match self.skip_whitespace() {
            b'"' => self.skip_string_unchecked(),
            b'{' | b'[' => unsafe { self.skip_container_unchecked() },
            _ => self.skip_literal_unchecked(),
        }
    }

    pub(super) fn skip_object<E: ErrorBuilder>(&mut self) -> Result<(), E> {
        self.inc(1);
        let mut tmp = self.skip_whitespace();

        if tmp == b'}' {
            return Ok(());
        }

        let err = loop {
            if tmp != b'"' {
                break E::unexpected_token();
            }

            self.skip_string()?;
            self.inc(1);

            if self.skip_whitespace() != b':' {
                break E::expected_colon();
            }

            self.inc(1);
            self.skip_value()?;
            self.inc(1);
            tmp = self.skip_whitespace();

            let comma = tmp == b',';

            if comma {
                self.inc(1);
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
        self.inc(1);
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

            self.inc(1);
            tmp = self.skip_whitespace();

            let comma = tmp == b',';

            if comma {
                self.inc(1);
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
                    return match Self::PRE_VALIDATED_UTF8
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
    unsafe fn skip_unicode_escape(&mut self) -> bool {
        self.inc(4);
        if !S::NULL_PADDED && self.idx() >= self.src.len() {
            self.dec(4);
            return false;
        }

        let codepoint = match u16::from_str_radix(
            from_utf8_unchecked(from_raw_parts(self.cur_ptr().sub(3), 4)),
            16,
        ) {
            Ok(v) => v as u32,
            _ => return false,
        };

        if (0xD800..=0xDFFF).contains(&codepoint) {
            if codepoint >= 0xDC00 {
                return false;
            }

            self.inc(6);
            if !S::NULL_PADDED && self.idx() >= self.src.len()
                || from_raw_parts(self.cur_ptr().sub(5), 2) != br"\u"
            {
                self.dec(6);
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
        }

        // `codepoint` <= 0x10FFFF (char::MAX) excluding `0xD800..=0xDFFF`
        true
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
                self.dec(1);
                if !neg || val < 9223372036854775809 {
                    return Ok(());
                }
            }

            if start == self.idx() {
                return Err(E::leading_decimal());
            }

            if let Some(val) = self.parse_f64(val, neg, start) {
                return match val.is_finite() {
                    true => Ok(()),
                    _ => Err(E::number_overflow()),
                };
            }

            return Err(match *self.cur_ptr().sub(1) {
                b'.' => E::trailing_decimal(),
                _ => E::invalid_literal(),
            });
        }

        self.inc(3);
        if S::NULL_PADDED || self.idx() < self.src.len() {
            match self.cur_ptr().sub(3).cast::<u32>().read_unaligned() {
                0x6c6c756e | 0x65757274 => return Ok(()),
                0x736c6166
                    if (S::NULL_PADDED || self.idx() + 1 != self.src.len())
                        && *self.cur_ptr().add(1) == b'e' =>
                {
                    self.inc(1);
                    return Ok(());
                }
                _ => {}
            }
        }

        Err(E::invalid_literal())
    }

    pub(crate) fn skip_literal_unchecked(&mut self) {
        pub const NON_LIT_LUT: [bool; 256] = {
            let mut tmp = [false; 256];

            tmp[b'{' as usize] = true;
            tmp[b'}' as usize] = true;
            tmp[b'[' as usize] = true;
            tmp[b']' as usize] = true;
            tmp[b'"' as usize] = true;
            tmp[b',' as usize] = true;
            tmp[b'/' as usize] = true;
            tmp[b' ' as usize] = true;
            tmp[b'\n' as usize] = true;
            tmp[b'\t' as usize] = true;
            tmp[b'\r' as usize] = true;
            tmp[b'\0' as usize] = true;

            tmp
        };

        loop {
            if NON_LIT_LUT[self.cur() as usize] {
                return self.dec(1);
            }

            self.inc(1);
            if !S::NULL_PADDED && self.idx() >= self.src.len() {
                return self.dec(1);
            }
        }
    }
}
