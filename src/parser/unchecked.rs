use core::{hint::unreachable_unchecked, slice::from_raw_parts};

use crate::{Parser, config::Config, misc::*, source::*, value::builder::*};

impl<'a, S: Source, C: Config> Parser<'a, S, C> {
    #[inline]
    pub(super) unsafe fn value_unchecked<V: ValueBuilder<'a, S>>(&mut self) -> V {
        if S::Volatility::IS_VOLATILE {
            let tmp = self.idx().wrapping_add(1);
            self.src.trim(tmp);
        }

        unsafe {
            match self.skip_whitespace() {
                b'"' => self.string_unchecked::<_, V::String, V::Error>(),
                b'{' => self.object_unchecked::<_, V::Object, V::Error>(),
                b'[' => self.array_unchecked(),
                _ => self.literal_unchecked(),
            }
        }
    }

    pub(super) unsafe fn object_unchecked<T: ValueBuilder<'a, S>, V, E>(&mut self) -> T
    where
        V: ObjectBuilder<'a, S, E> + Into<T>,
        E: ErrorBuilder,
    {
        #[cfg(feature = "prealloc")]
        let mut obj = V::with_capacity(self.prealloc);
        #[cfg(not(feature = "prealloc"))]
        let mut obj = V::new();
        #[cfg(feature = "span")]
        let start = self.idx();

        loop {
            match self.skip_whitespace() {
                b'"' => {
                    obj.on_value(self.string_unchecked::<_, V::Key, E>(), {
                        self.skip_whitespace();
                        self.value_unchecked()
                    });

                    continue;
                }
                b',' => continue,
                b'}' => {
                    obj.on_complete();

                    #[cfg(feature = "prealloc")]
                    (self.prealloc = obj.len());
                    #[allow(unused_mut)]
                    let mut tmp = obj.into();

                    #[cfg(feature = "span")]
                    tmp.apply_span(start, self.idx());
                    return tmp;
                }
                _ => unreachable_unchecked(),
            };
        }
    }

    #[allow(unused_mut)]
    pub(super) unsafe fn array_unchecked<V: ValueBuilder<'a, S>>(&mut self) -> V {
        #[cfg(feature = "span")]
        let start = self.idx();
        let mut arr = V::Array::new();

        loop {
            match self.skip_whitespace() {
                b',' => continue,
                b']' => {
                    arr.on_complete();
                    let mut tmp = arr.into();

                    #[cfg(feature = "span")]
                    tmp.apply_span(start, self.idx());
                    return tmp;
                }
                _ => {
                    self.dec();
                    arr.on_value(self.value_unchecked());
                }
            }
        }
    }

    pub(super) unsafe fn string_unchecked<T, V, E>(&mut self) -> T
    where
        V: StringBuilder<'a, S, E> + Into<T>,
        E: ErrorBuilder,
    {
        let start = self.idx();
        let mut offset = start + 1;
        let mut buf = V::new();
        let end = loop {
            if self.simd_str_unchecked() {
                continue;
            }

            self.inc(1);
            match self.cur() {
                b'"' => break self.idx(),
                b'\\' => {
                    buf.on_chunk(from_raw_parts(self.src.ptr(offset), self.idx() - offset));

                    self.inc(1);
                    offset = self.idx() + 1;

                    let tmp = self.cur();
                    let esc = ESC_LUT[tmp as usize];

                    if esc != 0 {
                        buf.on_escape(&[esc]);
                        continue;
                    }

                    let tmp = &mut [0; 4];
                    let esc = self.unicode_escape(tmp).unwrap_unchecked();

                    offset = self.idx() + 1;
                    buf.on_escape(esc);

                    continue;
                }
                _ => continue,
            }
        };

        buf.on_final_chunk(from_raw_parts(self.src.ptr(offset), end - offset));
        buf.on_complete(from_raw_parts(self.src.ptr(start + 1), end - start - 1))
            .unwrap_unchecked();
        #[cfg(feature = "span")]
        buf.apply_span(start, end);

        buf.into()
    }

    #[inline]
    #[allow(unused_mut)]
    pub(super) unsafe fn literal_unchecked<V: ValueBuilder<'a, S>>(&mut self) -> V {
        if V::CUSTOM_LITERAL {
            let start = self.idx();
            let end = loop {
                self.inc(1);

                if (S::NULL_PADDED || self.idx() >= self.src.len())
                    || NON_LIT_LUT[self.cur() as usize]
                {
                    self.dec();
                    break self.idx();
                }

                if self.simd_lit() {
                    break self.idx();
                }
            };

            return V::literal(from_raw_parts(self.src.ptr(start), end - start + 1))
                .unwrap_unchecked();
        }

        #[cfg(feature = "span")]
        let stamp = self.idx();
        let tmp = self.cur();

        if NUM_LUT[tmp as usize] {
            let neg = tmp == b'-';
            if neg {
                self.inc(1)
            }

            let start = self.idx();
            let (val, is_int) = self.parse_u64();
            let mut num = match is_int {
                true => V::integer(
                    match neg {
                        true => val.wrapping_neg(),
                        _ => val,
                    },
                    neg,
                ),
                _ => V::float(self.parse_f64(val, neg, start).unwrap_unchecked()),
            };

            self.dec();
            #[cfg(feature = "span")]
            num.apply_span(stamp, self.idx());
            return num;
        }

        let mut tmp = match self.cur() {
            b'n' => V::null(),
            c => {
                let val = c == b't';
                self.inc(!val as _);
                V::bool(val)
            }
        };

        self.inc(3);
        #[cfg(feature = "span")]
        tmp.apply_span(stamp, self.idx());
        tmp
    }
}
