#[derive(PartialEq, Eq)]
pub struct AdjustedMantissa {
    pub mantissa: u64,
    pub power2: i32,
}

impl AdjustedMantissa {
    #[inline]
    pub const fn zero_pow2(power2: i32) -> Self {
        Self {
            mantissa: 0,
            power2,
        }
    }
}
