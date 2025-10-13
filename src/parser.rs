#[cfg(feature = "line-count")]
use crate::metadata::Metadata;

#[cfg(feature = "span")]
use crate::Span;

use crate::{
    Wrap,
    error::Error,
    misc::Bypass,
    value::{Number, Object, Value},
};

use std::hint::unreachable_unchecked;

pub struct Parser<'a> {
    src: &'a [u8],
    stamp: usize,
    index: usize,
    comma: bool,
    trailing_comma: bool,
    #[cfg(feature = "prealloc")]
    prev_sizes: [usize; 2],
    #[cfg(feature = "line-count")]
    metadata: Metadata<'a>,
    #[cfg(all(feature = "comment", not(feature = "line-count")))]
    cmnts: Vec<Wrap<(&'a str, bool)>>,
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a str, comma: bool, trailing_comma: bool) -> Self {
        Self {
            comma,
            stamp: 0,
            src: src.as_bytes(),
            index: usize::MAX,
            trailing_comma: comma && !trailing_comma,
            #[cfg(feature = "prealloc")]
            prev_sizes: [0; 2],
            #[cfg(feature = "line-count")]
            metadata: Metadata {
                lines: Vec::new(),
                cmnts: Vec::new(),
            },
            #[cfg(all(feature = "comment", not(feature = "line-count")))]
            cmnts: Vec::new(),
        }
    }

    #[inline(always)]
    fn _parse<T>(mut self, handler: impl Fn(Wrap<Value>, Self) -> T) -> Result<T, Wrap<Error>> {
        let data = match self.value() {
            Ok(v) if self.skip_whitespace() == 0 => return Ok(handler(v, self)),
            Err(v) => v,
            _ => {
                self.index += 1;
                Error::ExpectedEof
            }
        };

        Err(
            #[cfg(feature = "span")]
            Span {
                data,
                start: match self.stamp {
                    0 => self.index,
                    v => v,
                },
                end: self.index,
            },
            #[cfg(not(feature = "span"))]
            data,
        )
    }

    #[cfg(all(not(feature = "line-count"), not(feature = "comment")))]
    pub fn parse(self) -> Result<Wrap<Value>, Wrap<Error>> {
        self._parse(|v, _| v)
    }

    #[cfg(all(feature = "comment", not(feature = "line-count")))]
    pub fn parse(self) -> Result<(Wrap<Value>, Vec<Wrap<(&'a str, bool)>>), Wrap<Error>> {
        self._parse(|a, b| (a, b.cmnts))
    }

    #[cfg(feature = "line-count")]
    pub fn parse(self) -> Result<(Wrap<Value>, Metadata<'a>), Wrap<Error>> {
        self._parse(|a, b| (a, b.metadata))
    }

    fn next(&mut self) -> u8 {
        self.index = self.index.wrapping_add(1);

        let index = self.index;
        let tmp = match self.src.get(index) {
            Some(v) => *v,
            _ => return 0,
        };

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
        #[cfg(feature = "comment")]
        let mut count = 0;
        #[cfg(feature = "comment")]
        let mut flag = 0;

        loop {
            let tmp = self.next();

            if tmp == 0 {
                return 0;
            }

            #[cfg(feature = "comment")]
            if flag > 0 {
                let tmp = if flag == 1 && tmp == b'\n' {
                    -1
                } else if flag == 2
                    && tmp == b'*'
                    && *self.src.get(self.index + 1).unwrap_or(&0) == b'/'
                {
                    1
                } else {
                    count += 1;
                    continue;
                };

                let start = self.index - count;
                let data = (
                    unsafe { str::from_utf8_unchecked(&self.src[start..self.index]) },
                    flag == 2,
                );

                #[cfg(feature = "span")]
                let data = Span {
                    data,
                    start: start - 2,
                    end: self.index,
                };

                self.index = self.index.wrapping_add_signed(tmp);

                #[cfg(feature = "line-count")]
                self.metadata.cmnts.push(data);

                #[cfg(all(feature = "comment", not(feature = "line-count")))]
                self.cmnts.push(data);

                flag = 0;
                count = 0;
            } else if tmp == b'/' {
                let tmp = self.src[self.index + 1];

                if tmp == b'/' {
                    flag = 1
                } else if tmp == b'*' {
                    flag = 2
                } else {
                    continue;
                }

                self.index += 1;
                continue;
            }

            if !tmp.is_ascii_whitespace() {
                self.index = self.index.wrapping_sub(1);

                return tmp;
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

        self.index += 4;

        if self.index >= self.src.len() {
            return Err(Error::UnexpectedToken);
        }

        let data = match &self.src[start..=self.index] {
            b"true" => Value::Boolean(true),
            b"null" => Value::Null,
            _ => {
                self.index += 1;

                if self.index >= self.src.len() || &self.src[start..=self.index] != b"false" {
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
        typ: usize,
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
        self.bypass()
            .field_like(b']', 1, || Ok(self.value()?), |v| Value::Array(v))
    }

    fn string(&mut self) -> Result<Wrap<Value>, Error> {
        self.expect(b'"')?;

        let stamp = self.index;
        let mut start = stamp + 1;
        let mut buf = Vec::new();
        let mut err = false;

        loop {
            self.index += 1;

            let tmp = &self.src[self.index..];
            #[cfg(feature = "line-count")]
            let tmp = memchr::memchr3(b'"', b'\\', b'\n', tmp);
            #[cfg(not(feature = "line-count"))]
            let tmp = memchr::memchr2(b'"', b'\\', tmp);

            self.index += match tmp {
                Some(v) => v,
                _ => return Err(Error::Eof),
            };

            let tmp = self.src[self.index];

            if tmp == b'"' {
                break;
            }

            if tmp == b'\\' {
                buf.extend_from_slice(&self.src[start..self.index]);

                start = self.index + 2;

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
                        self.index += 4;

                        if self.index >= self.src.len() {
                            err = true;
                            continue;
                        }

                        let Ok(tmp) = u8::from_str_radix(
                            unsafe { str::from_utf8_unchecked(&self.src[start..=self.index]) },
                            16,
                        ) else {
                            err = true;
                            continue;
                        };

                        start += 4;
                        tmp
                    }
                    _ => {
                        err = true;
                        continue;
                    }
                };

                buf.push(tmp);
                continue;
            }

            // note to myself: don't use else if chains here. the generated asm jumps around
            // quite a few time for... a reason ik but cant explain properly
            // usually rustc is smart enough to optimize away such cases but in the
            // above case rustc wasn't able to do it and the code was
            // roughly `0.1 ms` slower... ig the compiler got waku waku and got distracted
        }

        if err {
            self.stamp = stamp;
            return Err(Error::InvalidEscapeSequnce);
        }

        buf.extend_from_slice(&self.src[start..self.index]);

        let data = Value::String(unsafe { String::from_utf8_unchecked(buf) });

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
    fn number(&mut self) -> Result<Wrap<Value>, Error> {
        let mut int = true;
        let start = self.index.wrapping_add(1);
        let pos = !self.might(b'-');
        let tmp = self.index.wrapping_add(1);

        loop {
            let tmp = self.next();

            if !tmp.is_ascii_digit() {
                if !int || tmp != b'.' {
                    break;
                }

                int = false
            }
        }

        self.index -= 1;

        'tmp: {
            let tmp = match self.src[start] {
                b'.' => Error::LeadingDecimal,
                b'-' if self.index == start => Error::MissingDigitAfterNegative,
                _ if self.src[self.index] == b'.' => Error::TrailingDecimal,
                _ => break 'tmp,
            };

            self.stamp = start;
            return Err(tmp);
        }

        let data = unsafe { str::from_utf8_unchecked(&self.src[tmp..=self.index]) };
        let data = if int
            && let Ok(v) = data.parse::<u64>()
            && (pos || v <= i64::MIN as _)
        {
            match pos {
                true => Number::Unsigned(v),
                _ => Number::Signed(v.wrapping_neg() as _),
            }
        } else if let Ok(v) = fast_float2::parse(&self.src[start..=self.index]) {
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
