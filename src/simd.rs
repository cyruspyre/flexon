#![allow(unused)]

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{
    _SIDD_CMP_EQUAL_ANY, _SIDD_NEGATIVE_POLARITY, _mm_and_si128, _mm_clmulepi64_si128,
    _mm_cmpeq_epi8, _mm_cmpistri, _mm_cvtsi128_si32, _mm_cvtsi128_si64, _mm_load_si128,
    _mm_loadl_epi64, _mm_loadu_si128, _mm_madd_epi16, _mm_maddubs_epi16, _mm_movemask_epi8,
    _mm_or_si128, _mm_packus_epi32, _mm_set_epi64x, _mm_set1_epi8, _mm_setr_epi8, _mm_setr_epi16,
    _mm_setzero_si128, _mm_shuffle_epi8, _mm_sub_epi8, _mm_subs_epu8, _mm_xor_si128,
    _mm256_and_si256, _mm256_cmpeq_epi8, _mm256_loadu_si256, _mm256_movemask_epi8,
    _mm256_set1_epi8, _mm256_setzero_si256, _mm256_sub_epi8, _mm256_zeroupper,
};

use crate::{
    Parser,
    config::Config,
    misc::{NON_LIT_LUT, likely},
    source::Source,
};

const ONES: u64 = 0x0101_0101_0101_0101;
const HIGH: u64 = 0x8080_8080_8080_8080;

#[inline(always)]
pub fn simd_u64(ptr: *const u8) -> Option<u64> {
    // https://lemire.me/blog/2022/01/21/swar-explained-parsing-eight-digits/

    #[cfg(all(
        target_arch = "x86_64",
        target_feature = "ssse3",
        feature = "simd",
        not(feature = "runtime-detection")
    ))]
    unsafe {
        ssse3_u64(ptr)
    }

    #[cfg(not(all(
        target_arch = "x86_64",
        target_feature = "ssse3",
        feature = "simd",
        not(feature = "runtime-detection")
    )))]
    swar_u64(ptr)
}

#[inline(always)]
unsafe fn ssse3_u64(ptr: *const u8) -> Option<u64> {
    let zero = _mm_set1_epi8(b'0' as _);
    let mul_1_10 = _mm_setr_epi8(10, 1, 10, 1, 10, 1, 10, 1, 10, 1, 10, 1, 10, 1, 10, 1);
    let mul_1_100 = _mm_setr_epi16(100, 1, 100, 1, 100, 1, 100, 1);
    let mul_1_10000 = _mm_setr_epi16(10000, 1, 10000, 1, 10000, 1, 10000, 1);
    let chunk = _mm_sub_epi8(_mm_loadl_epi64(ptr.cast()), zero);

    if _mm_movemask_epi8(_mm_cmpeq_epi8(
        _mm_subs_epu8(chunk, _mm_set1_epi8(9)),
        _mm_setzero_si128(),
    )) != 0xFF
    {
        return None;
    }

    let t1 = _mm_maddubs_epi16(chunk, mul_1_10);
    let t2 = _mm_madd_epi16(t1, mul_1_100);
    // extracts low bits in case u might be wondering
    // but again kinda feels useless considering typical cpus nowadays
    #[cfg(not(target_feature = "sse4.1"))]
    let t3 = _mm_shuffle_epi8(
        t2,
        _mm_setr_epi8(0, 1, 4, 5, 8, 9, 12, 13, -1, -1, -1, -1, -1, -1, -1, -1),
    );
    #[cfg(target_feature = "sse4.1")]
    let t3 = _mm_packus_epi32(t2, t2);
    let t4 = _mm_madd_epi16(t3, mul_1_10000);

    Some(_mm_cvtsi128_si32(t4) as u32 as _)
}

#[inline(always)]
fn swar_u64(ptr: *const u8) -> Option<u64> {
    const ZERO: u64 = 0x3030303030303030;
    const MASK: u64 = 0x000000FF000000FF;
    const MUL1: u64 = 0x000F424000000064;
    const MUL2: u64 = 0x0000271000000001;

    let mut chunk = unsafe { ptr.cast::<u64>().read_unaligned() };
    if chunk & chunk.wrapping_add(0x0606060606060606) & 0xF0F0F0F0F0F0F0F0 != ZERO {
        return None;
    }

    chunk -= ZERO;
    chunk = (chunk * 10) + (chunk >> 8);
    chunk = (chunk & MASK)
        .wrapping_mul(MUL1)
        .wrapping_add(((chunk >> 16) & MASK).wrapping_mul(MUL2))
        >> 32;

    Some(chunk)
}

#[inline(always)]
#[cfg(all(
    target_arch = "x86_64",
    target_feature = "pclmulqdq",
    feature = "simd",
    not(feature = "runtime-detection")
))]
unsafe fn compute_inside_mask(mask: u64) -> u64 {
    _mm_cvtsi128_si64(_mm_clmulepi64_si128(
        _mm_set_epi64x(0, mask as _),
        _mm_set_epi64x(0, -1),
        0,
    )) as _
}

#[inline(always)]
#[cfg(not(all(
    target_arch = "x86_64",
    target_feature = "pclmulqdq",
    feature = "simd",
    not(feature = "runtime-detection")
)))]
fn compute_inside_mask(mut mask: u64) -> u64 {
    mask ^= mask << 1;
    mask ^= mask << 2;
    mask ^= mask << 4;
    mask ^= mask << 8;
    mask ^= mask << 16;
    mask ^= mask << 32;
    mask
}

#[inline(always)]
fn compute_esc_mask(mut mask: u64, last_slash: &mut u64) -> u64 {
    // took it from `sonic_rs`. why should i torture myself?
    const ODD: u64 = 0x5555_5555_5555_5555;

    mask &= !*last_slash;
    let follows_escape = (mask << 1) | *last_slash;
    let odd_start = mask & !ODD & !follows_escape;
    let (even_start, overflow) = odd_start.overflowing_add(mask);
    let invert_mask = even_start << 1;
    *last_slash = overflow as u64;

    !((ODD ^ invert_mask) & follows_escape)
}

impl<'a, S: Source, C: Config> Parser<'a, S, C> {
    #[inline]
    pub(crate) fn simd_wh(&mut self) -> bool {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            #[cfg(all(
                target_feature = "sse4.2",
                feature = "simd",
                not(feature = "runtime-detection")
            ))]
            return self.wh_sse4_2();

            #[cfg(not(all(
                target_feature = "sse4.2",
                feature = "simd",
                not(feature = "runtime-detection")
            )))]
            self.wh_sse2()
        }

        #[cfg(not(target_arch = "x86_64"))]
        self.wh_swar()
    }

    #[inline]
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "sse4.2")]
    unsafe fn wh_sse4_2(&mut self) -> bool {
        const WHITESPACES: [u8; 16] = [
            b' ', b'\t', b'\n', b'\r', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        if S::NULL_PADDED || self.idx() + 16 < self.src.len() {
            self.inc(1);
            let idx = _mm_cmpistri(
                _mm_load_si128(WHITESPACES.as_ptr().cast()),
                _mm_loadu_si128(self.cur_ptr().cast()),
                _SIDD_NEGATIVE_POLARITY,
            );

            if idx != 16 {
                self.inc(idx as _);
                return true;
            }

            self.inc(15)
        }

        false
    }

    #[inline]
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "sse2")]
    unsafe fn wh_sse2(&mut self) -> bool {
        if S::NULL_PADDED || self.idx() + 16 < self.src.len() {
            self.inc(1);
            let chunk = _mm_loadu_si128(self.cur_ptr().cast());
            let mask = _mm_movemask_epi8(_mm_xor_si128(
                _mm_or_si128(
                    _mm_or_si128(
                        _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b' ' as _)),
                        _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'\n' as _)),
                    ),
                    _mm_or_si128(
                        _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'\t' as _)),
                        _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'\r' as _)),
                    ),
                ),
                _mm_set1_epi8(-1),
            )) as u16;

            if mask != 0 {
                self.inc(mask.trailing_zeros() as _);
                return true;
            }

            self.inc(15)
        }

        false
    }

    #[inline]
    fn wh_swar(&mut self) -> bool {
        const A: u64 = b' ' as u64 * ONES;
        const B: u64 = b'\t' as u64 * ONES;
        const C: u64 = b'\n' as u64 * ONES;
        const D: u64 = b'\r' as u64 * ONES;

        if S::NULL_PADDED || self.idx() + 8 < self.src.len() {
            self.inc(1);
            let chunk = unsafe { self.cur_ptr().cast::<u64>().read_unaligned() };

            let a = chunk ^ A;
            let b = chunk ^ B;
            let c = chunk ^ C;
            let d = chunk ^ D;

            let a = a.wrapping_sub(ONES) & !a & HIGH;
            let b = b.wrapping_sub(ONES) & !b & HIGH;
            let c = c.wrapping_sub(ONES) & !c & HIGH;
            let d = d.wrapping_sub(ONES) & !d & HIGH;

            let mask = a | b | c | d;

            if mask != HIGH {
                self.inc((!mask & HIGH).trailing_zeros() as usize >> 3);
                return true;
            }

            self.inc(7);
        }

        false
    }
}

impl<S: Source, C: Config> Parser<'_, S, C> {
    #[inline]
    pub(crate) fn simd_lit(&mut self) -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            // todo: try sse2 or ssse3
            #[cfg(feature = "runtime-detection")]
            return if is_x86_feature_detected!("sse4.2") {
                unsafe { self.lit_sse4_2() }
            } else {
                self.lit_lut()
            };

            #[cfg(not(feature = "runtime-detection"))]
            {
                #[cfg(all(feature = "simd", target_feature = "sse4.2"))]
                return unsafe { self.lit_sse4_2() };

                #[cfg(not(all(feature = "simd", target_feature = "sse4.2")))]
                return self.lit_lut();
            }
        }

        #[cfg(not(target_arch = "x86_64"))]
        self.lit_lut()
    }

    #[inline]
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "sse4.2")]
    unsafe fn lit_sse4_2(&mut self) -> bool {
        if S::NULL_PADDED || self.idx() + 16 < self.src.len() {
            const SPECIALS: [u8; 16] = [
                b'{', b'}', b'[', b']', b'"', b':', b',', b' ', b'\t', b'\n', b'\r', 0, 0, 0, 0, 0,
            ];

            let idx = _mm_cmpistri(
                _mm_load_si128(SPECIALS.as_ptr().cast()),
                _mm_loadu_si128(self.cur_ptr().add(1).cast()),
                _SIDD_CMP_EQUAL_ANY,
            );

            if idx != 16 {
                self.inc(idx as _);
                return true;
            }

            self.inc(16)
        }

        false
    }

    #[inline(always)]
    fn lit_lut(&mut self) -> bool {
        if S::NULL_PADDED || self.idx() + 2 < self.src.len() {
            let ptr = self.cur_ptr();

            let a = unsafe { *ptr.add(1) };
            let b = unsafe { *ptr.add(2) };

            let a = !NON_LIT_LUT[a as usize];
            let b = !NON_LIT_LUT[b as usize];

            if a & b {
                self.inc(2);
                return false;
            }

            self.inc(a as _)
        }

        false
    }
}

impl<S: Source, C: Config> Parser<'_, S, C> {
    #[inline]
    pub(crate) fn simd_str(&mut self) -> bool {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            self.str_sse2()
        }

        #[cfg(not(target_arch = "x86_64"))]
        self.str_swar()
    }

    #[inline]
    pub(crate) fn simd_str_unchecked(&mut self) -> bool {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            self.str_sse2_unchecked()
        }

        #[cfg(not(target_arch = "x86_64"))]
        self.str_swar_unchecked()
    }

    #[inline]
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "sse2")]
    unsafe fn str_sse2(&mut self) -> bool {
        if S::NULL_PADDED || likely(self.idx() + 16 < self.src.len()) {
            let chunk = _mm_loadu_si128(self.cur_ptr().add(1).cast());
            let mask = _mm_movemask_epi8(_mm_or_si128(
                _mm_or_si128(
                    _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'"' as _)),
                    _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'\\' as _)),
                ),
                _mm_or_si128(
                    _mm_cmpeq_epi8(
                        _mm_subs_epu8(chunk, _mm_set1_epi8(0x1F)),
                        _mm_setzero_si128(),
                    ),
                    _mm_cmpeq_epi8(chunk, _mm_set1_epi8(0x7F)),
                ),
            ));

            if mask == 0 {
                self.inc(16);
                return true;
            }

            self.inc(mask.trailing_zeros() as _)
        }

        false
    }

    #[inline]
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "sse2")]
    unsafe fn str_sse2_unchecked(&mut self) -> bool {
        if S::NULL_PADDED || likely(self.idx() + 16 < self.src.len()) {
            let chunk = _mm_loadu_si128(self.cur_ptr().add(1).cast());
            let mask = _mm_movemask_epi8(_mm_or_si128(
                _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'"' as _)),
                _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'\\' as _)),
            ));

            if mask == 0 {
                self.inc(16);
                return true;
            }

            self.inc(mask.trailing_zeros() as _)
        }

        false
    }

    #[inline]
    fn str_swar(&mut self) -> bool {
        if S::NULL_PADDED || likely(self.idx() + 8 < self.src.len()) {
            const QUOTE: u64 = b'"' as u64 * ONES;
            const SLASH: u64 = b'\\' as u64 * ONES;
            const CTRL: u64 = 0x20 * ONES;
            const DEL: u64 = 127 * ONES;

            let chunk = unsafe { self.cur_ptr().add(1).cast::<u64>().read_unaligned() };

            let ctrl = chunk.wrapping_sub(CTRL) & !chunk & HIGH;

            let tmp = chunk ^ QUOTE;
            let quote = tmp.wrapping_sub(ONES) & !tmp & HIGH;

            let tmp = chunk ^ SLASH;
            let slash = tmp.wrapping_sub(ONES) & !tmp & HIGH;

            let tmp = chunk ^ DEL;
            let del = tmp.wrapping_sub(ONES) & !tmp & HIGH;

            let mask = quote | slash | ctrl | del;

            if mask == 0 {
                self.inc(8);
                return true;
            }

            self.inc(mask.trailing_zeros() as usize >> 3)
        }

        false
    }

    #[inline]
    fn str_swar_unchecked(&mut self) -> bool {
        if S::NULL_PADDED || likely(self.idx() + 8 < self.src.len()) {
            const QUOTE: u64 = b'"' as u64 * ONES;
            const SLASH: u64 = b'\\' as u64 * ONES;

            let chunk = unsafe { self.cur_ptr().add(1).cast::<u64>().read_unaligned() };

            let tmp = chunk ^ QUOTE;
            let quote = tmp.wrapping_sub(ONES) & !tmp & HIGH;

            let tmp = chunk ^ SLASH;
            let slash = tmp.wrapping_sub(ONES) & !tmp & HIGH;

            let mask = quote | slash;

            if mask == 0 {
                self.inc(8);
                return true;
            }

            self.inc(mask.trailing_zeros() as usize >> 3)
        }

        false
    }
}

impl<S: Source, C: Config> Parser<'_, S, C> {
    /// This functions expects to be started from `'{'` or `'['`.
    pub(crate) unsafe fn skip_container_unchecked(&mut self) {
        #[cfg(target_arch = "x86_64")]
        {
            #[cfg(feature = "runtime-detection")]
            return {
                use core::{
                    hint::unreachable_unchecked,
                    sync::atomic::{AtomicU8, Ordering::Relaxed},
                };

                static FLAG: AtomicU8 = AtomicU8::new(0);

                match FLAG.load(Relaxed) {
                    2 => self.skip_container_avx2(),
                    1 => self.skip_container_sse2(),
                    0 => {
                        let tmp = if is_x86_feature_detected!("avx2") {
                            2
                        } else {
                            1
                        };

                        FLAG.store(tmp, Relaxed);
                        match tmp {
                            2 => self.skip_container_avx2(),
                            1 => self.skip_container_sse2(),
                            _ => unreachable_unchecked(),
                        }
                    }
                    _ => unreachable_unchecked(),
                }
            };

            #[cfg(not(feature = "runtime-detection"))]
            {
                #[cfg(all(feature = "simd", target_feature = "avx2"))]
                return self.skip_container_avx2();

                #[cfg(not(all(feature = "simd", target_feature = "avx2")))]
                self.skip_container_sse2()
            }
        }

        #[cfg(not(target_arch = "x86_64"))]
        self.skip_container_naive()
    }

    #[inline]
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn skip_container_avx2(&mut self) {
        let quote = _mm256_set1_epi8(b'"' as _);
        let slash = _mm256_set1_epi8(b'\\' as _);
        let mut tail = [0u8; 64];
        let mut depth = 0_usize;
        let mut in_string = 0;
        let mut last_slash = 0;

        loop {
            let ptr = if self.idx() + 64 < self.src.len() {
                self.cur_ptr()
            } else {
                // this wont run twice cause it HAS TO END after this in valid json
                let tmp = tail.as_mut_ptr();
                tmp.copy_from_nonoverlapping(self.cur_ptr(), self.src.len() - self.idx());
                tmp
            };

            let c0 = _mm256_loadu_si256(ptr.cast());
            let c1 = _mm256_loadu_si256(ptr.add(32).cast());

            let m0 = _mm256_movemask_epi8(_mm256_cmpeq_epi8(c0, quote)) as u32 as u64;
            let m1 = _mm256_movemask_epi8(_mm256_cmpeq_epi8(c1, quote)) as u32 as u64;
            let mut quote_mask = (m0 | (m1 << 32)) & !last_slash;

            let m0 = _mm256_movemask_epi8(_mm256_cmpeq_epi8(c0, slash)) as u32 as u64;
            let m1 = _mm256_movemask_epi8(_mm256_cmpeq_epi8(c1, slash)) as u32 as u64;
            let mut slash_mask = m0 | (m1 << 32);

            if slash_mask != 0 {
                let mask = slash_mask;
                slash_mask = last_slash;
                quote_mask &= compute_esc_mask(mask, &mut slash_mask);
            }

            let inside_mask = compute_inside_mask(quote_mask ^ in_string);

            // check for '{', '}', '[', ']'
            let m0 = _mm256_movemask_epi8(_mm256_cmpeq_epi8(
                _mm256_and_si256(
                    _mm256_sub_epi8(c0, _mm256_set1_epi8(91)),
                    _mm256_set1_epi8(-35),
                ),
                _mm256_setzero_si256(),
            )) as u32 as u64;
            let m1 = _mm256_movemask_epi8(_mm256_cmpeq_epi8(
                _mm256_and_si256(
                    _mm256_sub_epi8(c1, _mm256_set1_epi8(91)),
                    _mm256_set1_epi8(-35),
                ),
                _mm256_setzero_si256(),
            )) as u32 as u64;
            let sp_mask = m0 | (m1 << 32);

            let mut mask = sp_mask & !inside_mask;

            in_string = inside_mask >> 63;
            last_slash = slash_mask;

            while mask != 0 {
                let idx = mask.trailing_zeros() as _;
                match *self.cur_ptr().add(idx) {
                    b'{' | b'[' => depth += 1,
                    _ => {
                        depth -= 1;
                        if depth == 0 {
                            _mm256_zeroupper();
                            return self.inc(idx);
                        }
                    }
                }
                mask &= mask - 1;
            }

            self.inc(64)
        }
    }

    #[inline]
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "sse2")]
    unsafe fn skip_container_sse2(&mut self) {
        let quote = _mm_set1_epi8(b'"' as _);
        let slash = _mm_set1_epi8(b'\\' as _);
        let mut tail = [0u8; 64];
        let mut depth = 0_usize;
        let mut in_string = 0;
        let mut last_slash = 0;

        loop {
            let ptr = if self.idx() + 64 < self.src.len() {
                self.cur_ptr()
            } else {
                let tmp = tail.as_mut_ptr();
                tmp.copy_from_nonoverlapping(self.cur_ptr(), self.src.len() - self.idx());
                tmp
            };

            let c0 = _mm_loadu_si128(ptr.cast());
            let c1 = _mm_loadu_si128(ptr.add(16).cast());
            let c2 = _mm_loadu_si128(ptr.add(32).cast());
            let c3 = _mm_loadu_si128(ptr.add(48).cast());

            let m0 = _mm_movemask_epi8(_mm_cmpeq_epi8(c0, quote)) as u32 as u64;
            let m1 = _mm_movemask_epi8(_mm_cmpeq_epi8(c1, quote)) as u32 as u64;
            let m2 = _mm_movemask_epi8(_mm_cmpeq_epi8(c2, quote)) as u32 as u64;
            let m3 = _mm_movemask_epi8(_mm_cmpeq_epi8(c3, quote)) as u32 as u64;
            let mut quote_mask = (m0 | (m1 << 16) | (m2 << 32) | (m3 << 48)) & !last_slash;

            let m0 = _mm_movemask_epi8(_mm_cmpeq_epi8(c0, slash)) as u32 as u64;
            let m1 = _mm_movemask_epi8(_mm_cmpeq_epi8(c1, slash)) as u32 as u64;
            let m2 = _mm_movemask_epi8(_mm_cmpeq_epi8(c2, slash)) as u32 as u64;
            let m3 = _mm_movemask_epi8(_mm_cmpeq_epi8(c3, slash)) as u32 as u64;
            let mut slash_mask = m0 | (m1 << 16) | (m2 << 32) | (m3 << 48);

            if slash_mask != 0 {
                let mask = slash_mask;
                slash_mask = last_slash;
                quote_mask &= compute_esc_mask(mask, &mut slash_mask);
            }

            let inside_mask = compute_inside_mask(quote_mask ^ in_string);

            let m0 = _mm_movemask_epi8(_mm_cmpeq_epi8(
                _mm_and_si128(_mm_sub_epi8(c0, _mm_set1_epi8(91)), _mm_set1_epi8(-35)),
                _mm_setzero_si128(),
            )) as u32 as u64;
            let m1 = _mm_movemask_epi8(_mm_cmpeq_epi8(
                _mm_and_si128(_mm_sub_epi8(c1, _mm_set1_epi8(91)), _mm_set1_epi8(-35)),
                _mm_setzero_si128(),
            )) as u32 as u64;
            let m2 = _mm_movemask_epi8(_mm_cmpeq_epi8(
                _mm_and_si128(_mm_sub_epi8(c2, _mm_set1_epi8(91)), _mm_set1_epi8(-35)),
                _mm_setzero_si128(),
            )) as u32 as u64;
            let m3 = _mm_movemask_epi8(_mm_cmpeq_epi8(
                _mm_and_si128(_mm_sub_epi8(c3, _mm_set1_epi8(91)), _mm_set1_epi8(-35)),
                _mm_setzero_si128(),
            )) as u32 as u64;
            let sp_mask = m0 | (m1 << 16) | (m2 << 32) | (m3 << 48);

            let mut mask = sp_mask & !inside_mask;

            in_string = inside_mask >> 63;
            last_slash = slash_mask;

            while mask != 0 {
                let idx = mask.trailing_zeros() as _;
                match *self.cur_ptr().add(idx) {
                    b'{' | b'[' => depth += 1,
                    _ => {
                        depth -= 1;
                        if depth == 0 {
                            return self.inc(idx);
                        }
                    }
                }
                mask &= mask - 1;
            }

            self.inc(64)
        }
    }

    #[inline]
    unsafe fn skip_container_naive(&mut self) {
        let mut depth = 0_usize;
        let mut in_string = false;

        loop {
            self.inc(1);
            match *self.cur_ptr().sub(1) {
                b'\\' => self.inc(1),
                b'"' => in_string ^= true,
                _ if in_string => continue,
                b'{' | b'[' => depth += 1,
                b'}' | b']' => {
                    depth -= 1;
                    if depth == 0 {
                        return;
                    }
                }
                _ => continue,
            }
        }
    }

    // todo: im unable to make it work when it includes slashes. things ive tried-
    //       - unpack/repack and use `compute_esc_mask`. doesnt work, doing so has
    //         different carry behavior.
    //       - try to make another function based on `compute_esc_mask` but for swar.
    //         failed. i got cooked in the carry behavior for swar or wtv the hell that is.
    //
    // unsafe fn skip_container_swar(&mut self) {
    //     const ONES: u64 = 0x0101_0101_0101_0101;
    //     const HIGH: u64 = 0x8080_8080_8080_8080;
    //     const QUOTE: u64 = b'"' as u64 * ONES;
    //     const SLASH: u64 = b'\\' as u64 * ONES;
    //
    //     let mut tail = [0u8; 8];
    //     let mut depth = 0_usize;
    //     let mut in_string = 0;
    //     let mut last_slash = 0;
    //
    //     loop {
    //         let ptr = if self.idx() + 8 < self.src.len() {
    //             self.cur_ptr()
    //         } else {
    //             let tmp = tail.as_mut_ptr();
    //             tmp.copy_from_nonoverlapping(self.cur_ptr(), self.src.len() - self.idx());
    //             tmp
    //         };
    //
    //         let chunk = ptr.cast::<u64>().read_unaligned();
    //
    //         let tmp = chunk ^ QUOTE;
    //         let quote = (tmp.wrapping_sub(ONES) & !tmp & HIGH) & !last_slash;
    //
    //         let tmp = chunk ^ SLASH;
    //         let slash = tmp.wrapping_sub(ONES) & !tmp & HIGH;
    //
    //         if slash != 0 {
    //             todo!()
    //         }
    //
    //         let mut inside = quote ^ in_string;
    //
    //         inside ^= inside << 8;
    //         inside ^= inside << 16;
    //         inside ^= inside << 32;
    //
    //         // let sp = (chunk.wrapping_add(HIGH).wrapping_sub(0x5B5B_5B5B_5B5B_5B5B) ^ HIGH)
    //         //     & 0xDDDD_DDDD_DDDD_DDDD;
    //         let tmp = (chunk.wrapping_add(0x2525_2525_2525_2525) ^ HIGH) & 0xDDDD_DDDD_DDDD_DDDD;
    //         let sp = tmp.wrapping_sub(ONES) & !tmp & HIGH;
    //
    //         let mut mask = sp & !inside;
    //
    //         in_string = inside >> 56;
    //         // last_slash = slash;
    //
    //         while mask != 0 {
    //             let idx = mask.trailing_zeros() as usize >> 3;
    //             match *self.cur_ptr().add(idx) {
    //                 b'{' | b'[' => depth += 1,
    //                 _ => {
    //                     depth -= 1;
    //                     if depth == 0 {
    //                         return self.inc(idx);
    //                     }
    //                 }
    //             }
    //             mask &= mask - 1;
    //         }
    //
    //         self.inc(8)
    //     }
    // }
}
