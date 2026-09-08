pub const NUM_LUT: [bool; 256] = {
    let mut tmp = [false; 256];
    let mut idx = b'0';

    while idx <= b'9' {
        tmp[idx as usize] = true;
        idx += 1;
    }

    tmp[b'-' as usize] = true;
    tmp[b'.' as usize] = true;
    tmp[b'e' as usize] = true;
    tmp[b'E' as usize] = true;

    tmp
};

pub const ESC_LUT: [u8; 256] = {
    let mut tmp = [0; 256];

    tmp[b'"' as usize] = b'"';
    tmp[b'/' as usize] = b'/';
    tmp[b'n' as usize] = b'\n';
    tmp[b't' as usize] = b'\t';
    tmp[b'r' as usize] = b'\r';
    tmp[b'\\' as usize] = b'\\';
    tmp[b'b' as usize] = b'\x08';
    tmp[b'f' as usize] = b'\x0C';

    tmp
};

pub trait Sealed {}

pub unsafe fn parse_4hex(v: *const u8) -> Option<u32> {
    const ONES: u32 = 0x0101_0101;
    const LOW7: u32 = 0x7F * ONES;

    let chunk = v.cast::<u32>().read_unaligned();

    let digit = chunk.wrapping_sub(0x30 * ONES);
    let alpha = chunk & !(0x20 * ONES);

    let digit_bad = (digit & LOW7).wrapping_add(0x76 * ONES);
    let alpha_bad = (alpha.wrapping_add(0x3F * ONES) & LOW7).wrapping_add(0x7A * ONES);

    if (digit_bad & alpha_bad | chunk) & 0x80 * ONES != 0 {
        return None;
    }

    let letters = chunk & 0x40 * ONES;
    let nine = (letters >> 6) * 9; // 0x09 per letter lane
    let nib = (chunk & 0x0F * ONES) + nine;

    let tmp = (nib | nib << 12) & 0xFF00_FF00;
    let val = (tmp | tmp >> 24) & 0xFFFF;

    Some(val)
}

#[inline(always)]
pub fn likely(b: bool) -> bool {
    #[cfg(feature = "nightly")]
    return core::hint::likely(b);

    #[cfg(not(feature = "nightly"))]
    return b;
}

#[inline(always)]
pub fn unlikely(b: bool) -> bool {
    #[cfg(feature = "nightly")]
    return core::hint::unlikely(b);

    #[cfg(not(feature = "nightly"))]
    return b;
}

#[inline(never)]
#[cfg(feature = "alloc")]
pub fn capacity_overflow() -> ! {
    panic!("capacity overflow")
}
