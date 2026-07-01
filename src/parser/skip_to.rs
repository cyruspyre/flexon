use crate::{
    Parser,
    config::Config,
    misc::ESC_LUT,
    pointer::JsonPointer,
    source::{NonVolatile, Source},
    utf8,
    value::builder::ErrorBuilder,
};
use core::hint::unreachable_unchecked;

// ehh... used in the string matching functions below to just keep
// reading and make their subsequent matches invalid once invalidated.
//
// INVALID[0]: start points here on mismatch, so it stays mismatched forever.
//
// INVALID[1]: used as both start and end when target is empty, so an empty JSON
//             key matches; distinct address from [0] as such a mismatch
//             doesn't accidentally satisfy `start == end` at return.
static INVALID: [u8; 2] = [0xFF; 2];

impl<'a, S: Source, C: Config> Parser<'a, S, C> {
    /// Skips to the given path.
    ///
    /// This will return early as soon as it reaches the specified path.
    /// If the JSON is invalid or path does not exist, returns error.
    ///
    /// # Example
    /// ```
    /// use flexon::Parser;
    /// use serde::Deserialize;
    ///
    /// let src = r#"{"one": 1, "two": 2}"#;
    /// let mut parser = Parser::new(src);
    ///
    /// parser.skip_to(["two"])?;
    /// println!("two is {}", u32::deserialize(&mut parser)?);
    ///
    /// # Ok::<(), flexon::serde::de::Error>(())
    /// ```
    pub fn skip_to<E, P>(&mut self, p: P) -> Result<(), E>
    where
        E: ErrorBuilder,
        P: IntoIterator,
        P::Item: JsonPointer,
    {
        self._skip_to(p)?;
        self.dec_if_not_empty();
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn _skip_to<E, P>(&mut self, p: P) -> Result<u8, E>
    where
        E: ErrorBuilder,
        P: IntoIterator,
        P::Item: JsonPointer,
    {
        let mut char = self.skip_whitespace();

        'main: for pointer in p {
            #[allow(unused_mut)]
            let mut err = if let Some(key) = pointer.as_key()
                && char == b'{'
            {
                #[cfg(feature = "span")]
                let start = self.idx();
                char = self.skip_whitespace();

                if char == b'}' {
                    let mut tmp = E::expected_value();

                    #[cfg(feature = "span")]
                    tmp.apply_span(start, self.idx());
                    return Err(tmp);
                }

                loop {
                    if char != b'"' {
                        break E::unexpected_token();
                    }

                    let matches = unsafe { self.string_match(key)? };
                    if self.skip_whitespace() != b':' {
                        break E::expected_colon();
                    }

                    char = self.skip_whitespace();
                    if matches {
                        continue 'main;
                    }

                    match char {
                        b'"' => self.skip_string(),
                        b'{' => self.skip_object(),
                        b'[' => self.skip_array(),
                        _ => unsafe { self.skip_literal() },
                    }?;

                    char = self.skip_whitespace();
                    let comma = char == b',';
                    if comma {
                        char = self.skip_whitespace();
                    }

                    if char == b'}' {
                        if !comma || self.cfg.trailing_comma() {
                            let mut tmp = E::expected_value();

                            #[cfg(feature = "span")]
                            tmp.apply_span(start, self.idx());
                            return Err(tmp);
                        } else {
                            #[cfg(feature = "span")]
                            self.dec();
                            break E::trailing_comma();
                        }
                    }

                    if comma || self.cfg.comma() {
                        continue;
                    }

                    break match char {
                        0 => E::eof(),
                        _ => E::unexpected_token(),
                    };
                }
            } else if let Some(mut idx) = pointer.as_index()
                && char == b'['
            {
                #[cfg(feature = "span")]
                let start = self.idx();
                char = self.skip_whitespace();

                if char == b']' {
                    let mut tmp = E::expected_value();

                    #[cfg(feature = "span")]
                    tmp.apply_span(start, self.idx());
                    return Err(tmp);
                }

                loop {
                    if idx == 0 {
                        continue 'main;
                    }

                    idx -= 1;
                    match char {
                        b'"' => self.skip_string(),
                        b'{' => self.skip_object(),
                        b'[' => self.skip_array(),
                        _ => unsafe { self.skip_literal() },
                    }?;

                    char = self.skip_whitespace();
                    let comma = char == b',';
                    if comma {
                        char = self.skip_whitespace();
                    }

                    if char == b']' {
                        if !comma || self.cfg.trailing_comma() {
                            let mut tmp = E::expected_value();

                            #[cfg(feature = "span")]
                            tmp.apply_span(start, self.idx());
                            return Err(tmp);
                        } else {
                            #[cfg(feature = "span")]
                            self.dec();
                            break E::trailing_comma();
                        }
                    }

                    if comma || self.cfg.comma() {
                        continue;
                    }

                    break match char {
                        0 => E::eof(),
                        _ => E::unexpected_token(),
                    };
                }
            } else {
                match char {
                    0 => E::eof(),
                    _ => E::unexpected_token(),
                }
            };

            #[cfg(feature = "span")]
            err.apply_span(self.idx(), self.idx());
            return Err(err);
        }

        Ok(char)
    }

    unsafe fn string_match<E: ErrorBuilder>(&mut self, target: &str) -> Result<bool, E> {
        let mut utf8 = utf8::Parser::new();
        let (mut start, end) = if target.is_empty() {
            (&raw const INVALID[1], &raw const INVALID[1])
        } else {
            (target.as_ptr(), target.as_ptr().add(target.len()))
        };

        let err = loop {
            self.inc(1);
            if !S::NULL_PADDED && self.idx() >= self.src.len() {
                break E::unclosed_string();
            }

            break match self.cur() {
                b'"' => {
                    if !S::UTF8 && utf8.is_expecting() {
                        break E::unexpected_token();
                    }

                    return Ok(start == end);
                }
                b'\\' => {
                    self.inc(1);
                    if !S::NULL_PADDED && self.idx() == self.src.len() {
                        break E::unclosed_string();
                    }

                    let cur = self.cur();
                    let esc = ESC_LUT[cur as usize];

                    if esc != 0 {
                        start = match start != end && *start == esc {
                            true => start.add(1),
                            _ => &raw const INVALID[0],
                        };
                        continue;
                    }

                    if cur == b'u'
                        && let Some(esc) = self.unicode_escape(&mut [0; 4])
                    {
                        for &v in esc {
                            start = match start != end && *start == v {
                                true => start.add(1),
                                _ => &raw const INVALID[0],
                            }
                        }
                        continue;
                    }

                    E::invalid_escape()
                }
                v @ 0x20.. => {
                    if !S::UTF8 && utf8.advance(v) {
                        break E::unexpected_token();
                    }

                    start = match start != end && *start == v {
                        true => start.add(1),
                        _ => &raw const INVALID[0],
                    };
                    continue;
                }
                _ => E::control_character(),
            };
        };

        Err(err)
    }
}

impl<'a, S: Source<Volatility = NonVolatile>, C: Config> Parser<'a, S, C> {
    /// Skips to the given path without validation.
    ///
    /// Same as [`Parser::skip_to`] but if the JSON is invalid or
    /// the path does not exist, then there is no guarantee of this function.
    pub unsafe fn skip_to_unchecked<P>(&mut self, p: P)
    where
        P: IntoIterator,
        P::Item: JsonPointer,
    {
        self._skip_to_unchecked(p);
        self.dec();
    }

    #[inline(always)]
    pub(crate) unsafe fn _skip_to_unchecked<P>(&mut self, p: P) -> u8
    where
        P: IntoIterator,
        P::Item: JsonPointer,
    {
        let mut char = self.skip_whitespace();

        'main: for pointer in p {
            if let Some(key) = pointer.as_key() {
                loop {
                    self.skip_whitespace(); // skip '"'
                    let matches = self.string_match_unchecked(key);
                    self.skip_whitespace(); // skip ':'
                    char = self.skip_whitespace();

                    if matches {
                        continue 'main;
                    }

                    match char {
                        b'"' => self.skip_string_unchecked(),
                        b'{' | b'[' => self.skip_container_unchecked(),
                        _ => self.skip_literal_unchecked(),
                    }
                    self.skip_whitespace(); // skip ','
                }
            }

            let Some(mut idx) = pointer.as_index() else {
                unreachable_unchecked()
            };

            loop {
                char = self.skip_whitespace();
                if idx == 0 {
                    continue 'main;
                }

                idx -= 1;
                match char {
                    b'"' => self.skip_string_unchecked(),
                    b'{' | b'[' => self.skip_container_unchecked(),
                    _ => self.skip_literal_unchecked(),
                }
                self.skip_whitespace(); // skip ','
            }
        }

        char
    }

    unsafe fn string_match_unchecked(&mut self, target: &str) -> bool {
        let (mut start, end) = if target.is_empty() {
            (&raw const INVALID[1], &raw const INVALID[1])
        } else {
            (target.as_ptr(), target.as_ptr().add(target.len()))
        };

        loop {
            self.inc(1);
            match self.cur() {
                b'"' => return start == end,
                b'\\' => {
                    self.inc(1);
                    let esc = ESC_LUT[self.cur() as usize];

                    if esc != 0 {
                        start = match start != end && *start == esc {
                            true => start.add(1),
                            _ => &raw const INVALID[0],
                        };
                        continue;
                    }

                    let mut tmp = [0; 4];
                    let esc = self.unicode_escape(&mut tmp).unwrap_unchecked();

                    for &v in esc {
                        start = match start != end && *start == v {
                            true => start.add(1),
                            _ => &raw const INVALID[0],
                        }
                    }
                }
                v => match start != end && *start == v {
                    true => start = start.add(1),
                    _ => start = &raw const INVALID[0],
                },
            }
        }
    }
}
