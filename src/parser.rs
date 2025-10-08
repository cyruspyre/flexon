#[cfg(feature = "metadata")]
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
    index: usize,
    comma: bool,
    trailing_comma: bool,
    #[cfg(feature = "metadata")]
    metadata: Metadata<'a>,
    #[cfg(all(feature = "comment", not(feature = "metadata")))]
    cmnts: Vec<&'a str>,
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a str, comma: bool, trailing_comma: bool) -> Self {
        Self {
            src: src.as_bytes(),
            comma,
            index: usize::MAX,
            trailing_comma: comma && !trailing_comma,
            #[cfg(feature = "metadata")]
            metadata: Metadata {
                lines: Vec::new(),
                cmnts: Vec::new(),
            },
            #[cfg(all(feature = "comment", not(feature = "metadata")))]
            cmnts: Vec::new(),
        }
    }

    #[cfg(all(not(feature = "metadata"), not(feature = "comment")))]
    pub fn parse(mut self) -> Result<Span<Value>, Error> {
        self.value()
    }

    #[cfg(all(feature = "comment", not(feature = "metadata")))]
    pub fn parse(mut self) -> Result<(Span<Value>, Vec<&'a str>), Error> {
        self.value().map(|v| (v, self.cmnts))
    }

    #[cfg(feature = "metadata")]
    pub fn parse(mut self) -> Result<(Span<Value>, Metadata<'a>), Error> {
        self.value().map(|v| {
            self.metadata.lines.push(self.index);
            (v, self.metadata)
        })
    }

    fn next(&mut self) -> u8 {
        self.index += 1;

        let index = self.index;
        let tmp = match self.src.get(index) {
            Some(v) => *v,
            None => return 0,
        };

        #[cfg(feature = "metadata")]
        if tmp == b'\n' && self.metadata.lines.last().is_some_and(|v| *v < index) {
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
                if flag == 1 && tmp == b'\n' {
                } else if flag == 2
                    && tmp == b'*'
                    && *self.src.get(self.index + 1).unwrap_or(&0) == b'/'
                {
                    self.index += 1
                } else {
                    count += 1;
                    continue;
                }

                let start = self.index - count;
                let cmnt = unsafe { str::from_utf8_unchecked(&self.src[start..self.index]) };

                #[cfg(feature = "metadata")]
                self.metadata.cmnts.push((start, flag == 2, unsafe {
                    str::from_utf8_unchecked(&self.src[start..self.index])
                }));

                #[cfg(all(feature = "comment", not(feature = "metadata")))]
                self.cmnts.push(cmnt);

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
                self.index -= 1;

                return tmp;
            }
        }
    }

    fn might(&mut self, de: u8) -> bool {
        let tmp = self.skip_whitespace() == de;

        if tmp {
            self.index += 1
        }

        tmp
    }

    fn expect(&mut self, de: u8) -> Result<(), Error> {
        if !self.might(de) {
            return Err(Error::Expected);
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
                v if v == b'-' || v.is_ascii_digit() => self.number(),
                _ => break 'tmp,
            };
        };

        let start = self.index + 1;
        while self.next().is_ascii_alphabetic() {}
        let tmp = &self.src[start..self.index];

        self.index -= 1;

        Ok(Span {
            data: match tmp {
                b"true" | b"false" => Value::Boolean(tmp.len() == 4),
                b"null" => Value::Null,
                _ => {
                    return Err(Error::Unexpected);
                }
            },
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
        self.index += 1;

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
        self.index += 1;

        let start = self.index;
        let mut buf = Vec::new();

        loop {
            let tmp = match self.next() {
                b'"' => break,
                b'\\' => match self.next() {
                    0 => 0,
                    b'"' => b'"',
                    b'r' => b'\r',
                    b'n' => b'\n',
                    b'\\' => b'\\',
                    _ => return Err(Error::InvalidEscapeSequnce),
                },
                v => v,
            };

            if tmp == 0 {
                return Err(Error::Eof);
            }

            buf.push(tmp)
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
        let start = self.index + 1;
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

        if self.src[self.index] == b'.' {
            return Err(Error::TrailingDecimal);
        }

        let data = unsafe { str::from_utf8_unchecked(&self.src[start..self.index]) };
        let data = if dot && let Ok(v) = data.parse() {
            Number::Float(v)
        } else if neg && let Ok(v) = data.parse() {
            Number::Signed(v)
        } else if let Ok(v) = data.parse() {
            Number::Unsigned(v)
        } else {
            unsafe { unreachable_unchecked() }
        };

        self.index -= 1;

        return Ok(Span {
            data: Value::Number(data),
            start,
            end: self.index,
        });
    }
}
