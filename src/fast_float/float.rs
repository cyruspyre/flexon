pub const MANTISSA_EXPLICIT_BITS: usize = 52;
pub const MIN_EXPONENT_ROUND_TO_EVEN: i32 = -4;
pub const MAX_EXPONENT_ROUND_TO_EVEN: i32 = 23;
pub const MIN_EXPONENT_FAST_PATH: i64 = -22; // assuming FLT_EVAL_METHOD = 0
pub const MAX_EXPONENT_FAST_PATH: i64 = 22;
pub const MAX_EXPONENT_DISGUISED_FAST_PATH: i64 = 37;
pub const MINIMUM_EXPONENT: i32 = -1023;
pub const INFINITE_POWER: i32 = 0x7FF;
pub const SIGN_INDEX: usize = 63;
pub const SMALLEST_POWER_OF_TEN: i32 = -342;
pub const LARGEST_POWER_OF_TEN: i32 = 308;

pub const MAX_MANTISSA_FAST_PATH: u64 = 2_u64 << MANTISSA_EXPLICIT_BITS;

pub fn pow10_fast_path(exponent: usize) -> f64 {
    const TABLE: [f64; 32] = [
        1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
        1e17, 1e18, 1e19, 1e20, 1e21, 1e22, 0., 0., 0., 0., 0., 0., 0., 0., 0.,
    ];

    TABLE[exponent & 31]
}
