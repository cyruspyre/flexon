use crate::{Parser, config::Config, source::Source};

pub struct Decimal {
    pub num_digits: usize,
    pub decimal_point: i32,
    pub negative: bool,
    pub truncated: bool,
    pub digits: [u8; Self::MAX_DIGITS],
}

impl Decimal {
    pub const MAX_DIGITS: usize = 768;
    pub const MAX_DIGITS_WITHOUT_OVERFLOW: usize = 19;
    pub const DECIMAL_POINT_RANGE: i32 = 2047;

    #[inline]
    pub fn try_add_digit(&mut self, digit: u8) {
        if self.num_digits < Self::MAX_DIGITS {
            unsafe { *self.digits.get_unchecked_mut(self.num_digits) = digit }
        }

        self.num_digits += 1;
    }

    #[inline]
    pub fn trim(&mut self) {
        while unsafe {
            self.num_digits != 0 && *self.digits.get_unchecked(self.num_digits - 1) == 0
        } {
            self.num_digits -= 1;
        }
    }

    #[inline]
    pub fn round(&self) -> u64 {
        if self.num_digits == 0 || self.decimal_point < 0 {
            return 0;
        } else if self.decimal_point > 18 {
            return 0xFFFF_FFFF_FFFF_FFFF_u64;
        }

        let dp = self.decimal_point as usize;
        let mut n = 0_u64;

        for i in 0..dp {
            n *= 10;

            if i < self.num_digits {
                n += unsafe { *self.digits.get_unchecked(i) as u64 }
            }
        }

        let mut round_up = false;

        if dp < self.num_digits {
            unsafe {
                round_up = *self.digits.get_unchecked(dp) >= 5;

                if *self.digits.get_unchecked(dp) == 5 && dp + 1 == self.num_digits {
                    round_up = self.truncated
                        || ((dp != 0) && (1 & *self.digits.get_unchecked(dp - 1) != 0));
                }
            }
        }

        if round_up {
            n += 1;
        }

        n
    }

    #[inline]
    pub fn left_shift(&mut self, shift: usize) {
        if self.num_digits == 0 {
            return;
        }

        let num_new_digits = number_of_digits_decimal_left_shift(self, shift);
        let mut read_index = self.num_digits;
        let mut write_index = self.num_digits + num_new_digits;
        let mut n = 0_u64;

        while read_index != 0 {
            read_index -= 1;
            write_index -= 1;
            n += unsafe { *self.digits.get_unchecked(read_index) as u64 } << shift;

            let quotient = n / 10;
            let remainder = n - (10 * quotient);

            if write_index < Self::MAX_DIGITS {
                unsafe { *self.digits.get_unchecked_mut(write_index) = remainder as u8 }
            } else if remainder > 0 {
                self.truncated = true;
            }

            n = quotient;
        }

        while n > 0 {
            write_index -= 1;

            let quotient = n / 10;
            let remainder = n - (10 * quotient);

            if write_index < Self::MAX_DIGITS {
                unsafe { *self.digits.get_unchecked_mut(write_index) = remainder as u8 };
            } else if remainder > 0 {
                self.truncated = true;
            }

            n = quotient;
        }

        self.num_digits += num_new_digits;

        if self.num_digits > Self::MAX_DIGITS {
            self.num_digits = Self::MAX_DIGITS;
        }

        self.decimal_point += num_new_digits as i32;
        self.trim();
    }

    #[inline]
    pub fn right_shift(&mut self, shift: usize) {
        let mut read_index = 0;
        let mut write_index = 0;
        let mut n = 0_u64;

        while (n >> shift) == 0 {
            if read_index < self.num_digits {
                n = (10 * n) + unsafe { *self.digits.get_unchecked(read_index) as u64 };
                read_index += 1;
            } else if n == 0 {
                return;
            } else {
                while (n >> shift) == 0 {
                    n *= 10;
                    read_index += 1;
                }
                break;
            }
        }

        self.decimal_point -= read_index as i32 - 1;

        if self.decimal_point < -Self::DECIMAL_POINT_RANGE {
            self.num_digits = 0;
            self.decimal_point = 0;
            self.negative = false;
            self.truncated = false;
            return;
        }

        let mask = (1_u64 << shift) - 1;

        while read_index < self.num_digits {
            let new_digit = (n >> shift) as u8;

            n = (10 * (n & mask)) + unsafe { *self.digits.get_unchecked(read_index) as u64 };
            read_index += 1;
            unsafe { *self.digits.get_unchecked_mut(write_index) = new_digit };
            write_index += 1;
        }

        while n > 0 {
            let new_digit = (n >> shift) as u8;

            n = 10 * (n & mask);

            if write_index < Self::MAX_DIGITS {
                unsafe { *self.digits.get_unchecked_mut(write_index) = new_digit };
                write_index += 1;
            } else if new_digit > 0 {
                self.truncated = true;
            }
        }

        self.num_digits = write_index;
        self.trim();
    }
}

impl<S: Source, C: Config> Parser<'_, S, C> {
    #[inline(never)]
    pub(super) unsafe fn parse_decimal(&mut self, start: usize) -> Decimal {
        let mut d = Decimal {
            num_digits: 0,
            decimal_point: 0,
            negative: false,
            truncated: false,
            digits: [0; Decimal::MAX_DIGITS],
        };
        let mut idx = start;
        let tmp = *self.src.ptr(idx);

        if tmp == b'-' {
            d.negative = true;
            idx += 1;
        }

        // first char wil always be '-' or ascii digit
        idx += (*self.src.ptr(start) ^ b'0').wrapping_sub(1) as usize >> 7;
        self.parse_digits(&mut idx, |digit| d.try_add_digit(digit));

        if self.check_cur(&mut idx, b'.') {
            idx += 1;
            let first = idx;

            while (S::NULL_PADDED || idx + 8 < self.src.len())
                && d.num_digits + 8 < Decimal::MAX_DIGITS
            {
                let v = u64::from_le(self.src.ptr(idx).cast::<u64>().read_unaligned());
                let a = v.wrapping_add(0x4646_4646_4646_4646);
                let b = v.wrapping_sub(0x3030_3030_3030_3030);

                if (a | b) & 0x8080_8080_8080_8080 != 0 {
                    break;
                }

                d.digits
                    .get_unchecked_mut(d.num_digits..)
                    .as_mut_ptr()
                    .cast::<u64>()
                    .write_unaligned(u64::to_le(v - 0x3030_3030_3030_3030));
                d.num_digits += 8;
                idx += 8;
            }

            self.parse_digits(&mut idx, |digit| d.try_add_digit(digit));
            d.decimal_point = (idx - first).wrapping_neg() as _;
        }

        if d.num_digits != 0 {
            // Ignore the trailing zeros if there are any
            let mut n_trailing_zeros = 0;

            for i in (start..idx).rev() {
                let tmp = *self.src.ptr(i);

                if tmp == b'0' {
                    n_trailing_zeros += 1;
                } else if tmp != b'.' {
                    break;
                }
            }

            d.decimal_point += n_trailing_zeros as i32;
            d.num_digits -= n_trailing_zeros;
            d.decimal_point += d.num_digits as i32;

            if d.num_digits > Decimal::MAX_DIGITS {
                d.truncated = true;
                d.num_digits = Decimal::MAX_DIGITS;
            }
        }

        if self.check_cur_either(&mut idx, b'e', b'E') {
            idx += 1;
            let mut neg_exp = false;

            if self.check_cur(&mut idx, b'-') {
                neg_exp = true;
                idx += 1;
            } else if self.check_cur(&mut idx, b'+') {
                idx += 1;
            }

            let mut exp_num = 0_i32;

            self.parse_digits(&mut idx, |digit| {
                if exp_num < 0x10000 {
                    exp_num = 10 * exp_num + digit as i32;
                }
            });

            d.decimal_point += if neg_exp { -exp_num } else { exp_num };
        }

        for i in d.num_digits..Decimal::MAX_DIGITS_WITHOUT_OVERFLOW {
            unsafe { *d.digits.get_unchecked_mut(i) = 0 }
        }

        d
    }

    #[inline]
    fn parse_digits(&mut self, idx: &mut usize, mut f: impl FnMut(u8)) {
        while S::NULL_PADDED || *idx != self.src.len() {
            let tmp = unsafe { (*self.src.ptr(*idx)).wrapping_sub(b'0') };

            if tmp < 10 {
                f(tmp);
                *idx += 1;
            } else {
                break;
            }
        }
    }

    #[inline]
    unsafe fn check_cur(&mut self, offset: &mut usize, c: u8) -> bool {
        (S::NULL_PADDED || *offset != self.src.len()) && *self.src.ptr(*offset) == c
    }

    #[inline]
    unsafe fn check_cur_either(&mut self, offset: &mut usize, a: u8, b: u8) -> bool {
        (S::NULL_PADDED || *offset != self.src.len())
            && (*self.src.ptr(*offset) == a || *self.src.ptr(*offset) == b)
    }
}

#[inline]
fn number_of_digits_decimal_left_shift(d: &Decimal, mut shift: usize) -> usize {
    const TABLE: [u16; 65] = [
        0x0000, 0x0800, 0x0801, 0x0803, 0x1006, 0x1009, 0x100D, 0x1812, 0x1817, 0x181D, 0x2024,
        0x202B, 0x2033, 0x203C, 0x2846, 0x2850, 0x285B, 0x3067, 0x3073, 0x3080, 0x388E, 0x389C,
        0x38AB, 0x38BB, 0x40CC, 0x40DD, 0x40EF, 0x4902, 0x4915, 0x4929, 0x513E, 0x5153, 0x5169,
        0x5180, 0x5998, 0x59B0, 0x59C9, 0x61E3, 0x61FD, 0x6218, 0x6A34, 0x6A50, 0x6A6D, 0x6A8B,
        0x72AA, 0x72C9, 0x72E9, 0x7B0A, 0x7B2B, 0x7B4D, 0x8370, 0x8393, 0x83B7, 0x83DC, 0x8C02,
        0x8C28, 0x8C4F, 0x9477, 0x949F, 0x94C8, 0x9CF2, 0x051C, 0x051C, 0x051C, 0x051C,
    ];
    const TABLE_POW5: [u8; 0x051C] = [
        5, 2, 5, 1, 2, 5, 6, 2, 5, 3, 1, 2, 5, 1, 5, 6, 2, 5, 7, 8, 1, 2, 5, 3, 9, 0, 6, 2, 5, 1,
        9, 5, 3, 1, 2, 5, 9, 7, 6, 5, 6, 2, 5, 4, 8, 8, 2, 8, 1, 2, 5, 2, 4, 4, 1, 4, 0, 6, 2, 5,
        1, 2, 2, 0, 7, 0, 3, 1, 2, 5, 6, 1, 0, 3, 5, 1, 5, 6, 2, 5, 3, 0, 5, 1, 7, 5, 7, 8, 1, 2,
        5, 1, 5, 2, 5, 8, 7, 8, 9, 0, 6, 2, 5, 7, 6, 2, 9, 3, 9, 4, 5, 3, 1, 2, 5, 3, 8, 1, 4, 6,
        9, 7, 2, 6, 5, 6, 2, 5, 1, 9, 0, 7, 3, 4, 8, 6, 3, 2, 8, 1, 2, 5, 9, 5, 3, 6, 7, 4, 3, 1,
        6, 4, 0, 6, 2, 5, 4, 7, 6, 8, 3, 7, 1, 5, 8, 2, 0, 3, 1, 2, 5, 2, 3, 8, 4, 1, 8, 5, 7, 9,
        1, 0, 1, 5, 6, 2, 5, 1, 1, 9, 2, 0, 9, 2, 8, 9, 5, 5, 0, 7, 8, 1, 2, 5, 5, 9, 6, 0, 4, 6,
        4, 4, 7, 7, 5, 3, 9, 0, 6, 2, 5, 2, 9, 8, 0, 2, 3, 2, 2, 3, 8, 7, 6, 9, 5, 3, 1, 2, 5, 1,
        4, 9, 0, 1, 1, 6, 1, 1, 9, 3, 8, 4, 7, 6, 5, 6, 2, 5, 7, 4, 5, 0, 5, 8, 0, 5, 9, 6, 9, 2,
        3, 8, 2, 8, 1, 2, 5, 3, 7, 2, 5, 2, 9, 0, 2, 9, 8, 4, 6, 1, 9, 1, 4, 0, 6, 2, 5, 1, 8, 6,
        2, 6, 4, 5, 1, 4, 9, 2, 3, 0, 9, 5, 7, 0, 3, 1, 2, 5, 9, 3, 1, 3, 2, 2, 5, 7, 4, 6, 1, 5,
        4, 7, 8, 5, 1, 5, 6, 2, 5, 4, 6, 5, 6, 6, 1, 2, 8, 7, 3, 0, 7, 7, 3, 9, 2, 5, 7, 8, 1, 2,
        5, 2, 3, 2, 8, 3, 0, 6, 4, 3, 6, 5, 3, 8, 6, 9, 6, 2, 8, 9, 0, 6, 2, 5, 1, 1, 6, 4, 1, 5,
        3, 2, 1, 8, 2, 6, 9, 3, 4, 8, 1, 4, 4, 5, 3, 1, 2, 5, 5, 8, 2, 0, 7, 6, 6, 0, 9, 1, 3, 4,
        6, 7, 4, 0, 7, 2, 2, 6, 5, 6, 2, 5, 2, 9, 1, 0, 3, 8, 3, 0, 4, 5, 6, 7, 3, 3, 7, 0, 3, 6,
        1, 3, 2, 8, 1, 2, 5, 1, 4, 5, 5, 1, 9, 1, 5, 2, 2, 8, 3, 6, 6, 8, 5, 1, 8, 0, 6, 6, 4, 0,
        6, 2, 5, 7, 2, 7, 5, 9, 5, 7, 6, 1, 4, 1, 8, 3, 4, 2, 5, 9, 0, 3, 3, 2, 0, 3, 1, 2, 5, 3,
        6, 3, 7, 9, 7, 8, 8, 0, 7, 0, 9, 1, 7, 1, 2, 9, 5, 1, 6, 6, 0, 1, 5, 6, 2, 5, 1, 8, 1, 8,
        9, 8, 9, 4, 0, 3, 5, 4, 5, 8, 5, 6, 4, 7, 5, 8, 3, 0, 0, 7, 8, 1, 2, 5, 9, 0, 9, 4, 9, 4,
        7, 0, 1, 7, 7, 2, 9, 2, 8, 2, 3, 7, 9, 1, 5, 0, 3, 9, 0, 6, 2, 5, 4, 5, 4, 7, 4, 7, 3, 5,
        0, 8, 8, 6, 4, 6, 4, 1, 1, 8, 9, 5, 7, 5, 1, 9, 5, 3, 1, 2, 5, 2, 2, 7, 3, 7, 3, 6, 7, 5,
        4, 4, 3, 2, 3, 2, 0, 5, 9, 4, 7, 8, 7, 5, 9, 7, 6, 5, 6, 2, 5, 1, 1, 3, 6, 8, 6, 8, 3, 7,
        7, 2, 1, 6, 1, 6, 0, 2, 9, 7, 3, 9, 3, 7, 9, 8, 8, 2, 8, 1, 2, 5, 5, 6, 8, 4, 3, 4, 1, 8,
        8, 6, 0, 8, 0, 8, 0, 1, 4, 8, 6, 9, 6, 8, 9, 9, 4, 1, 4, 0, 6, 2, 5, 2, 8, 4, 2, 1, 7, 0,
        9, 4, 3, 0, 4, 0, 4, 0, 0, 7, 4, 3, 4, 8, 4, 4, 9, 7, 0, 7, 0, 3, 1, 2, 5, 1, 4, 2, 1, 0,
        8, 5, 4, 7, 1, 5, 2, 0, 2, 0, 0, 3, 7, 1, 7, 4, 2, 2, 4, 8, 5, 3, 5, 1, 5, 6, 2, 5, 7, 1,
        0, 5, 4, 2, 7, 3, 5, 7, 6, 0, 1, 0, 0, 1, 8, 5, 8, 7, 1, 1, 2, 4, 2, 6, 7, 5, 7, 8, 1, 2,
        5, 3, 5, 5, 2, 7, 1, 3, 6, 7, 8, 8, 0, 0, 5, 0, 0, 9, 2, 9, 3, 5, 5, 6, 2, 1, 3, 3, 7, 8,
        9, 0, 6, 2, 5, 1, 7, 7, 6, 3, 5, 6, 8, 3, 9, 4, 0, 0, 2, 5, 0, 4, 6, 4, 6, 7, 7, 8, 1, 0,
        6, 6, 8, 9, 4, 5, 3, 1, 2, 5, 8, 8, 8, 1, 7, 8, 4, 1, 9, 7, 0, 0, 1, 2, 5, 2, 3, 2, 3, 3,
        8, 9, 0, 5, 3, 3, 4, 4, 7, 2, 6, 5, 6, 2, 5, 4, 4, 4, 0, 8, 9, 2, 0, 9, 8, 5, 0, 0, 6, 2,
        6, 1, 6, 1, 6, 9, 4, 5, 2, 6, 6, 7, 2, 3, 6, 3, 2, 8, 1, 2, 5, 2, 2, 2, 0, 4, 4, 6, 0, 4,
        9, 2, 5, 0, 3, 1, 3, 0, 8, 0, 8, 4, 7, 2, 6, 3, 3, 3, 6, 1, 8, 1, 6, 4, 0, 6, 2, 5, 1, 1,
        1, 0, 2, 2, 3, 0, 2, 4, 6, 2, 5, 1, 5, 6, 5, 4, 0, 4, 2, 3, 6, 3, 1, 6, 6, 8, 0, 9, 0, 8,
        2, 0, 3, 1, 2, 5, 5, 5, 5, 1, 1, 1, 5, 1, 2, 3, 1, 2, 5, 7, 8, 2, 7, 0, 2, 1, 1, 8, 1, 5,
        8, 3, 4, 0, 4, 5, 4, 1, 0, 1, 5, 6, 2, 5, 2, 7, 7, 5, 5, 5, 7, 5, 6, 1, 5, 6, 2, 8, 9, 1,
        3, 5, 1, 0, 5, 9, 0, 7, 9, 1, 7, 0, 2, 2, 7, 0, 5, 0, 7, 8, 1, 2, 5, 1, 3, 8, 7, 7, 7, 8,
        7, 8, 0, 7, 8, 1, 4, 4, 5, 6, 7, 5, 5, 2, 9, 5, 3, 9, 5, 8, 5, 1, 1, 3, 5, 2, 5, 3, 9, 0,
        6, 2, 5, 6, 9, 3, 8, 8, 9, 3, 9, 0, 3, 9, 0, 7, 2, 2, 8, 3, 7, 7, 6, 4, 7, 6, 9, 7, 9, 2,
        5, 5, 6, 7, 6, 2, 6, 9, 5, 3, 1, 2, 5, 3, 4, 6, 9, 4, 4, 6, 9, 5, 1, 9, 5, 3, 6, 1, 4, 1,
        8, 8, 8, 2, 3, 8, 4, 8, 9, 6, 2, 7, 8, 3, 8, 1, 3, 4, 7, 6, 5, 6, 2, 5, 1, 7, 3, 4, 7, 2,
        3, 4, 7, 5, 9, 7, 6, 8, 0, 7, 0, 9, 4, 4, 1, 1, 9, 2, 4, 4, 8, 1, 3, 9, 1, 9, 0, 6, 7, 3,
        8, 2, 8, 1, 2, 5, 8, 6, 7, 3, 6, 1, 7, 3, 7, 9, 8, 8, 4, 0, 3, 5, 4, 7, 2, 0, 5, 9, 6, 2,
        2, 4, 0, 6, 9, 5, 9, 5, 3, 3, 6, 9, 1, 4, 0, 6, 2, 5,
    ];

    shift &= 63;

    let x_a = TABLE[shift];
    let x_b = TABLE[shift + 1];
    let num_new_digits = (x_a >> 11) as usize;
    let pow5_a = (0x7FF & x_a) as usize;
    let pow5_b = (0x7FF & x_b) as usize;
    let pow5 = &TABLE_POW5[pow5_a..];

    for (i, &p5) in pow5.iter().enumerate().take(pow5_b - pow5_a) {
        return if i >= d.num_digits {
            num_new_digits - 1
        } else if d.digits[i] == p5 {
            continue;
        } else if d.digits[i] < p5 {
            num_new_digits - 1
        } else {
            num_new_digits
        };
    }

    num_new_digits
}
