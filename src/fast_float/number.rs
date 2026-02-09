use super::float::*;

pub const MIN_19DIGIT_INT: u64 = 100_0000_0000_0000_0000;
pub const INT_POW10: [u64; 16] = [
    1,
    10,
    100,
    1000,
    10000,
    100000,
    1000000,
    10000000,
    100000000,
    1000000000,
    10000000000,
    100000000000,
    1000000000000,
    10000000000000,
    100000000000000,
    1000000000000000,
];

#[derive(Clone, Copy, Debug)]
pub struct Number {
    pub exponent: i64,
    pub mantissa: u64,
    pub negative: bool,
    pub many_digits: bool,
}

impl Number {
    #[inline]
    fn is_fast_path(&self) -> bool {
        MIN_EXPONENT_FAST_PATH <= self.exponent
            && self.exponent <= MAX_EXPONENT_DISGUISED_FAST_PATH
            && self.mantissa <= MAX_MANTISSA_FAST_PATH
            && !self.many_digits
    }

    #[inline]
    pub fn try_fast_path(&self) -> Option<f64> {
        if self.is_fast_path() {
            let mut value = if self.exponent <= MAX_EXPONENT_FAST_PATH {
                // normal fast path
                let value = self.mantissa as f64;

                if self.exponent < 0 {
                    value / pow10_fast_path((-self.exponent) as usize)
                } else {
                    value * pow10_fast_path(self.exponent as usize)
                }
            } else {
                // disguised fast path
                let shift = self.exponent - MAX_EXPONENT_FAST_PATH;
                let mantissa = self.mantissa.checked_mul(INT_POW10[shift as usize])?;

                if mantissa > MAX_MANTISSA_FAST_PATH {
                    return None;
                }

                mantissa as f64 * pow10_fast_path(MAX_EXPONENT_FAST_PATH as usize)
            };

            value = f64::from_bits(value.to_bits() ^ ((self.negative as u64) << 63));

            Some(value)
        } else {
            None
        }
    }
}
