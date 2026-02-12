// from `fast-float2` at commit `d4f5749`

mod binary;
mod common;
mod decimal;
mod float;
mod number;
mod simple;
mod table;

use binary::compute_float;
use float::*;
use number::Number;

use crate::{
    Parser, config::Config, fast_float::number::MIN_19DIGIT_INT, misc::INT_LUT, simd::simd_u64,
    source::Source,
};

impl<S: Source, C: Config> Parser<'_, S, C> {
    #[inline(always)]
    pub(crate) unsafe fn parse_f64(
        &mut self,
        mut mantissa: u64,
        neg: bool,
        start: usize,
    ) -> Option<f64> {
        let mut n_digits = self.idx() - start;
        let mut exponent = 0;
        let int_end = self.idx();

        if self.cur() == b'.' {
            self.inc(1);
            let stamp = self.idx();

            'tmp: {
                if S::NULL_PADDED || self.idx() + 7 < self.src.len() {
                    let Some(chunk) = simd_u64(self.cur_ptr()) else {
                        break 'tmp;
                    };

                    mantissa = mantissa.wrapping_mul(100_000_000).wrapping_add(chunk);
                    self.inc(8);
                }
            }

            while S::NULL_PADDED || self.idx() != self.src.len() {
                let tmp = INT_LUT[self.cur() as usize];
                if tmp == 16 {
                    break;
                }

                mantissa = mantissa.wrapping_mul(10).wrapping_add(tmp as _);
                self.inc(1);
            }

            let tmp = self.idx() - stamp;
            if tmp == 0 {
                return None;
            }

            exponent = tmp.wrapping_neg() as i64;
            n_digits += tmp;
        }

        let exp_number = self.parse_scientific()?;
        exponent += exp_number;
        let num = 'tmp: {
            if n_digits <= 19 {
                break 'tmp Number {
                    exponent,
                    mantissa,
                    negative: neg,
                    many_digits: false,
                };
            }

            // count the rest
            while S::NULL_PADDED || self.idx() != self.src.len() {
                let tmp = INT_LUT[self.cur() as usize];
                if tmp == 16 {
                    break;
                }

                n_digits += 1;
                self.inc(1);
            }

            n_digits -= 19;
            let mut idx = start;
            while (S::NULL_PADDED || idx != self.src.len()) && n_digits != 0 {
                match *self.src.ptr(idx) {
                    v @ (b'.' | b'0') => {
                        // decrement when its '0'
                        n_digits -= (v - b'.') as usize >> 1;
                        idx += 1;
                    }
                    _ => break,
                }
            }

            let mut many_digits = false;
            if n_digits != 0 {
                // at this point we have more than 19 significant digits, let's try again
                mantissa = 0;
                many_digits = true;

                let mut idx = start;
                self.try_parse_19digits(&mut idx, &mut mantissa);

                exponent = if mantissa >= MIN_19DIGIT_INT {
                    (int_end - idx) as _ // big int
                } else {
                    // cur idx will be at '.'
                    idx += 1;
                    let before = idx;
                    self.try_parse_19digits(&mut idx, &mut mantissa);
                    (idx - before).wrapping_neg() as _
                };
                exponent += exp_number; // add back the explicit part
            }

            Number {
                exponent,
                mantissa,
                negative: neg,
                many_digits,
            }
        };

        if let Some(value) = num.try_fast_path() {
            return Some(value);
        }

        let mut am = compute_float(num.exponent, num.mantissa);
        if num.many_digits && am != compute_float(num.exponent, num.mantissa + 1) {
            am.power2 = -1;
        }

        if am.power2 < 0 {
            am = self.parse_long_mantissa(start - neg as usize)
        }

        let mut word = am.mantissa;

        word |= (am.power2 as u64) << MANTISSA_EXPLICIT_BITS;
        word |= (num.negative as u64) << SIGN_INDEX;

        Some(f64::from_bits(word))
    }

    #[inline(always)]
    unsafe fn parse_scientific(&mut self) -> Option<i64> {
        if !S::NULL_PADDED && self.idx() == self.src.len() || !matches!(self.cur(), b'e' | b'E') {
            return Some(0);
        }

        self.inc(1);
        if !S::NULL_PADDED && self.idx() == self.src.len() {
            return None;
        }

        let mut num = 0i64;
        let mut neg = false;
        let mut flag = false;

        if matches!(self.cur(), b'-' | b'+') {
            neg = self.cur() == b'-';
            self.inc(1);
        }

        while S::NULL_PADDED || self.idx() != self.src.len() {
            let tmp = self.cur();

            if tmp < b'0' || tmp > b'9' {
                break;
            }

            if num < 0x10000 {
                num = num * 10 + (tmp - b'0') as i64;
            }

            flag = true;
            self.inc(1);
        }

        if flag {
            Some(if neg { -num } else { num })
        } else {
            None
        }
    }

    #[inline]
    unsafe fn try_parse_19digits(&mut self, idx: &mut usize, mantissa: &mut u64) {
        while (S::NULL_PADDED || *idx != self.src.len()) && *mantissa < MIN_19DIGIT_INT {
            let tmp = INT_LUT[*self.src.ptr(*idx) as usize];

            if tmp == 16 {
                break;
            }

            *mantissa = mantissa.wrapping_mul(10).wrapping_add((tmp) as _);
            *idx += 1;
        }
    }
}
