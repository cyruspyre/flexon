use super::{common::AdjustedMantissa, decimal::Decimal, float::*};
use crate::{Parser, config::Config, source::Source};

impl<S: Source, C: Config> Parser<'_, S, C> {
    #[inline]
    pub(super) fn parse_long_mantissa(&mut self, start: usize) -> AdjustedMantissa {
        const MAX_SHIFT: usize = 60;
        const NUM_POWERS: usize = 19;
        const POWERS: [u8; 19] = [
            0, 3, 6, 9, 13, 16, 19, 23, 26, 29, 33, 36, 39, 43, 46, 49, 53, 56, 59,
        ];

        let get_shift = |n: usize| {
            if n < NUM_POWERS {
                POWERS[n] as usize
            } else {
                MAX_SHIFT
            }
        };
        let am_zero = AdjustedMantissa::zero_pow2(0);
        let am_inf = AdjustedMantissa::zero_pow2(INFINITE_POWER);
        let mut d = unsafe { self.parse_decimal(start) };

        if d.num_digits == 0 || d.decimal_point < -324 {
            return am_zero;
        } else if d.decimal_point >= 310 {
            return am_inf;
        }

        let mut exp2 = 0_i32;

        while d.decimal_point > 0 {
            let n = d.decimal_point as usize;
            let shift = get_shift(n);

            d.right_shift(shift);
            if d.decimal_point < -Decimal::DECIMAL_POINT_RANGE {
                return am_zero;
            }
            exp2 += shift as i32;
        }

        while d.decimal_point <= 0 {
            let shift = if d.decimal_point == 0 {
                match d.digits[0] {
                    digit if digit >= 5 => break,
                    0 | 1 => 2,
                    _ => 1,
                }
            } else {
                get_shift((-d.decimal_point) as usize)
            };

            d.left_shift(shift);
            if d.decimal_point > Decimal::DECIMAL_POINT_RANGE {
                return am_inf;
            }
            exp2 -= shift as i32;
        }

        exp2 -= 1;

        while (MINIMUM_EXPONENT + 1) > exp2 {
            let mut n = ((MINIMUM_EXPONENT + 1) - exp2) as usize;

            if n > MAX_SHIFT {
                n = MAX_SHIFT;
            }

            d.right_shift(n);
            exp2 += n as i32;
        }

        if (exp2 - MINIMUM_EXPONENT) >= INFINITE_POWER {
            return am_inf;
        }

        d.left_shift(MANTISSA_EXPLICIT_BITS + 1);
        let mut mantissa = d.round();

        if mantissa >= (1_u64 << (MANTISSA_EXPLICIT_BITS + 1)) {
            d.right_shift(1);
            exp2 += 1;
            mantissa = d.round();

            if (exp2 - MINIMUM_EXPONENT) >= INFINITE_POWER {
                return am_inf;
            }
        }

        let mut power2 = exp2 - MINIMUM_EXPONENT;
        if mantissa < (1_u64 << MANTISSA_EXPLICIT_BITS) {
            power2 -= 1;
        }
        mantissa &= (1_u64 << MANTISSA_EXPLICIT_BITS) - 1;

        AdjustedMantissa { mantissa, power2 }
    }
}
