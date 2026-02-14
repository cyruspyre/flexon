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

pub const INT_LUT: [u64; 256] = {
    let mut tmp = [16; 256];
    let mut idx = b'0';

    while idx <= b'9' {
        tmp[idx as usize] = (idx - b'0') as u64;
        idx += 1;
    }

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

pub const NON_LIT_LUT: [bool; 256] = {
    let mut tmp = [false; 256];

    tmp[b'"' as usize] = true;
    tmp[b':' as usize] = true;
    tmp[b',' as usize] = true;
    tmp[b'{' as usize] = true;
    tmp[b'}' as usize] = true;
    tmp[b'[' as usize] = true;
    tmp[b']' as usize] = true;
    tmp[b' ' as usize] = true;
    tmp[b'\t' as usize] = true;
    tmp[b'\n' as usize] = true;
    tmp[b'\r' as usize] = true;

    tmp
};

pub trait Sealed {}

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

#[inline(always)]
pub fn cold_path() {
    #[cfg(feature = "nightly")]
    core::hint::cold_path()
}
