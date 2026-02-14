mod skip;
mod skip_to;
mod unchecked;

use core::{
    hint::select_unpredictable,
    marker::PhantomData,
    slice::from_raw_parts,
    str::{from_utf8_unchecked, from_utf8_unchecked_mut},
};
use std::io::Read;

use crate::{
    config::{CTConfig, Config},
    misc::*,
    pointer::JsonPointer,
    simd::simd_u64,
    source::*,
    value::builder::*,
};

#[cfg(feature = "comment")]
use crate::Comment;

// todo: trim source when skipping values

/// JSON parser structure.
pub struct Parser<'a, S: Source, C: Config = CTConfig> {
    pub(crate) src: S,
    pub(crate) cfg: C,
    cur: Cur,
    #[cfg(feature = "prealloc")]
    prealloc: usize,
    #[cfg(feature = "comment")]
    comments: Vec<Comment<'a>>,
    __: PhantomData<&'a ()>,
}

// represents the current byte offset.
union Cur {
    idx: usize,
    // "pinned" pointer from non volatile source.
    ptr: *mut u8,
}

impl<'a, S: Source, C: Config> Parser<'a, S, C> {
    /// Create a parser with the given source and configuration.
    ///
    /// # Example
    /// ```
    /// use flexon::{Parser, config::CTConfig};
    ///
    /// let parser = Parser::new_with(
    ///     r#"{"key": "value",}"#,
    ///     CTConfig::new().allow_trailing_comma(),
    /// );
    /// ```
    #[inline]
    pub fn new_with(mut src: S, cfg: C) -> Self {
        const {
            assert!(
                !(S::NULL_PADDED && S::Volatility::IS_VOLATILE),
                "if the source is null padded then it must be non volatile"
            );

            assert!(
                !(S::INSITU && S::Volatility::IS_VOLATILE),
                "if the source enables in-situ parsing then it must be non volatile"
            );
        }

        Self {
            #[cfg(feature = "prealloc")]
            prealloc: 0,
            #[cfg(feature = "comment")]
            comments: Vec::new(),
            cur: match S::NULL_PADDED {
                true => Cur {
                    ptr: match S::INSITU {
                        true => unsafe { src.ptr_mut(0).sub(1) },
                        // not actually mutating
                        _ => unsafe { src.ptr(0).sub(1).cast_mut() },
                    },
                },
                _ => Cur { idx: usize::MAX },
            },
            __: PhantomData,
            src,
            cfg,
        }
    }

    /// Replaces the parser's configuration with a new one.
    ///
    /// # Example
    /// ```
    /// use flexon::{Parser, Value, config::RTConfig};
    ///
    /// let parser: Value<'_> = Parser::from_slice(br#"[42, 68]"#)
    ///     .with_config(RTConfig::new())
    ///     .parse()
    ///     .unwrap();
    /// ```
    #[inline]
    pub fn with_config<N: Config>(self, cfg: N) -> Parser<'a, S, N> {
        Parser {
            cfg,
            cur: self.cur,
            src: self.src,
            __: PhantomData,
            #[cfg(feature = "comment")]
            comments: self.comments,
            #[cfg(feature = "prealloc")]
            prealloc: 0,
        }
    }

    /// Parses JSON into the specified type.
    ///
    /// Unlike serde's deserialization API, this method is specifically for parsing
    /// arbitrary JSON values that implement the [`ValueBuilder`] trait.
    ///
    /// # Example
    /// ```
    /// use flexon::{Parser, Value};
    ///
    /// let val: Value<'_> = Parser::from_str(r#"[101, 201]"#).parse().unwrap();
    /// ```
    #[inline]
    pub fn parse<V: ValueBuilder<'a, S>>(&mut self) -> Result<V, V::Error> {
        const {
            assert!(
                !(V::LAZY & S::Volatility::IS_VOLATILE),
                "source must be non volatile if the value builder is lazy"
            )
        }

        if V::LAZY {
            // "mom, can we have json skipper??"
            // "no we have json skipper at home"
            self.skip_value()?;
            Ok(unsafe { V::raw(from_raw_parts(self.src.ptr(0), self.src.len())) })
        } else {
            self.value()
        }
    }

    /// Parses JSON into the specified type.
    ///
    /// Similar to [`Parser::parse`] but this won't perform any validation.
    /// The JSON must be valid otherwise there is no guarantee of this function
    ///
    /// # Example
    /// ```
    /// use flexon::{Parser, Value};
    ///
    /// let val: Value = unsafe { Parser::from_str(r#"[101, 201]"#).parse_unchecked() };
    ///
    /// assert_eq!(val[0].as_u64(), Some(101))
    /// ```
    #[inline]
    pub unsafe fn parse_unchecked<V: ValueBuilder<'a, S>>(&mut self) -> V {
        const {
            assert!(
                !(V::LAZY & S::Volatility::IS_VOLATILE),
                "source must be non volatile if the value builder is lazy"
            )
        }

        if V::LAZY {
            // omg so fast
            V::raw(from_raw_parts(self.src.ptr(0), self.src.len()))
        } else {
            self.value_unchecked()
        }
    }

    /// Skips to the given path and parses JSON into the specified type.
    ///
    /// This will return early as soon as it finishes parsing. As such, any trailing data
    /// is ignored. If the path does not exist then it will return error.
    ///
    /// Unlike serde's deserialization API, this method is specifically for parsing
    /// arbitrary JSON values that implement the [`ValueBuilder`] trait.
    ///
    /// # Example
    /// ```
    /// use flexon::{Parser, Value};
    ///
    /// let val: Value = Parser::from_str(r#"[101, 201]"#).parse_at([1]).unwrap();
    ///
    /// assert_eq!(val.as_u64(), Some(201))
    /// ```
    pub fn parse_at<V, P>(&mut self, p: P) -> Result<V, V::Error>
    where
        V: ValueBuilder<'a, S>,
        P: IntoIterator,
        P::Item: JsonPointer,
    {
        const {
            assert!(
                !(V::LAZY & S::Volatility::IS_VOLATILE),
                "source must be non volatile if the value builder is lazy"
            )
        }

        unsafe {
            let char = self.skip_to(p)?;

            if V::LAZY {
                // source is non volatile
                let start = self.cur_ptr();
                match char {
                    b'"' => self.skip_string(),
                    b'{' => self.skip_object(),
                    b'[' => self.skip_array(),
                    0 => return Err(V::Error::expected_value()),
                    _ => self.skip_literal(),
                }?;

                Ok(V::raw(from_raw_parts(
                    start,
                    self.cur_ptr().offset_from_unsigned(start) + 1,
                )))
            } else {
                match char {
                    b'"' => self.string::<_, V::String, _>(),
                    b'{' => self.object::<_, V::Object, _>(),
                    b'[' => self.array(),
                    0 => {
                        #[allow(unused_mut)]
                        let mut tmp = V::Error::expected_value();
                        #[cfg(feature = "span")]
                        tmp.apply_span(self.idx(), self.idx());
                        Err(tmp)
                    }
                    _ => self.literal(),
                }
            }
        }
    }

    /// Skips to the given path and parses JSON into the specified type.
    ///
    /// Same as [`Parser::parse_at`] but wihout validation. There is no
    /// guarantee if the JSON is invalid or the path does not exist.
    ///
    /// # Example
    /// ```
    /// use flexon::{Parser, Value};
    ///
    /// let val: Value = unsafe { Parser::from_str(r#"[101, 201]"#).parse_at_unchecked([1]) };
    ///
    /// assert_eq!(val.as_u64(), Some(201))
    /// ```
    pub unsafe fn parse_at_unchecked<V, P>(&mut self, p: P) -> V
    where
        V: ValueBuilder<'a, S>,
        P: IntoIterator,
        P::Item: JsonPointer,
    {
        const {
            assert!(
                !(V::LAZY & S::Volatility::IS_VOLATILE),
                "source must be non volatile if the value builder is lazy"
            )
        }

        let char = self.skip_to_unchecked(p);

        if V::LAZY {
            V::raw(from_raw_parts(self.cur_ptr(), self.src.len() - self.idx()))
        } else {
            match char {
                b'"' => self.string_unchecked::<_, V::String, _>(),
                b'{' => self.object_unchecked::<_, V::Object, _>(),
                b'[' => self.array_unchecked(),
                _ => self.literal_unchecked(),
            }
        }
    }

    /// Consumes the parser and returns the accumulated comments.
    ///
    /// # Example
    /// ```
    /// use flexon::{Parser, config::CTConfig};
    /// use serde::Deserialize;
    ///
    /// let config = CTConfig::new().allow_comments();
    /// let mut parser = Parser::from_str("/* foo bar */ 123").with_config(config);
    ///
    /// assert_eq!(u8::deserialize(&mut parser)?, 123);
    /// assert_eq!(parser.take_comments()[0].as_str(), " foo bar ");
    ///
    /// # Ok::<(), flexon::serde::Error>(())
    /// ```
    #[inline]
    #[cfg(feature = "comment")]
    pub fn take_comments(self) -> Vec<Comment<'a>> {
        self.comments
    }

    #[inline(always)]
    pub(crate) fn inc(&mut self, n: usize) {
        unsafe {
            match S::NULL_PADDED {
                true => self.cur.ptr = self.cur.ptr.add(n),
                _ => self.cur.idx = self.cur.idx.wrapping_add(n),
            }
        }
    }

    #[inline(always)]
    pub(crate) fn dec(&mut self) {
        unsafe {
            match S::NULL_PADDED {
                true => self.cur.ptr = self.cur.ptr.sub(1),
                _ => self.cur.idx = self.cur.idx.wrapping_sub(1),
            }
        }
    }

    #[inline(always)]
    pub(crate) fn idx(&mut self) -> usize {
        unsafe {
            match S::NULL_PADDED {
                true => self.cur.ptr.offset_from_unsigned(self.src.ptr(0)),
                _ => self.cur.idx,
            }
        }
    }

    #[inline(always)]
    pub(crate) fn cur_ptr(&mut self) -> *const u8 {
        unsafe {
            match S::NULL_PADDED {
                true => self.cur.ptr,
                _ => self.src.ptr(self.cur.idx),
            }
        }
    }

    #[inline(always)]
    pub(crate) fn cur_ptr_mut(&mut self) -> *mut u8 {
        unsafe {
            match S::NULL_PADDED {
                true => self.cur.ptr,
                _ => self.src.ptr_mut(self.cur.idx),
            }
        }
    }

    #[inline(always)]
    pub(crate) fn cur(&mut self) -> u8 {
        unsafe { *self.cur_ptr() }
    }

    pub(crate) fn skip_whitespace(&mut self) -> u8 {
        let mut fast = false;

        loop {
            if match S::NULL_PADDED {
                true => unsafe { *self.cur_ptr().add(1) == 0 },
                _ => self.idx().wrapping_add(1) >= self.src.len(),
            } {
                return 0;
            }

            self.inc(1);
            let tmp = self.cur();

            if !matches!(tmp, b' ' | b'\t' | b'\n' | b'\r') {
                #[cfg(feature = "comment")]
                if tmp == b'/' && self.cfg.comments() {
                    self.comment();
                    continue;
                }

                return tmp;
            }

            if fast && self.simd_wh() {
                #[cfg(feature = "comment")]
                if self.cur() == b'/' && self.cfg.comments() {
                    self.comment();
                    continue;
                }

                return self.cur();
            }

            fast = true;
        }
    }

    #[cfg(feature = "comment")]
    fn comment(&mut self) {
        // i dont think its worth adding simd here
        if match S::NULL_PADDED {
            true => unsafe { *self.cur_ptr().add(1) == 0 },
            _ => self.idx() + 1 == self.src.len(),
        } {
            return;
        }

        self.inc(1);
        let mut multi = false;
        let stamp = self.idx() + 1;

        match self.cur() {
            b'/' => loop {
                if match S::NULL_PADDED {
                    true => unsafe { *self.cur_ptr().add(1) == 0 },
                    _ => self.idx() + 1 == self.src.len(),
                } {
                    return;
                }

                self.inc(1);
                if self.cur() == b'\n' {
                    break;
                }
            },
            b'*' => loop {
                if match S::NULL_PADDED {
                    true => unsafe { *self.cur_ptr().add(1) == 0 },
                    _ => self.idx() + 1 == self.src.len(),
                } {
                    return;
                }

                self.inc(1);
                if self.cur() == b'*'
                    && match S::NULL_PADDED {
                        true => unsafe { *self.cur_ptr().add(1) != b'0' },
                        _ => self.idx() + 1 != self.src.len(),
                    }
                    && unsafe { *self.cur_ptr().add(1) == b'/' }
                {
                    multi = true;
                    self.inc(1);
                    break;
                }
            },
            _ => {
                self.dec();
                return;
            }
        }

        let idx = self.idx();
        let len = idx - stamp - multi as usize;
        let src = self.src.ptr(stamp).cast_mut();

        self.comments.push(Comment::new(
            src,
            len,
            multi,
            // omits checking non zero len when (de)allocating
            S::Volatility::IS_VOLATILE && len != 0,
            #[cfg(feature = "span")]
            [stamp - 2, idx - !multi as usize],
        ))
    }

    #[inline]
    fn value<V: ValueBuilder<'a, S>>(&mut self) -> Result<V, V::Error> {
        if S::Volatility::IS_VOLATILE {
            let tmp = self.idx().wrapping_add(1);
            self.src.trim(tmp);
        }

        unsafe {
            match self.skip_whitespace() {
                b'"' => self.string::<_, V::String, V::Error>(),
                b'{' => self.object::<_, V::Object, V::Error>(),
                b'[' => self.array(),
                0 => {
                    #[allow(unused_mut)]
                    let mut tmp = V::Error::expected_value();
                    #[cfg(feature = "span")]
                    tmp.apply_span(self.idx(), self.idx());
                    Err(tmp)
                }
                _ => self.literal(),
            }
        }
    }

    #[allow(unused_mut)]
    unsafe fn object<T, V, E>(&mut self) -> Result<T, E>
    where
        T: ValueBuilder<'a, S>,
        V: ObjectBuilder<'a, S, E> + Into<T>,
        E: ErrorBuilder,
    {
        #[cfg(feature = "span")]
        let start = self.idx();
        #[cfg(feature = "prealloc")]
        let mut obj = V::with_capacity(self.prealloc);
        #[cfg(not(feature = "prealloc"))]
        let mut obj = V::new();
        let mut tmp = self.skip_whitespace();

        if tmp == b'}' {
            obj.on_complete();
            let mut tmp = obj.into();

            #[cfg(feature = "span")]
            tmp.apply_span(start, self.idx());
            return Ok(tmp);
        }

        let mut err = loop {
            if tmp != b'"' {
                break E::unexpected_token();
            }

            let key = self.string::<_, V::Key, E>()?;
            if self.skip_whitespace() != b':' {
                break E::expected_colon();
            }

            obj.on_value(key, self.value()?);
            tmp = self.skip_whitespace();
            let comma = tmp == b',';
            if comma {
                tmp = self.skip_whitespace();
            }

            if tmp == b'}' {
                if !comma || self.cfg.trailing_comma() {
                    obj.on_complete();

                    #[cfg(feature = "prealloc")]
                    (self.prealloc = obj.len());
                    #[allow(unused_mut)]
                    let mut tmp = obj.into();

                    #[cfg(feature = "span")]
                    tmp.apply_span(start, self.idx());
                    return Ok(tmp);
                }

                #[cfg(feature = "span")]
                self.dec();
                break E::trailing_comma();
            }

            if comma || self.cfg.comma() {
                continue;
            }

            break match tmp {
                0 => E::eof(),
                _ => E::unexpected_token(),
            };
        };

        #[cfg(feature = "span")]
        err.apply_span(self.idx(), self.idx());
        cold_path();
        Err(err)
    }

    #[allow(unused_mut)]
    unsafe fn array<V: ValueBuilder<'a, S>>(&mut self) -> Result<V, V::Error> {
        #[cfg(feature = "span")]
        let start = self.idx();
        let mut arr = V::Array::new();
        let mut tmp = self.skip_whitespace();

        if tmp == b']' {
            arr.on_complete();
            let mut tmp = arr.into();

            #[cfg(feature = "span")]
            tmp.apply_span(start, self.idx());
            return Ok(tmp);
        }

        let mut err = loop {
            arr.on_value(match tmp {
                b'"' => self.string::<_, V::String, _>(),
                b'{' => self.object::<_, V::Object, _>(),
                b'[' => self.array(),
                0 => {
                    #[allow(unused_mut)]
                    let mut err = V::Error::eof();
                    #[cfg(feature = "span")]
                    err.apply_span(self.idx(), self.idx());
                    return Err(err);
                }
                _ => self.literal(),
            }?);
            tmp = self.skip_whitespace();
            let comma = tmp == b',';

            if comma {
                tmp = self.skip_whitespace();
            }

            if tmp == b']' {
                if !comma || self.cfg.trailing_comma() {
                    arr.on_complete();
                    let mut tmp = arr.into();

                    #[cfg(feature = "span")]
                    tmp.apply_span(start, self.idx());
                    return Ok(tmp);
                }

                #[cfg(feature = "span")]
                self.dec();
                break V::Error::trailing_comma();
            }

            if comma || self.cfg.comma() {
                continue;
            }

            break match tmp {
                0 => V::Error::eof(),
                _ => V::Error::unexpected_token(),
            };
        };

        #[cfg(feature = "span")]
        err.apply_span(self.idx(), self.idx());
        cold_path();
        Err(err)
    }

    unsafe fn string<T, V, E>(&mut self) -> Result<T, E>
    where
        V: StringBuilder<'a, S, E> + Into<T>,
        E: ErrorBuilder,
    {
        let start = self.idx();
        let mut offset = start + 1;
        let mut buf = V::new();
        let end = 'main: {
            let err = loop {
                if self.simd_str() {
                    continue;
                }

                self.inc(1);
                if !S::NULL_PADDED && self.idx() >= self.src.len() {
                    break E::unclosed_string();
                }

                break match self.cur() {
                    b'"' => break 'main self.idx(),
                    b'\\' => {
                        buf.on_chunk(from_raw_parts(self.src.ptr(offset), self.idx() - offset));

                        self.inc(1);
                        offset = self.idx() + 1;

                        if !S::NULL_PADDED && self.idx() == self.src.len() {
                            break E::unclosed_string();
                        }

                        let tmp = self.cur();
                        let esc = ESC_LUT[tmp as usize];

                        if esc != 0 {
                            buf.on_escape(&[esc]);
                            continue;
                        }

                        if tmp == b'u'
                            && let Some(v) = self.unicode_escape(&mut [0; 4])
                        {
                            offset = self.idx() + 1;
                            buf.on_escape(v);
                            continue;
                        }

                        if V::REJECT_INVALID_ESCAPE {
                            break E::invalid_escape();
                        }

                        continue;
                    }
                    c if !c.is_ascii_control() => continue,
                    0 if S::NULL_PADDED => E::eof(),
                    _ => match V::REJECT_CTRL_CHAR {
                        true => E::control_character(),
                        _ => continue,
                    },
                };
            };
            #[allow(unused_mut)]
            let mut err = self.close_string(err);

            #[cfg(feature = "span")]
            err.apply_span(start, self.idx());

            return Err(err);
        };
        let raw = from_raw_parts(self.src.ptr(start + 1), end - start - 1);

        if !S::UTF8 && simdutf8::basic::from_utf8(raw).is_err() {
            #[allow(unused_mut)]
            let mut err = E::unexpected_token();

            #[cfg(feature = "span")]
            err.apply_span(start, end);

            return Err(err);
        }

        buf.on_final_chunk(from_raw_parts(self.src.ptr(offset), end - offset));
        buf.on_complete(raw)?;
        #[cfg(feature = "span")]
        buf.apply_span(start, end);

        Ok(buf.into())
    }

    // cba
    #[inline(never)]
    pub(crate) unsafe fn unicode_escape<'esc>(
        &mut self,
        buf: &'esc mut [u8; 4],
    ) -> Option<&'esc [u8]> {
        self.inc(4);
        if !S::NULL_PADDED && self.idx() >= self.src.len() {
            match S::NULL_PADDED {
                true => self.cur.ptr = self.cur.ptr.sub(4),
                _ => self.cur.idx = self.cur.idx.wrapping_sub(4),
            }
            return None;
        }

        let mut codepoint = match u16::from_str_radix(
            from_utf8_unchecked(from_raw_parts(self.cur_ptr().sub(3), 4)),
            16,
        ) {
            Ok(v) => v as u32,
            _ => return None,
        };

        if (0xD800..=0xDBFF).contains(&codepoint) {
            self.inc(6);
            if !S::NULL_PADDED && self.idx() >= self.src.len()
                || from_raw_parts(self.cur_ptr().sub(5), 2) != br"\u"
            {
                match S::NULL_PADDED {
                    true => self.cur.ptr = self.cur.ptr.sub(6),
                    _ => self.cur.idx = self.cur.idx.wrapping_sub(6),
                }
                return None;
            }

            let low = match u16::from_str_radix(
                from_utf8_unchecked(from_raw_parts(self.cur_ptr().sub(3), 4)),
                16,
            ) {
                Ok(v) => v as u32,
                _ => return None,
            };

            if !(0xDC00..=0xDFFF).contains(&low) {
                return None;
            }

            codepoint = 0x10000 + (((codepoint - 0xD800) << 10) | (low - 0xDC00));
        }

        char::from_u32(codepoint).map(|v| v.encode_utf8(buf).as_bytes())
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn close_string<E: ErrorBuilder>(&mut self, with: E) -> E {
        let mut flag = true;

        loop {
            if match S::NULL_PADDED {
                true => unsafe { *self.cur_ptr().add(1) == 0 },
                _ => self.idx() + 1 >= self.src.len(),
            } {
                return E::unclosed_string();
            }

            self.inc(1);
            flag = match self.cur() {
                b'"' if flag => return with,
                v => v != b'\\',
            };
        }
    }

    #[inline]
    #[allow(unused_mut)]
    unsafe fn literal<V: ValueBuilder<'a, S>>(&mut self) -> Result<V, V::Error> {
        if V::CUSTOM_LITERAL {
            const EXCLUDED: [bool; 256] = {
                let mut tmp = [false; 256];

                tmp[b':' as usize] = true;
                tmp[b',' as usize] = true;
                tmp[b'}' as usize] = true;
                tmp[b']' as usize] = true;

                tmp
            };

            if EXCLUDED[self.cur() as usize] {
                #[allow(unused_mut)]
                let mut tmp = V::Error::unexpected_token();
                #[cfg(feature = "span")]
                tmp.apply_span(self.idx(), self.idx());
                return Err(tmp);
            }

            let start = self.idx();
            let end = loop {
                self.inc(1);

                if (!S::NULL_PADDED && self.idx() >= self.src.len())
                    || NON_LIT_LUT[self.cur() as usize]
                {
                    self.dec();
                    break self.idx();
                }

                if self.simd_lit() {
                    break self.idx();
                }
            };

            return V::literal(from_raw_parts(self.src.ptr(start), end - start + 1));
        }

        #[cfg(feature = "span")]
        let stamp = self.idx();
        let tmp = self.cur();

        if NUM_LUT[tmp as usize] {
            let neg = tmp == b'-';
            if neg {
                self.inc(1)
            }

            if unlikely(
                !S::NULL_PADDED && self.idx() == self.src.len() || !NUM_LUT[self.cur() as usize],
            ) {
                let mut tmp = V::Error::invalid_literal();
                #[cfg(feature = "span")]
                tmp.apply_span(stamp, stamp);
                return Err(tmp);
            }

            if self.cur() == b'0'
                && (S::NULL_PADDED || self.idx() + 1 != self.src.len())
                && matches!(*self.cur_ptr().add(1), b'0'..=b'9')
            {
                let mut tmp = V::Error::leading_zero();
                #[cfg(feature = "span")]
                tmp.apply_span(self.idx(), self.idx());
                return Err(tmp);
            }

            let start = self.idx();
            let (val, is_int) = self.parse_u64();

            'int: {
                if is_int {
                    self.dec();
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
                    return Ok(tmp);
                }
            }

            if start == self.idx() {
                let mut tmp = V::Error::leading_decimal();
                #[cfg(feature = "span")]
                tmp.apply_span(stamp, stamp);
                return Err(tmp);
            }

            if let Some(val) = self.parse_f64(val, neg, start) {
                self.dec();
                return if val.is_finite() {
                    let mut tmp = V::float(val);

                    #[cfg(feature = "span")]
                    tmp.apply_span(stamp, self.idx());

                    Ok(tmp)
                } else {
                    let mut tmp = V::Error::number_overflow();

                    #[cfg(feature = "span")]
                    tmp.apply_span(stamp, self.idx());

                    Err(tmp)
                };
            }

            let mut tmp = select_unpredictable(
                *self.cur_ptr().sub(1) == b'.',
                V::Error::trailing_decimal(),
                V::Error::invalid_literal(),
            );

            #[cfg(feature = "span")]
            tmp.apply_span(self.idx() - 1, self.idx() - 1);
            return Err(tmp);
        }

        self.inc(3);
        let mut tmp = 'tmp: {
            let mut err = 'err: {
                break 'tmp if S::NULL_PADDED || self.idx() < self.src.len() {
                    let ptr = self.cur_ptr().sub(3);

                    match ptr.cast::<u32>().read_unaligned() {
                        0x6c6c756e => V::null(),
                        0x65757274 => V::bool(true),
                        0x736c6166
                            if (S::NULL_PADDED || self.idx() + 1 != self.src.len())
                                && *ptr.add(4) == b'e' =>
                        {
                            self.inc(1);
                            V::bool(false)
                        }
                        _ => break 'err V::Error::invalid_literal(),
                    }
                } else {
                    break 'err V::Error::invalid_literal();
                };
            };

            #[cfg(feature = "span")]
            err.apply_span(stamp, stamp);
            return Err(err);
        };

        #[cfg(feature = "span")]
        tmp.apply_span(stamp, self.idx());
        Ok(tmp)
    }

    #[inline(always)]
    pub(crate) unsafe fn parse_u64(&mut self) -> (u64, bool) {
        let mut val = 0;
        let mut overflow = false;

        while S::NULL_PADDED || self.idx() + 8 < self.src.len() {
            let Some(chunk) = simd_u64(self.cur_ptr()) else {
                break;
            };

            overflow = val > u64::MAX / 100_000_000;
            let mul = val.wrapping_mul(100_000_000);
            overflow = overflow || mul > u64::MAX - chunk;

            val = mul.wrapping_add(chunk);
            self.inc(8);

            if overflow {
                break;
            }
        }

        if !overflow {
            loop {
                if !S::NULL_PADDED && self.idx() == self.src.len() {
                    return (val, true);
                }

                let cur = self.cur() as usize;
                let tmp = INT_LUT[cur];

                if tmp == 16 {
                    return (val, !NUM_LUT[cur]);
                }

                overflow = val > u64::MAX / 10;
                let mul = val.wrapping_mul(10);
                overflow = overflow || mul > u64::MAX - tmp;

                val = mul.wrapping_add(tmp);
                self.inc(1);

                if overflow {
                    break;
                }
            }
        }

        // ignore overflow as it will be handled in float parsing
        loop {
            if !S::NULL_PADDED && self.idx() == self.src.len() {
                self.dec();
                break;
            }

            let tmp = INT_LUT[self.cur() as usize];
            if tmp == 16 {
                break;
            }

            val = val.wrapping_mul(10).wrapping_add(tmp);
            self.inc(1);
        }

        (val, false)
    }
}

impl<'a, S: Source> Parser<'a, S> {
    /// Createa a parser with the given source and default configuration.
    ///
    /// This is equivalent to calling `Parser::new_with(src, CTConfig)`.
    ///
    /// # Example
    /// ```
    /// use flexon::Parser;
    ///
    /// let parser = Parser::new(r#"{"key": "value"}"#);
    /// ```
    #[inline]
    pub fn new(src: S) -> Self {
        Self::new_with(src, CTConfig)
    }
}

impl<'a> Parser<'a, &'a str> {
    /// Creates a parser from `&str`.
    #[inline]
    pub fn from_str(s: &'a str) -> Self {
        Self::new(s)
    }

    /// Creates a parser from `&[u8]`, validating UTF-8 encoding.
    ///
    /// If the input is not valid UTF-8, the type of error returned is unspecified.
    #[inline]
    pub fn from_slice(s: &'a [u8]) -> Self {
        Self::new(simdutf8::basic::from_utf8(s).unwrap_or_default())
    }

    /// Creates a parser from `&[u8]`, without validating UTF-8 encoding.
    #[inline]
    pub unsafe fn from_slice_unchecked(s: &'a [u8]) -> Self {
        Self::new(from_utf8_unchecked(s))
    }
}

impl<'a> Parser<'a, &'a mut str> {
    /// Creates a parser from `&mut str`, may perform In-situ parsing.
    #[inline]
    pub fn from_mut_str(s: &'a mut str) -> Self {
        Self::new(s)
    }

    /// Creates a parser from `&mut [u8]` with UTF-8 validation, may perform In-situ parsing.
    #[inline]
    pub fn from_mut_slice(s: &'a mut [u8]) -> Self {
        Self::new(simdutf8::basic::from_utf8_mut(s).unwrap_or_default())
    }

    /// Creates a parser from `&mut [u8]` without UTF-8 validation, may perform In-situ parsing.
    #[inline]
    pub unsafe fn from_mut_slice_unchecked(s: &'a mut [u8]) -> Self {
        Self::new(from_utf8_unchecked_mut(s))
    }
}

impl<R: Read> Parser<'_, Reader<R, false>> {
    /// Creates a parser from a type implementing [Read], with UTF-8 validation.
    ///
    /// The reader will try to keep a minimal buffer while parsing due to the way
    /// the parser is. For optimal performance, it is recommended to wrap the input
    /// in a buffered reader such as [BufReader](std::io::BufReader).
    #[inline]
    pub fn from_reader(r: R) -> Self {
        Self::new(Reader::new(r))
    }
}

impl<R: Read> Parser<'_, Reader<R, true>> {
    /// Creates a parser from a type implementing [Read], without UTF-8 validation.
    ///
    /// The reader will try to keep a minimal buffer while parsing due to the way
    /// the parser is. For optimal performance, it is recommended to wrap the input
    /// in a buffered reader such as [BufReader](std::io::BufReader).
    #[inline]
    pub unsafe fn from_reader_unchecked(r: R) -> Self {
        Self::new(Reader::new_unchecked(r))
    }
}
