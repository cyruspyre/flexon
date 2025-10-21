#[cfg(feature = "line-count")]
use crate::metadata::Metadata;

#[cfg(feature = "span")]
use crate::Span;

use crate::{
    Wrap,
    error::Error,
    misc::Bypass,
    source::Source,
    value::{Number, Object, Value},
    wrap,
};

#[cfg(feature = "comment")]
use std::borrow::Cow;

use std::{
    alloc::{Layout, dealloc, realloc},
    hint::unreachable_unchecked,
    marker::PhantomData,
    ptr::{dangling_mut, null_mut},
};

/// JSON parser for... *JSON?*
pub struct Parser<'a, S: Source> {
    src: S,
    stamp: usize,
    index: usize,
    comma: bool,
    trailing_comma: bool,
    #[cfg(feature = "prealloc")]
    prev_sizes: [usize; 2],
    #[cfg(feature = "line-count")]
    metadata: Metadata<'a>,
    #[cfg(all(feature = "comment", not(feature = "line-count")))]
    cmnts: Vec<Wrap<(Cow<'a, str>, bool)>>,
    #[cfg(feature = "comment")]
    allow_cmnt: bool,
    phantom: PhantomData<&'a ()>,
}

impl<'a, S: Source + 'a> Parser<'a, S> {
    /// Creates a new JSON `Parser` with the given options.
    ///
    /// # Arguments
    ///
    /// - `src`: The JSON source to parse.
    /// - `comma`: Whether to require commas while parsing.
    /// - `trailing_comma`: Whether trailing commas are allowed (has no effect when commas are optional).
    pub fn new(src: S, comma: bool, trailing_comma: bool) -> Self {
        Self {
            comma,
            src,
            stamp: usize::MAX,
            index: usize::MAX,
            trailing_comma: comma && !trailing_comma,
            #[cfg(feature = "prealloc")]
            prev_sizes: [0; 2],
            #[cfg(feature = "comment")]
            allow_cmnt: false,
            #[cfg(feature = "line-count")]
            metadata: Metadata {
                lines: Vec::new(),
                cmnts: Vec::new(),
            },
            #[cfg(all(feature = "comment", not(feature = "line-count")))]
            cmnts: Vec::new(),
            phantom: PhantomData,
        }
    }

    #[inline(always)]
    fn _parse<T>(mut self, handler: impl Fn(Wrap<Value>, Self) -> T) -> Result<T, Wrap<Error>> {
        let data = match self.value() {
            Ok(v) if self.skip_whitespace() == 0 => return Ok(handler(v, self)),
            Err(v) => v,
            _ => {
                self.index += 1;
                Error::UnexpectedToken
            }
        };

        Err(
            #[cfg(feature = "span")]
            Span {
                data,
                start: match self.stamp {
                    usize::MAX => self.index,
                    v => v,
                },
                end: self.index,
            },
            #[cfg(not(feature = "span"))]
            data,
        )
    }

    /// Parses the JSON source into a single value.
    pub fn parse(self) -> Result<wrap!(Value), wrap!(Error)> {
        self._parse(|v, _| v)
    }

    /// Parses the JSON source and returns the value along with any comments.
    #[cfg(feature = "comment")]
    #[cfg_attr(docsrs, doc(cfg(feature = "comment")))]
    pub fn parse_with_comments(
        mut self,
    ) -> Result<(wrap!(Value), Vec<wrap!((Cow<'a, str>, bool))>), wrap!(Error)> {
        self.allow_cmnt = true;
        self._parse(|a, b| {
            (
                a,
                #[cfg(feature = "line-count")]
                b.metadata.cmnts,
                #[cfg(all(feature = "comment", not(feature = "line-count")))]
                b.cmnts,
            )
        })
    }

    /// Parses the JSON source and returns the value along with its metadata.
    #[cfg(feature = "line-count")]
    #[cfg_attr(docsrs, doc(cfg(feature = "line-count")))]
    pub fn parse_with_metadata(mut self) -> Result<(wrap!(Value), Metadata<'a>), wrap!(Error)> {
        self.allow_cmnt = true;
        self._parse(|a, b| (a, b.metadata))
    }

    fn next(&mut self) -> u8 {
        self.index = self.index.wrapping_add(1);

        let index = self.index;
        let tmp = self.src.get_checked(index);

        #[cfg(feature = "line-count")]
        if tmp == b'\n'
            && match self.metadata.lines.last() {
                Some(v) => *v < index,
                _ => true,
            }
        {
            self.metadata.lines.push(index)
        }

        return tmp;
    }

    fn skip_whitespace(&mut self) -> u8 {
        const WHITESPACE: u8 = 1;
        const OTHER: u8 = 2;
        const EOF: u8 = 255;

        #[cfg(feature = "comment")]
        const COMMENT: u8 = 3;

        const LOOKUP: [u8; 256] = const {
            let mut tmp = [OTHER; 256];

            tmp[0] = EOF;
            tmp[' ' as usize] = WHITESPACE;
            tmp['\t' as usize] = WHITESPACE;
            tmp['\n' as usize] = WHITESPACE;
            tmp['\r' as usize] = WHITESPACE;
            tmp['\x0C' as usize] = WHITESPACE;

            #[cfg(feature = "comment")]
            (tmp['/' as usize] = COMMENT);

            tmp
        };

        loop {
            let tmp = self.next();

            match LOOKUP[tmp as usize] {
                OTHER => {
                    self.index = self.index.wrapping_sub(1);

                    return tmp;
                }
                WHITESPACE => continue,
                #[cfg(feature = "comment")]
                COMMENT => {
                    if self.allow_cmnt {
                        self.comment();
                    }

                    continue;
                }
                _ => return 0,
            }
        }
    }

    fn might(&mut self, de: u8) -> bool {
        let tmp = self.skip_whitespace() == de;

        if tmp {
            self.index = self.index.wrapping_add(1)
        }

        tmp
    }

    fn expect(&mut self, de: u8) -> Result<(), Error> {
        if !self.might(de) {
            self.index += 1;
            return Err(Error::Expected(de as char));
        }

        Ok(())
    }

    #[cfg(feature = "comment")]
    #[inline(always)]
    fn comment(&mut self) {
        let v = match self.src.get_checked(self.index + 1) {
            0 => return,
            v => v,
        };
        let start = self.index;
        let mut multi = true;

        match v {
            b'/' => {
                self.index = match self.src.unbounded_search([b'\n'], self.index + 1) {
                    0 => return self.index = self.src.len() - 1,
                    v => v,
                };

                multi = false;
                #[cfg(feature = "line-count")]
                self.metadata.lines.push(self.index + 1);
            }
            b'*' => loop {
                #[cfg(feature = "line-count")]
                let tmp = self.src.unbounded_search([b'\n', b'/'], self.index + 1);
                #[cfg(not(feature = "line-count"))]
                let tmp = self.src.unbounded_search([b'/'], self.index + 1);

                self.index = match tmp {
                    0 => return self.index = self.src.len() - 1,
                    v => v,
                };

                #[cfg(feature = "line-count")]
                if self.src.get(self.index) == b'\n' {
                    self.metadata.lines.push(self.index);
                    continue;
                }

                if self.src.get(tmp - 1) == b'*' {
                    break;
                }

                self.index += 1;
            },
            _ => return,
        }

        let data = unsafe {
            str::from_utf8_unchecked(&self.src.slice(start + 2..self.index - multi as usize))
        };
        let data = (
            if S::BORROWED {
                Cow::Borrowed(unsafe { &*(data as *const _) })
            } else {
                Cow::Owned(data.to_string())
            },
            multi,
        );
        #[cfg(feature = "span")]
        let data = Span {
            data,
            start: start,
            end: self.index - !multi as usize,
        };

        #[cfg(feature = "line-count")]
        self.metadata.cmnts.push(data);

        #[cfg(all(feature = "comment", not(feature = "line-count")))]
        self.cmnts.push(data);
    }

    fn value(&mut self) -> Result<Wrap<Value>, Error> {
        'tmp: {
            return match self.skip_whitespace() {
                0 => Err(Error::Eof),
                b'"' => self.string(),
                b'{' => self.object(),
                b'[' => self.array(),
                v if v == b'-' || v.is_ascii_digit() || v == b'.' => self.number(),
                _ => break 'tmp,
            };
        };

        let start = self.index.wrapping_add(1);

        self.index = self.index.wrapping_add(4);
        self.src.hint(self.index + 1);

        if self.index >= self.src.len() {
            self.index = start;
            return Err(Error::UnexpectedToken);
        }

        let data = match self.src.slice(start..=self.index) {
            b"true" => Value::Boolean(true),
            b"null" => Value::Null,
            _ => {
                self.index += 1;

                if self.index >= self.src.len() || self.src.slice(start..=self.index) != b"false" {
                    self.index = start;
                    return Err(Error::UnexpectedToken);
                }

                Value::Boolean(false)
            }
        };

        Ok(
            #[cfg(feature = "span")]
            Span {
                data,
                start,
                end: self.index,
            },
            #[cfg(not(feature = "span"))]
            data,
        )
    }

    #[inline(always)]
    fn field_like<T>(
        &mut self,
        de: u8,
        #[cfg(feature = "prealloc")] typ: usize,
        mut handler: impl FnMut() -> Result<T, Error>,
        mut wrap: impl FnMut(Vec<T>) -> Value,
    ) -> Result<Wrap<Value>, Error> {
        self.index = self.index.wrapping_add(1);

        #[cfg(feature = "span")]
        let start = self.index;
        let mut flag = false;

        #[cfg(feature = "prealloc")]
        let mut data = Vec::with_capacity(self.prev_sizes[typ]);
        #[cfg(not(feature = "prealloc"))]
        let mut data = Vec::new();

        loop {
            let tmp = self.skip_whitespace();

            if tmp == b',' {
                self.index += 1;

                if self.might(de) {
                    if self.trailing_comma || !flag {
                        return Err(Error::TrailingComma);
                    }

                    break;
                }
            } else if tmp == de {
                break;
            } else if flag && self.comma {
                return Err(Error::Expected(','));
            }

            data.push(handler()?);

            flag = true;
        }

        self.index += 1;
        #[cfg(feature = "prealloc")]
        (self.prev_sizes[typ] = data.len());

        Ok(
            #[cfg(feature = "span")]
            Span {
                data: wrap(data),
                start,
                end: self.index,
            },
            #[cfg(not(feature = "span"))]
            wrap(data),
        )
    }

    #[inline(always)]
    fn object(&mut self) -> Result<Wrap<Value>, Error> {
        self.bypass().field_like(
            b'}',
            #[cfg(feature = "prealloc")]
            0,
            || {
                let key = self.string()?;

                #[cfg(feature = "span")]
                let key = Span {
                    data: match key.data {
                        Value::String(v) => v,
                        _ => unsafe { unreachable_unchecked() },
                    },
                    start: key.start,
                    end: key.end,
                };

                #[cfg(not(feature = "span"))]
                let Value::String(key) = key else {
                    unsafe { unreachable_unchecked() }
                };

                self.expect(b':')?;

                Ok((key, self.value()?))
            },
            |mut v| {
                v.sort_unstable_by(|a, b| {
                    #[cfg(feature = "span")]
                    return a.0.data.cmp(&b.0.data);

                    #[cfg(not(feature = "span"))]
                    a.0.cmp(&b.0)
                });
                Value::Object(Object(v))
            },
        )
    }

    #[inline(always)]
    fn array(&mut self) -> Result<Wrap<Value>, Error> {
        self.bypass().field_like(
            b']',
            #[cfg(feature = "prealloc")]
            1,
            || Ok(self.value()?),
            |v| Value::Array(v),
        )
    }

    // He who controls string, controls speed
    fn string(&mut self) -> Result<Wrap<Value>, Error> {
        self.expect(b'"')?;

        let stamp = self.index;
        let mut start = stamp + 1;
        let mut buf = null_mut::<u8>();
        let mut cap = 0;
        let mut len = 0;
        let mut err = false;

        loop {
            self.index += 1;

            #[cfg(feature = "line-count")]
            let tmp = self.src.unbounded_search([b'"', b'\\', b'\n'], self.index);
            #[cfg(not(feature = "line-count"))]
            let tmp = self.src.unbounded_search([b'"', b'\\'], self.index);

            if tmp == 0 {
                return Err(Error::Eof);
            }

            self.index = tmp;

            let tmp = self.src.get(self.index);

            if tmp == b'"' {
                break;
            }

            if tmp == b'\\' {
                let src = self.src.slice(start..self.index);
                let new_len = len + src.len() + 4;

                if cap < new_len {
                    let tmp = new_len * 3 / 2;

                    buf = unsafe {
                        realloc(
                            buf,
                            Layout::array::<u8>(cap).unwrap(),
                            Layout::array::<u8>(tmp).unwrap().size(),
                        )
                    };
                    cap = tmp;
                }

                unsafe {
                    buf.add(len)
                        .copy_from_nonoverlapping(src.as_ptr(), src.len())
                }

                len += src.len();
                start = self.index + 2;

                let buf = unsafe { buf.add(len) };
                let tmp = match self.next() {
                    b'"' => b'"',
                    b'\\' => b'\\',
                    b'n' => b'\n',
                    b't' => b'\t',
                    b'r' => b'\r',
                    b'b' => b'\x08',
                    b'f' => b'\x0C',
                    b'/' => b'/',
                    b'u' => {
                        let tmp = self.unicode_escape(buf, start);

                        start = self.index + 1;
                        err |= tmp == 0;
                        len += tmp;

                        continue;
                    }
                    _ => {
                        err = true;
                        continue;
                    }
                };

                unsafe { buf.write(tmp) }
                len += 1;

                continue;
            }
        }

        if err {
            self.stamp = stamp;
            self.src.stamp(stamp);
            unsafe { dealloc(buf, Layout::array::<u8>(cap).unwrap()) };
            return Err(Error::InvalidEscapeSequnce);
        }

        let src = self.src.slice(start..self.index);
        let new_len = len + src.len();

        if cap < new_len {
            buf = unsafe {
                realloc(
                    buf,
                    Layout::array::<u8>(cap).unwrap(),
                    Layout::array::<u8>(new_len).unwrap().size(),
                )
            };
            cap = new_len;
        }

        unsafe {
            buf.add(len)
                .copy_from_nonoverlapping(src.as_ptr(), src.len())
        }

        let data = Value::String(unsafe {
            String::from_raw_parts(
                if new_len == 0 { dangling_mut() } else { buf },
                new_len,
                cap,
            )
        });

        Ok(
            #[cfg(feature = "span")]
            Span {
                data,
                start: stamp,
                end: self.index,
            },
            #[cfg(not(feature = "span"))]
            data,
        )
    }

    #[inline(always)]
    fn unicode_escape(&mut self, buf: *mut u8, mut start: usize) -> usize {
        self.index += 4;
        self.src.hint(self.index);

        if self.index >= self.src.len() {
            self.index -= 4;
            return 0;
        }

        let mut codepoint = match u16::from_str_radix(
            unsafe { str::from_utf8_unchecked(self.src.slice(start..=self.index)) },
            16,
        ) {
            Ok(v) => v as u32,
            _ => return 0,
        };

        if (0xD800..=0xDBFF).contains(&codepoint) {
            start = self.index + 3;

            self.index += 6;
            self.src.hint(self.index);

            if self.index >= self.src.len() || self.src.slice(start - 2..start) != br"\u" {
                self.index -= 6;
                return 0;
            }

            let low = match u16::from_str_radix(
                unsafe { str::from_utf8_unchecked(self.src.slice(start..=self.index)) },
                16,
            ) {
                Ok(v) => v as u32,
                _ => return 0,
            };

            if !(0xDC00..=0xDFFF).contains(&low) {
                return 0;
            }

            codepoint = 0x10000 + (((codepoint - 0xD800) << 10) | (low as u32 - 0xDC00))
        }

        let mut utf8 = [0u8; 4];
        let utf8 = match char::from_u32(codepoint) {
            Some(v) => v.encode_utf8(&mut utf8).as_bytes(),
            _ => return 0,
        };

        for (i, v) in utf8.iter().enumerate() {
            unsafe { buf.add(i).write(*v) }
        }

        utf8.len()
    }

    #[inline(always)]
    fn number(&mut self) -> Result<Wrap<Value>, Error> {
        let mut int = true;

        // flags indicating if either of them were encountered
        let mut exp = false;
        let mut exp_sign = false;

        let start = self.index.wrapping_add(1);
        let pos = !self.might(b'-');
        let tmp = self.index.wrapping_add(1);

        const DIGIT: u8 = 1;
        const DOT: u8 = 2;
        const EXP: u8 = 3;
        const EXP_SIGN: u8 = 4;
        const LOOKUP: [u8; 256] = const {
            let mut tmp = [0; 256];
            let mut idx = b'0';

            while idx <= b'9' {
                tmp[idx as usize] = DIGIT;
                idx += 1;
            }

            tmp[b'.' as usize] = DOT;
            tmp[b'e' as usize] = EXP;
            tmp[b'E' as usize] = EXP;
            tmp[b'+' as usize] = EXP_SIGN;
            tmp[b'-' as usize] = EXP_SIGN;

            tmp
        };

        loop {
            match LOOKUP[self.next() as usize] {
                DIGIT => continue,
                DOT => {
                    if !int {
                        break;
                    }

                    int = false;
                }
                EXP => {
                    if exp {
                        break;
                    }

                    exp = true;
                    int = false;
                }
                EXP_SIGN => {
                    if exp_sign {
                        break;
                    }

                    exp_sign = true;
                }
                _ => break,
            }
        }

        self.index -= 1;

        'tmp: {
            self.stamp = start;
            self.src.stamp(start);

            let tmp = match self.src.get(start) {
                b'.' => Error::LeadingDecimal,
                b'-' if self.index == start => Error::MissingDigitAfterNegative,
                _ => match self.src.get(self.index) {
                    b'.' => Error::TrailingDecimal,
                    v if !v.is_ascii_digit() => Error::ExpectedExponentValue,
                    _ => break 'tmp,
                },
            };

            return Err(tmp);
        }

        let data = unsafe { str::from_utf8_unchecked(self.src.slice(tmp..=self.index)) };
        let data = if int
            && let Ok(v) = data.parse::<u64>()
            && (pos || v <= i64::MIN as _)
        {
            match pos {
                true => Number::Unsigned(v),
                _ => Number::Signed(v.wrapping_neg() as _),
            }
        } else if let Ok(v) = fast_float2::parse(self.src.slice(start..=self.index)) {
            Number::Float(v)
        } else {
            return Err(Error::NumberOverflow);
        };
        let data = Value::Number(data);

        Ok(
            #[cfg(feature = "span")]
            Span {
                data,
                start,
                end: self.index,
            },
            #[cfg(not(feature = "span"))]
            data,
        )
    }
}
