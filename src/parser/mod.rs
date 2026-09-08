mod skip;
mod skip_to;
mod unchecked;

use crate::{
    JsonPointer,
    config::{CTConfig, Config, Depth},
    misc::*,
    simd::simd_u64,
    source::*,
    value::builder::*,
};
use core::{
    hint::cold_path,
    marker::PhantomData,
    slice::from_raw_parts,
    str::{from_utf8_unchecked, from_utf8_unchecked_mut},
};
use simdutf8::compat::from_utf8;

#[cfg(feature = "std")]
use std::io::Read;

#[cfg(feature = "comment")]
use {crate::Comment, alloc::vec::Vec};

// todo: trim source when skipping values

/// JSON parser structure.
pub struct Parser<'a, S: Source + 'a, C: Config = CTConfig> {
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

struct DepthGuard<'a, 'de, S: Source, C: Config>(&'a mut Parser<'de, S, C>);

impl<'de, S: Source, C: Config> core::ops::Deref for DepthGuard<'_, 'de, S, C> {
    type Target = Parser<'de, S, C>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'de, S: Source, C: Config> core::ops::DerefMut for DepthGuard<'_, 'de, S, C> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<S: Source, C: Config> Drop for DepthGuard<'_, '_, S, C> {
    #[inline(always)]
    fn drop(&mut self) {
        self.0.cfg.depth().decrease()
    }
}

impl<'a, S: Source, C: Config> Parser<'a, S, C> {
    pub(crate) const PRE_VALIDATED_UTF8: bool =
        S::UTF8 | !S::Volatility::IS_VOLATILE & C::PRE_VALIDATE_UTF8;

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

        let offset = if !S::UTF8 & !S::Volatility::IS_VOLATILE & C::PRE_VALIDATE_UTF8
            && let Err(e) = unsafe { from_utf8(from_raw_parts(src.ptr(0), src.len())) }
        {
            e.valid_up_to()
        } else {
            0
        };

        Self {
            #[cfg(feature = "prealloc")]
            prealloc: 0,
            #[cfg(feature = "comment")]
            comments: Vec::new(),
            cur: match S::NULL_PADDED {
                true => Cur {
                    ptr: match S::INSITU {
                        true => src.ptr_mut(offset),
                        _ => src.ptr(offset).cast_mut(), // not actually mutating
                    },
                },
                _ => Cur { idx: offset },
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

        if !V::LAZY {
            return self.parse_value();
        }

        let char = self.skip_whitespace();
        let start = self.idx();

        match char {
            b'"' => self.skip_string(),
            b'{' => self.depth_guard()?.skip_object(),
            b'[' => self.depth_guard()?.skip_array(),
            0 => return Err(V::Error::expected_value()),
            _ => unsafe { self.skip_literal() },
        }?;

        unsafe {
            Ok(V::raw(from_raw_parts(
                self.src.ptr(start),
                self.src.len() - start,
            )))
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

        if !V::LAZY {
            return self.parse_value_unchecked();
        }

        self.skip_whitespace();
        V::raw(from_raw_parts(self.cur_ptr(), self.src.len() - self.idx()))
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
        P: IntoIterator<Item: JsonPointer>,
    {
        const {
            assert!(
                !(V::LAZY & S::Volatility::IS_VOLATILE),
                "source must be non volatile if the value builder is lazy"
            )
        }

        unsafe {
            let char = self._skip_to(p)?;

            if V::LAZY {
                let start = self.idx();

                match char {
                    b'"' => self.skip_string(),
                    b'{' => self.depth_guard()?.skip_object(),
                    b'[' => self.depth_guard()?.skip_array(),
                    0 => return Err(V::Error::expected_value()),
                    _ => self.skip_literal(),
                }?;

                Ok(V::raw(from_raw_parts(
                    self.src.ptr(start),
                    self.src.len() - start,
                )))
            } else {
                match char {
                    b'"' => self.parse_string::<_, V::String, _>(),
                    b'{' => self.depth_guard()?.parse_object(),
                    b'[' => self.depth_guard()?.parse_array(),
                    0 => {
                        #[allow(unused_mut)]
                        let mut tmp = V::Error::expected_value();
                        #[cfg(feature = "span")]
                        tmp.apply_span(self.idx(), self.idx());
                        Err(tmp)
                    }
                    _ => self.parse_literal(),
                }
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
    /// # Ok::<(), flexon::serde::de::Error>(())
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
                _ => self.cur.idx += n,
            }
        }
    }

    #[inline(always)]
    pub(crate) fn dec(&mut self, n: usize) {
        unsafe {
            match S::NULL_PADDED {
                true => self.cur.ptr = self.cur.ptr.sub(n),
                _ => self.cur.idx -= n,
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
    #[cfg(feature = "serde")]
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

    #[inline(always)]
    fn depth_guard<'b, E: ErrorBuilder>(&'b mut self) -> Result<DepthGuard<'b, 'a, S, C>, E> {
        if self.cfg.depth().is_limit_reached() {
            return Err(E::depth_limit_exceeded());
        }

        self.cfg.depth().increase();
        Ok(DepthGuard(self))
    }

    pub(crate) fn skip_whitespace(&mut self) -> u8 {
        let mut simd = false;

        loop {
            if !S::NULL_PADDED && self.idx() >= self.src.len() {
                return 0;
            }

            let tmp = self.cur();
            if !matches!(tmp, b' ' | b'\t' | b'\n' | b'\r') {
                #[cfg(feature = "comment")]
                if tmp == b'/' && self.cfg.comments() && self.parse_comment() {
                    continue;
                }

                return tmp;
            }

            self.inc(1);
            if simd && self.simd_wh() {
                #[cfg(feature = "comment")]
                if self.cur() == b'/' && self.cfg.comments() && self.parse_comment() {
                    continue;
                }

                return self.cur();
            }

            simd = true;
        }
    }

    // This function expects to be called at '/', reading `n + 1` on `true`,
    // `0` on `false` where `n` is the length of the comment.
    #[inline(never)]
    #[cfg(feature = "comment")]
    pub(crate) fn parse_comment(&mut self) -> bool {
        self.inc(1);
        if !S::NULL_PADDED && self.idx() == self.src.len() {
            self.dec(1);
            return false;
        }

        let mut multi = false;
        let offset = self.idx() + 1;

        match self.cur() {
            b'/' => loop {
                self.inc(1);
                if !S::NULL_PADDED && self.idx() == self.src.len() {
                    break;
                }

                match self.cur() {
                    b'\n' | b'\r' => break,
                    0 if S::NULL_PADDED => break,
                    _ => {}
                }
            },
            b'*' => loop {
                self.inc(1);
                if !S::NULL_PADDED && self.idx() == self.src.len() {
                    return true;
                }

                match self.cur() {
                    b'*' if (S::NULL_PADDED || self.idx() + 1 != self.src.len())
                        && unsafe { *self.cur_ptr().add(1) == b'/' } =>
                    {
                        multi = true;
                        self.inc(2);
                        break;
                    }
                    0 if S::NULL_PADDED => return true,
                    _ => {}
                }
            },
            _ => {
                self.dec(1);
                return false;
            }
        }

        let idx = self.idx();
        let len = idx - offset - multi as usize * 2;
        let src = self.src.ptr(offset);

        if Self::PRE_VALIDATED_UTF8 || unsafe { from_utf8(from_raw_parts(src, len)).is_ok() } {
            self.comments.push(Comment::new(
                src,
                len,
                multi,
                S::Volatility::IS_VOLATILE && len != 0,
                #[cfg(feature = "span")]
                [offset - 2, idx - 1],
            ));
        }

        true
    }

    #[inline]
    fn parse_value<V: ValueBuilder<'a, S>>(&mut self) -> Result<V, V::Error> {
        if S::Volatility::IS_VOLATILE {
            let tmp = self.idx();
            self.src.trim(tmp);
        }

        unsafe {
            match self.skip_whitespace() {
                b'"' => self.parse_string::<_, V::String, V::Error>(),
                b'{' => self.depth_guard()?.parse_object(),
                b'[' => self.depth_guard()?.parse_array(),
                0 => {
                    #[allow(unused_mut)]
                    let mut tmp = V::Error::expected_value();
                    #[cfg(feature = "span")]
                    tmp.apply_span(self.idx(), self.idx());
                    Err(tmp)
                }
                _ => self.parse_literal(),
            }
        }
    }

    #[allow(unused_mut)]
    unsafe fn parse_object<V: ValueBuilder<'a, S>>(&mut self) -> Result<V, V::Error> {
        #[cfg(feature = "span")]
        let start = self.idx();
        #[cfg(feature = "prealloc")]
        let mut obj = V::Object::with_capacity(self.prealloc);
        #[cfg(not(feature = "prealloc"))]
        let mut obj = V::Object::new();
        self.inc(1);
        let mut tmp = self.skip_whitespace();

        if tmp == b'}' {
            obj.on_complete();
            let mut tmp = obj.into();

            #[cfg(feature = "span")]
            tmp.apply_span(start, self.idx());
            return Ok(tmp);
        }

        #[cfg(feature = "span")]
        let err_idx;
        let mut err = loop {
            if tmp != b'"' {
                #[cfg(feature = "span")]
                (err_idx = self.idx());
                break V::Error::unexpected_token();
            }

            let key = self.parse_string::<V::String, V::String, V::Error>()?;

            self.inc(1);
            if self.skip_whitespace() != b':' {
                #[cfg(feature = "span")]
                (err_idx = self.idx());
                break V::Error::expected_colon();
            }

            self.inc(1);
            obj.on_value(key, self.parse_value()?);
            self.inc(1);
            tmp = self.skip_whitespace();

            #[cfg(feature = "span")]
            let comma_idx = self.idx();
            let comma = tmp == b',';

            if comma {
                self.inc(1);
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
                (err_idx = comma_idx);
                break V::Error::trailing_comma();
            }

            if comma || self.cfg.comma() {
                continue;
            }

            #[cfg(feature = "span")]
            (err_idx = self.idx());
            break match tmp {
                0 => V::Error::eof(),
                _ => V::Error::unexpected_token(),
            };
        };

        #[cfg(feature = "span")]
        err.apply_span(err_idx, err_idx);
        cold_path();
        Err(err)
    }

    #[allow(unused_mut)]
    unsafe fn parse_array<V: ValueBuilder<'a, S>>(&mut self) -> Result<V, V::Error> {
        #[cfg(feature = "span")]
        let start = self.idx();
        let mut arr = V::Array::new();
        self.inc(1);
        let mut tmp = self.skip_whitespace();

        if tmp == b']' {
            arr.on_complete();
            let mut tmp = arr.into();

            #[cfg(feature = "span")]
            tmp.apply_span(start, self.idx());
            return Ok(tmp);
        }

        #[cfg(feature = "span")]
        let err_idx;
        let mut err = loop {
            arr.on_value(match tmp {
                b'"' => self.parse_string::<_, V::String, _>(),
                b'{' => self.depth_guard()?.parse_object(),
                b'[' => self.depth_guard()?.parse_array(),
                0 => {
                    let mut err = V::Error::eof();
                    #[cfg(feature = "span")]
                    err.apply_span(self.idx(), self.idx());
                    return Err(err);
                }
                _ => self.parse_literal(),
            }?);

            self.inc(1);
            tmp = self.skip_whitespace();

            #[cfg(feature = "span")]
            let comma_idx = self.idx();
            let comma = tmp == b',';

            if comma {
                self.inc(1);
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
                (err_idx = comma_idx);
                break V::Error::trailing_comma();
            }

            if comma || self.cfg.comma() {
                continue;
            }

            #[cfg(feature = "span")]
            (err_idx = self.idx());
            break match tmp {
                0 => V::Error::eof(),
                _ => V::Error::unexpected_token(),
            };
        };

        #[cfg(feature = "span")]
        err.apply_span(err_idx, err_idx);
        cold_path();
        Err(err)
    }

    unsafe fn parse_string<T, V, E>(&mut self) -> Result<T, E>
    where
        V: StringBuilder<'a, S> + Into<T>,
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
                            && let Some(v) = self.parse_unicode_escape(&mut [0; 4])
                        {
                            offset = self.idx() + 1;
                            buf.on_escape(v);
                            continue;
                        }

                        E::invalid_escape()
                    }
                    0x20.. => continue,
                    0 if S::NULL_PADDED => E::eof(),
                    _ => E::control_character(),
                };
            };
            #[allow(unused_mut)]
            let mut err = self.close_string(err);

            #[cfg(feature = "span")]
            err.apply_span(start, self.idx());

            return Err(err);
        };

        buf.on_final_chunk(from_raw_parts(self.src.ptr(offset), end - offset));

        let raw = from_raw_parts(self.src.ptr(start + 1), end - start - 1);
        if !Self::PRE_VALIDATED_UTF8 {
            #[cfg(feature = "span")]
            if let Err(utf) = from_utf8(raw) {
                let mut err = E::unexpected_token();
                let idx = start + utf.valid_up_to();
                err.apply_span(idx, idx);
                return Err(err);
            }

            #[cfg(not(feature = "span"))]
            if simdutf8::basic::from_utf8(raw).is_err() {
                return Err(E::unexpected_token());
            }
        }

        #[cfg(feature = "span")]
        buf.apply_span(start, end);

        Ok(buf.into())
    }

    #[inline(never)]
    pub(crate) unsafe fn parse_unicode_escape<'esc>(
        &mut self,
        buf: &'esc mut [u8; 4],
    ) -> Option<&'esc [u8]> {
        if !S::NULL_PADDED && self.idx() + 4 >= self.src.len() {
            return None;
        }

        let Some(mut codepoint) = parse_4hex(self.cur_ptr().add(1)) else {
            return None;
        };

        match codepoint {
            0xDC00..=0xDFFF => return None,
            0xD800..=0xDBFF => {
                if !S::NULL_PADDED && self.idx() + 10 >= self.src.len()
                    || from_raw_parts(self.cur_ptr().add(5), 2) != br"\u"
                {
                    return None;
                }

                let Some(low @ 0xDC00..=0xDFFF) = parse_4hex(self.cur_ptr().add(7)) else {
                    return None;
                };

                codepoint = 0x10000 + (codepoint - 0xD800 << 10 | low - 0xDC00);
                self.inc(10);
            }
            _ => self.inc(4),
        }

        // `codepoint` <= 0x10FFFF (char::MAX) excluding `0xD800..=0xDFFF`
        Some(
            char::from_u32_unchecked(codepoint)
                .encode_utf8(buf)
                .as_bytes(),
        )
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
    unsafe fn parse_literal<V: ValueBuilder<'a, S>>(&mut self) -> Result<V, V::Error> {
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

            let mut tmp = match *self.cur_ptr().sub(1) {
                b'.' => V::Error::trailing_decimal(),
                _ => V::Error::invalid_literal(),
            };

            #[cfg(feature = "span")]
            tmp.apply_span(self.idx() - 1, self.idx() - 1);
            return Err(tmp);
        }

        self.inc(3);
        let mut tmp = 'ok: {
            if S::NULL_PADDED || self.idx() < self.src.len() {
                match self.cur_ptr().sub(3).cast::<u32>().read_unaligned() {
                    0x6c6c756e => break 'ok V::null(),
                    0x65757274 => break 'ok V::bool(true),
                    0x736c6166
                        if (S::NULL_PADDED || self.idx() + 1 != self.src.len())
                            && *self.cur_ptr().add(1) == b'e' =>
                    {
                        self.inc(1);
                        break 'ok V::bool(false);
                    }
                    _ => {}
                }
            }

            let mut err = V::Error::invalid_literal();
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

        while S::NULL_PADDED || self.idx() + 8 <= self.src.len() {
            let Some(chunk) = simd_u64(self.cur_ptr()) else {
                break;
            };

            if val > u64::MAX / 100_000_000 {
                return (val, false);
            }

            let mul = val.wrapping_mul(100_000_000);

            if mul > u64::MAX - chunk {
                return (val, false);
            }

            val = mul.wrapping_add(chunk);
            self.inc(8);
        }

        loop {
            if !S::NULL_PADDED && self.idx() == self.src.len() {
                return (val, true);
            }

            let cur = self.cur();
            let num = cur.wrapping_sub(b'0');

            if num > 9 {
                return (val, !NUM_LUT[cur as usize]);
            }

            if val > u64::MAX / 10 {
                return (val, false);
            }

            let mul = val.wrapping_mul(10);

            if mul > u64::MAX - num as u64 {
                return (val, false);
            }

            val = mul.wrapping_add(num as u64);
            self.inc(1);
        }
    }
}

impl<'a, S: Source<Volatility = NonVolatile>, C: Config> Parser<'a, S, C> {
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
        P: IntoIterator<Item: JsonPointer>,
    {
        let char = self._skip_to_unchecked(p);

        if V::LAZY {
            V::raw(from_raw_parts(self.cur_ptr(), self.src.len() - self.idx()))
        } else {
            match char {
                b'"' => self.parse_string_unchecked::<_, V::String>(),
                b'{' => self.parse_object_unchecked(),
                b'[' => self.parse_array_unchecked(),
                _ => self.parse_literal_unchecked(),
            }
        }
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
        Self::new_with(src, CTConfig::new())
    }
}

impl<'a> Parser<'a, &'a str> {
    /// Creates a parser from `&str`.
    #[inline]
    pub fn from_str(s: &'a str) -> Self {
        Self::new(s)
    }

    /// Creates a parser from `&[u8]`, without validating UTF-8 encoding.
    #[inline]
    pub unsafe fn from_slice_unchecked(s: &'a [u8]) -> Self {
        Self::new(from_utf8_unchecked(s))
    }
}

impl<'a> Parser<'a, &'a [u8]> {
    /// Creates a parser from `&[u8]`, validating UTF-8 encoding.
    #[inline]
    pub fn from_slice(s: &'a [u8]) -> Self {
        Self::new(s)
    }
}

impl<'a> Parser<'a, &'a mut str> {
    /// Creates a parser from `&mut str`, may perform In-situ parsing.
    #[inline]
    pub fn from_mut_str(s: &'a mut str) -> Self {
        Self::new(s)
    }

    /// Creates a parser from `&mut [u8]` without UTF-8 validation, may perform In-situ parsing.
    #[inline]
    pub unsafe fn from_mut_slice_unchecked(s: &'a mut [u8]) -> Self {
        Self::new(from_utf8_unchecked_mut(s))
    }
}

impl<'a> Parser<'a, &'a mut [u8]> {
    /// Creates a parser from `&mut [u8]` with UTF-8 validation, may perform In-situ parsing.
    #[inline]
    pub fn from_mut_slice(s: &'a mut [u8]) -> Self {
        Self::new(s)
    }
}

#[cfg(feature = "std")]
impl<R: Read> Parser<'_, Reader<false, R>> {
    /// Creates a parser from a type implementing [Read], with UTF-8 validation.
    ///
    /// Wrapping the input in [`BufReader`](std::io::BufReader) may or may not be beneficial.
    #[inline]
    pub fn from_reader(r: R) -> Self {
        Self::new(Reader::new(r))
    }
}

#[cfg(feature = "std")]
impl<R: Read> Parser<'_, Reader<true, R>> {
    /// Creates a parser from a type implementing [Read], without UTF-8 validation.
    ///
    /// Wrapping the input in [`BufReader`](std::io::BufReader) may or may not be beneficial.
    #[inline]
    pub unsafe fn from_reader_unchecked(r: R) -> Self {
        Self::new(Reader::new_unchecked(r))
    }
}
