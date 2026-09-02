use crate::{Parser, config::Config, misc::*, source::*, value::builder::*};
use core::{hint::unreachable_unchecked, slice::from_raw_parts};

impl<'a, S: Source, C: Config> Parser<'a, S, C> {
    #[inline]
    pub(super) unsafe fn parse_value_unchecked<V: ValueBuilder<'a, S>>(&mut self) -> V {
        if S::Volatility::IS_VOLATILE {
            let tmp = self.idx();
            self.src.trim(tmp);
        }

        match self.skip_whitespace() {
            b'"' => self.parse_string_unchecked::<_, V::String>(),
            b'{' => self.parse_object_unchecked(),
            b'[' => self.parse_array_unchecked(),
            _ => self.parse_literal_unchecked(),
        }
    }

    pub(super) unsafe fn parse_object_unchecked<V: ValueBuilder<'a, S>>(&mut self) -> V {
        #[cfg(feature = "prealloc")]
        let mut obj = V::Object::with_capacity(self.prealloc);
        #[cfg(not(feature = "prealloc"))]
        let mut obj = V::Object::new();
        #[cfg(feature = "span")]
        let start = self.idx();

        loop {
            self.inc(1);
            match self.skip_whitespace() {
                b'"' => {
                    obj.on_value(self.parse_string_unchecked::<V::String, V::String>(), {
                        self.inc(1);
                        self.skip_whitespace();
                        self.inc(1);
                        self.parse_value_unchecked()
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
    pub(super) unsafe fn parse_array_unchecked<V: ValueBuilder<'a, S>>(&mut self) -> V {
        #[cfg(feature = "span")]
        let start = self.idx();
        let mut arr = V::Array::new();

        loop {
            self.inc(1);
            match self.skip_whitespace() {
                b',' => continue,
                b']' => {
                    arr.on_complete();
                    let mut tmp = arr.into();

                    #[cfg(feature = "span")]
                    tmp.apply_span(start, self.idx());
                    return tmp;
                }
                v => {
                    if S::Volatility::IS_VOLATILE {
                        let tmp = self.idx();
                        self.src.trim(tmp);
                    }

                    arr.on_value(match v {
                        b'"' => self.parse_string_unchecked::<_, V::String>(),
                        b'{' => self.parse_object_unchecked(),
                        b'[' => self.parse_array_unchecked(),
                        _ => self.parse_literal_unchecked(),
                    });
                }
            }
        }
    }

    pub(crate) unsafe fn parse_string_unchecked<T, V>(&mut self) -> T
    where
        V: StringBuilder<'a, S> + Into<T>,
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
                    let esc = self.parse_unicode_escape(tmp).unwrap_unchecked();

                    offset = self.idx() + 1;
                    buf.on_escape(esc);

                    continue;
                }
                _ => continue,
            }
        };

        buf.on_final_chunk(from_raw_parts(self.src.ptr(offset), end - offset));
        #[cfg(feature = "span")]
        buf.apply_span(start, end);

        buf.into()
    }

    #[inline]
    #[allow(unused_mut)]
    pub(super) unsafe fn parse_literal_unchecked<V: ValueBuilder<'a, S>>(&mut self) -> V {
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

            'int: {
                if is_int {
                    self.dec(1);
                    let mut tmp = if neg {
                        if val > 9223372036854775808 {
                            break 'int;
                        }

                        V::integer(val.wrapping_neg(), true)
                    } else {
                        V::integer(val, false)
                    };

                    #[cfg(feature = "span")]
                    tmp.apply_span(stamp, self.idx());
                    return tmp;
                }
            }

            let mut tmp = V::float(self.parse_f64(val, neg, start).unwrap_unchecked());
            #[cfg(feature = "span")]
            tmp.apply_span(stamp, self.idx());
            return tmp;
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
