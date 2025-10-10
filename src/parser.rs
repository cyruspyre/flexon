#[cfg(feature = "line-count")]
use crate::metadata::Metadata;

use crate::{
    error::Error,
    misc::Bypass,
    span::Span,
    value::{Number, Value},
};

use std::hint::unreachable_unchecked;

pub struct Parser<'a> {
    src: &'a [u8],
    stamp: usize,
    index: usize,
    comma: bool,
    trailing_comma: bool,
    #[cfg(feature = "line-count")]
    metadata: Metadata<'a>,
    #[cfg(all(feature = "comment", not(feature = "line-count")))]
    cmnts: Vec<Span<(&'a str, bool)>>,
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a str, comma: bool, trailing_comma: bool) -> Self {
        Self {
            comma,
            stamp: 0,
            src: src.as_bytes(),
            index: usize::MAX,
            trailing_comma: comma && !trailing_comma,
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
    fn _parse<T>(mut self, handler: impl Fn(Span<Value>, Self) -> T) -> Result<T, Span<Error>> {
        match self.value() {
            Ok(v) => Ok(handler(v, self)),
            Err(data) => Err(Span {
                data,
                start: match self.stamp {
                    0 => self.index,
                    v => v,
                },
                end: self.index,
            }),
        }
    }

    #[cfg(all(not(feature = "line-count"), not(feature = "comment")))]
    pub fn parse(self) -> Result<Span<Value>, Span<Error>> {
        self._parse(|v, _| v)
    }

    #[cfg(all(feature = "comment", not(feature = "line-count")))]
    pub fn parse(self) -> Result<(Span<Value>, Vec<Span<(&'a str, bool)>>), Span<Error>> {
        self._parse(|a, b| (a, b.cmnts))
    }

    #[cfg(feature = "line-count")]
    pub fn parse(self) -> Result<(Span<Value>, Metadata<'a>), Span<Error>> {
        self._parse(|a, b| (a, b.metadata))
    }

    fn next(&mut self) -> u8 {
        self.index = self.index.wrapping_add(1);

        let index = self.index;
        let tmp = match self.src.get(index) {
            Some(v) => *v,
            None => return 0,
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
                let data = Span {
                    data: (
                        unsafe { str::from_utf8_unchecked(&self.src[start..self.index]) },
                        flag == 2,
                    ),
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

    fn value(&mut self) -> Result<Span<Value>, Error> {
        let de = self.skip_whitespace();

        'tmp: {
            return match de {
                0 => Err(Error::Eof),
                b'"' => self.string(),
                b'{' => self.object(),
                b'[' => self.array(),
                v if v == b'-' || v == b'.' || v.is_ascii_digit() => self.number(),
                _ => break 'tmp,
            };
        };

        let start = self.index.wrapping_add(1);

        while self.next().is_ascii_alphabetic() {}

        let tmp = &self.src[start..self.index];
        let data = match tmp {
            b"true" | b"false" => Value::Boolean(tmp.len() == 4),
            b"null" => Value::Null,
            _ => return Err(Error::UnexpectedToken),
        };

        self.index -= 1;

        Ok(Span {
            data,
            start,
            end: self.index,
        })
    }

    #[inline(always)]
    fn field_like<T: Into<Value>>(
        &mut self,
        de: u8,
        mut data: T,
        mut handler: impl FnMut(&mut T) -> Result<(), Error>,
    ) -> Result<Span<Value>, Error> {
        self.index = self.index.wrapping_add(1);

        let start = self.index;

        while !self.might(de) {
            handler(&mut data)?;

            let sep = self.might(b',');
            let end = self.might(de);

            if sep {
                if self.trailing_comma && end {
                    return Err(Error::TrailingComma);
                }
            } else if self.comma && !end {
                self.expect(b',')?;
            }

            if end {
                break;
            }
        }

        return Ok(Span {
            data: data.into(),
            start,
            end: self.index,
        });
    }

    #[inline(always)]
    fn object(&mut self) -> Result<Span<Value>, Error> {
        self.bypass().field_like(b'}', Vec::new(), |data| {
            let key = self.string()?;
            let key = Span {
                data: match key.data {
                    Value::String(v) => v,
                    _ => unsafe { unreachable_unchecked() },
                },
                start: key.start,
                end: key.end,
            };

            self.expect(b':')?;
            data.push((key, self.value()?));

            Ok(())
        })
    }

    #[inline(always)]
    fn array(&mut self) -> Result<Span<Value>, Error> {
        self.bypass().field_like(b']', Vec::new(), |data| {
            data.push(self.value()?);
            Ok(())
        })
    }

    fn string(&mut self) -> Result<Span<Value>, Error> {
        self.expect(b'"')?;

        let start = self.index;
        let mut buf = Vec::new();
        let mut err = false;

        loop {
            let tmp = match self.next() {
                b'"' => break,
                b'\\' => match self.next() {
                    0 => 0,
                    b'"' => b'"',
                    b'r' => b'\r',
                    b'n' => b'\n',
                    b'\\' => b'\\',
                    v => {
                        err = true;
                        v
                    }
                },
                v => v,
            };

            if tmp == 0 {
                return Err(Error::Eof);
            }

            buf.push(tmp)
        }

        if err {
            self.stamp = start;
            return Err(Error::InvalidEscapeSequnce);
        }

        Ok(Span {
            data: Value::String(unsafe { String::from_utf8_unchecked(buf) }),
            start,
            end: self.index,
        })
    }

    #[inline(always)]
    fn number(&mut self) -> Result<Span<Value>, Error> {
        let mut dot = false;
        let start = self.index.wrapping_add(1);
        let neg = self.might(b'-');

        loop {
            let tmp = self.next();

            if !tmp.is_ascii_digit() {
                if dot || tmp != b'.' {
                    break;
                }

                dot = true
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

        let data = unsafe { str::from_utf8_unchecked(&self.src[start..=self.index]) };
        let data = if dot && let Ok(v) = data.parse() {
            Number::Float(v)
        } else if neg && let Ok(v) = data.parse() {
            Number::Signed(v)
        } else if let Ok(v) = data.parse() {
            Number::Unsigned(v)
        } else {
            self.stamp = start;
            return Err(Error::NumberOverflow);
        };

        return Ok(Span {
            data: Value::Number(data),
            start,
            end: self.index,
        });
    }
}
