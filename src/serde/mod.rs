//! serde specific API.

pub mod error;
mod unchecked;

use core::{
    alloc::Layout, hint::select_unpredictable, ptr::dangling_mut, slice::from_raw_parts,
    str::from_utf8_unchecked,
};
use std::{
    alloc::{alloc, dealloc, realloc},
    io::Read,
};

use serde::{
    Deserialize, Deserializer,
    de::{
        self, DeserializeOwned, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, Unexpected,
        Visitor,
    },
    forward_to_deserialize_any,
};
use simdutf8::basic::from_utf8;

use crate::{
    Parser,
    config::Config,
    misc::{ESC_LUT, NUM_LUT, unlikely},
    pointer::JsonPointer,
    serde::{error::Kind, unchecked::Unchecked},
    source::{NullPadded, Source, Volatility},
};

pub use error::Error;

// todo: error

pub type Result<T> = core::result::Result<T, Error>;

impl<S: Source, C: Config> Parser<'_, S, C> {
    fn skip_whitespace_alt(&mut self) -> u8 {
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
                return tmp;
            }
        }
    }

    unsafe fn parse_literal<'a, V: Visitor<'a>>(&mut self, visitor: V) -> Result<V::Value> {
        if S::Volatility::IS_VOLATILE {
            let tmp = self.idx().wrapping_add(1);
            self.src.trim(tmp);
        }

        let tmp = self.skip_whitespace();

        if NUM_LUT[tmp as usize] {
            let neg = tmp == b'-';
            if neg {
                self.inc(1)
            }

            if unlikely(
                !S::NULL_PADDED && self.idx() == self.src.len() || !NUM_LUT[self.cur() as usize],
            ) {
                return Err(Kind::InvalidLiteral.into());
            }

            if self.cur() == b'0'
                && (S::NULL_PADDED || self.idx() + 1 != self.src.len())
                && matches!(*self.cur_ptr().add(1), b'0'..=b'9')
            {
                return Err(Kind::LeadingZero.into());
            }

            let start = self.idx();
            let (val, is_int) = self.parse_u64();

            'int: {
                if is_int {
                    self.dec();
                    return if neg {
                        if val > 9223372036854775808 {
                            break 'int;
                        }

                        visitor.visit_i64(val.wrapping_neg() as _)
                    } else {
                        visitor.visit_u64(val)
                    };
                }
            }

            if start == self.idx() {
                return Err(Kind::LeadingDecimal.into());
            }

            if let Some(val) = self.parse_f64(val, neg, start) {
                self.dec();
                return if val.is_finite() {
                    visitor.visit_f64(val)
                } else {
                    Err(Kind::NumberOverflow.into())
                };
            }

            return Err(select_unpredictable(
                *self.cur_ptr().sub(1) == b'.',
                Kind::TrailingDecimal,
                Kind::InvalidLiteral,
            )
            .into());
        }

        self.inc(3);
        if S::NULL_PADDED || self.idx() < self.src.len() {
            let ptr = self.cur_ptr().sub(3);

            match ptr.cast::<u32>().read_unaligned() {
                0x6c6c756e => visitor.visit_unit(),
                0x65757274 => visitor.visit_bool(true),
                0x736c6166
                    if (S::NULL_PADDED || self.idx() + 1 != self.src.len())
                        && *ptr.add(4) == b'e' =>
                {
                    self.inc(1);
                    visitor.visit_bool(false)
                }
                _ => Err(Kind::InvalidLiteral.into()),
            }
        } else {
            Err(Kind::InvalidLiteral.into())
        }
    }
}

macro_rules! deserialize_literal {
    ($($name:ident),* $(,)?) => {
        $(
            fn $name<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
                unsafe { self.parse_literal(visitor) }
            }
        )*
    }
}

impl<'de, S: Source, C: Config> Deserializer<'de> for &mut Parser<'de, S, C> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let tmp = self.skip_whitespace();
        self.dec();

        match tmp {
            b'"' => self.deserialize_str(visitor),
            b'{' => self.deserialize_map(visitor),
            b'[' => self.deserialize_seq(visitor),
            0 => Err(Kind::Eof.into()),
            _ => unsafe { self.parse_literal(visitor) },
        }
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.skip_whitespace() != b'"' {
            return Err(Kind::UnexpectedToken.into());
        }

        if S::Volatility::IS_VOLATILE {
            let tmp = self.idx();
            self.src.trim(tmp);
        }

        // hmm...
        if !S::Volatility::IS_VOLATILE & S::INSITU {
            let start = unsafe { self.cur_ptr_mut().add(1) };
            let mut offset = start;
            let mut len = 0;
            let err = loop {
                if self.simd_str() {
                    continue;
                }

                self.inc(1);
                if !S::NULL_PADDED && self.idx() >= self.src.len() {
                    break Kind::UnclosedString;
                }

                break match self.cur() {
                    b'"' => unsafe {
                        let count = self.cur_ptr().offset_from_unsigned(offset);
                        if len != 0 {
                            start.add(len).copy_from(offset, count);
                        }

                        len += count;
                        let tmp = from_raw_parts(start, len);

                        return if S::UTF8 || from_utf8(tmp).is_ok() {
                            visitor.visit_borrowed_str(from_utf8_unchecked(tmp))
                        } else {
                            Err(Kind::UnexpectedToken.into())
                        };
                    },
                    b'\\' => unsafe {
                        let count = self.cur_ptr().offset_from_unsigned(offset);

                        start.add(len).copy_from(offset, count);
                        self.inc(1);

                        len += count;
                        offset = self.cur_ptr_mut().add(1);

                        if !S::NULL_PADDED && self.idx() == self.src.len() {
                            break Kind::UnclosedString;
                        }

                        let ptr = start.add(len);
                        let tmp = self.cur();
                        let esc = ESC_LUT[tmp as usize];

                        if esc != 0 {
                            ptr.write(esc);
                            len += 1;
                            continue;
                        }

                        if tmp == b'u'
                            && let Some(v) = self.unicode_escape(&mut [0; 4])
                        {
                            ptr.copy_from_nonoverlapping(v.as_ptr(), v.len());
                            offset = self.cur_ptr_mut().add(1);
                            len += v.len();
                            continue;
                        }

                        select_unpredictable(
                            S::NULL_PADDED && tmp == 0,
                            Kind::UnclosedString,
                            Kind::InvalidEscapeSequnce,
                        )
                    },
                    c if !c.is_ascii_control() => continue,
                    0 => Kind::UnclosedString,
                    _ => Kind::ControlCharacter,
                };
            };

            Err(err.into())
        } else if !S::Volatility::IS_VOLATILE & !S::INSITU {
            let mut offset = unsafe { self.cur_ptr().add(1) };
            let mut buf = dangling_mut();
            let mut cap = 0;
            let mut len = 0;

            'main: {
                let err = loop {
                    if self.simd_str() {
                        continue;
                    }

                    self.inc(1);
                    if !S::NULL_PADDED && self.idx() >= self.src.len() {
                        break Kind::UnclosedString;
                    }

                    break match self.cur() {
                        b'"' => break 'main,
                        b'\\' => unsafe {
                            let count = self.cur_ptr().offset_from_unsigned(offset);
                            let new_len = len + count + 4;

                            if cap < new_len {
                                let tmp = new_len * 5 / 4;
                                let layout = Layout::array::<u8>(tmp).unwrap_unchecked();

                                buf = if cap != 0 {
                                    realloc(
                                        buf,
                                        Layout::array::<u8>(cap).unwrap_unchecked(),
                                        layout.size(),
                                    )
                                } else {
                                    alloc(layout)
                                };
                                cap = tmp;
                            }

                            buf.add(len).copy_from_nonoverlapping(offset, count);
                            self.inc(1);

                            len += count;
                            offset = self.cur_ptr().add(1);

                            if !S::NULL_PADDED && self.idx() == self.src.len() {
                                break Kind::UnclosedString;
                            }

                            let tmp = self.cur();
                            let buf = buf.add(len);
                            let esc = ESC_LUT[tmp as usize];

                            if esc != 0 {
                                buf.write(esc);
                                len += 1;
                                continue;
                            }

                            if tmp == b'u'
                                && let Some(v) = self.unicode_escape(&mut [0; 4])
                            {
                                buf.copy_from_nonoverlapping(v.as_ptr(), v.len());
                                offset = self.cur_ptr().add(1);
                                len += v.len();
                                continue;
                            }

                            select_unpredictable(
                                S::NULL_PADDED && tmp == 0,
                                Kind::UnclosedString,
                                Kind::InvalidEscapeSequnce,
                            )
                        },
                        c if !c.is_ascii_control() => continue,
                        0 if S::NULL_PADDED => Kind::Eof,
                        _ => Kind::ControlCharacter,
                    };
                };

                return unsafe {
                    if cap != 0 {
                        dealloc(buf, Layout::array::<u8>(cap).unwrap_unchecked());
                    }

                    Err(err.into())
                };
            }

            if len == 0 {
                return unsafe {
                    let tmp = from_raw_parts(offset, self.cur_ptr().offset_from_unsigned(offset));

                    if S::UTF8 || from_utf8(tmp).is_ok() {
                        visitor.visit_borrowed_str(from_utf8_unchecked(tmp))
                    } else {
                        Err(Kind::UnexpectedToken.into())
                    }
                };
            }

            let count = unsafe { self.cur_ptr().offset_from_unsigned(offset) };
            let new_len = len + count;

            if cap < new_len {
                buf = unsafe {
                    // if the string had esc chars then it would've allocated already if
                    // not it is returned above as borrowed string. so we can just use
                    // realloc without checking.
                    realloc(
                        buf,
                        Layout::array::<u8>(cap).unwrap_unchecked(),
                        Layout::array::<u8>(new_len).unwrap_unchecked().size(),
                    )
                };
                cap = new_len;
            }

            return unsafe {
                buf.add(len).copy_from_nonoverlapping(offset, count);

                if S::UTF8 || from_utf8(from_raw_parts(buf, new_len)).is_ok() {
                    visitor.visit_string(String::from_raw_parts(buf, new_len, cap))
                } else {
                    Err(Kind::UnexpectedToken.into())
                }
            };
        } else {
            let mut offset = self.idx() + 1;
            let mut buf = dangling_mut();
            let mut cap = 0;
            let mut len = 0;

            'main: {
                let err = loop {
                    if self.simd_str() {
                        continue;
                    }

                    self.inc(1);
                    if !S::NULL_PADDED && self.idx() >= self.src.len() {
                        break Kind::UnclosedString;
                    }

                    break match self.cur() {
                        b'"' => break 'main,
                        b'\\' => unsafe {
                            let count = self.idx() - offset;
                            let new_len = len + count + 4;

                            if cap < new_len {
                                let tmp = new_len * 5 / 4;
                                let layout = Layout::array::<u8>(tmp).unwrap_unchecked();

                                buf = if cap != 0 {
                                    realloc(
                                        buf,
                                        Layout::array::<u8>(cap).unwrap_unchecked(),
                                        layout.size(),
                                    )
                                } else {
                                    alloc(layout)
                                };
                                cap = tmp;
                            }

                            buf.add(len)
                                .copy_from_nonoverlapping(self.src.ptr(offset), count);
                            self.inc(1);

                            len += count;
                            offset = self.idx() + 1;

                            if !S::NULL_PADDED && self.idx() == self.src.len() {
                                break Kind::UnclosedString;
                            }

                            let tmp = self.cur();
                            let buf = buf.add(len);
                            let esc = ESC_LUT[tmp as usize];

                            if esc != 0 {
                                buf.write(esc);
                                len += 1;
                                continue;
                            }

                            if tmp == b'u'
                                && let Some(v) = self.unicode_escape(&mut [0; 4])
                            {
                                buf.copy_from_nonoverlapping(v.as_ptr(), v.len());
                                offset = self.idx() + 1;
                                len += v.len();
                                continue;
                            }

                            Kind::InvalidEscapeSequnce
                        },
                        c if !c.is_ascii_control() => continue,
                        _ => Kind::ControlCharacter,
                    };
                };

                return unsafe {
                    if cap != 0 {
                        dealloc(buf, Layout::array::<u8>(cap).unwrap_unchecked())
                    }

                    Err(err.into())
                };
            }

            let count = self.idx() - offset;
            let new_len = len + count;

            if cap < new_len {
                buf = unsafe {
                    let layout = Layout::array::<u8>(new_len).unwrap_unchecked();

                    if cap == 0 {
                        alloc(layout)
                    } else {
                        realloc(
                            buf,
                            Layout::array::<u8>(cap).unwrap_unchecked(),
                            layout.size(),
                        )
                    }
                };
                cap = new_len;
            }

            unsafe {
                buf.add(len)
                    .copy_from_nonoverlapping(self.src.ptr(offset), count);

                if S::UTF8 || from_utf8(from_raw_parts(buf, new_len)).is_ok() {
                    visitor.visit_string(String::from_raw_parts(buf, new_len, cap))
                } else {
                    Err(Kind::UnexpectedToken.into())
                }
            }
        }
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.skip_whitespace() {
            b'n' => unsafe {
                if (S::NULL_PADDED || self.idx() + 3 < self.src.len())
                    && from_raw_parts(self.cur_ptr(), 4) == b"null"
                {
                    self.inc(3);
                    visitor.visit_none()
                } else {
                    Err(Kind::InvalidLiteral.into())
                }
            },
            _ => {
                self.dec();
                visitor.visit_some(self)
            }
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if S::Volatility::IS_VOLATILE {
            let tmp = self.idx().wrapping_add(1);
            self.src.trim(tmp);
        }

        Err(match self.skip_whitespace() {
            b'[' => {
                let tmp = visitor.visit_seq(CommaSeparated::new(self))?;

                if self.skip_whitespace_alt() == b']' {
                    return Ok(tmp);
                } else {
                    Kind::UnexpectedToken
                }
            }
            0 => Kind::Eof,
            _ => Kind::UnexpectedToken,
        }
        .into())
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _: usize, visitor: V) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: usize,
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if S::Volatility::IS_VOLATILE {
            let tmp = self.idx().wrapping_add(1);
            self.src.trim(tmp);
        }

        Err(match self.skip_whitespace() {
            b'{' => return visitor.visit_map(CommaSeparated::new(self)),
            0 => Kind::Eof,
            _ => Kind::UnexpectedToken,
        }
        .into())
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        Err(match self.skip_whitespace() {
            b'{' => return visitor.visit_map(CommaSeparated::new(self)),
            b'[' => {
                let tmp = visitor.visit_seq(CommaSeparated::new(self))?;

                if self.skip_whitespace_alt() == b']' {
                    return Ok(tmp);
                } else {
                    Kind::UnexpectedToken
                }
            }
            _ => Kind::UnexpectedToken,
        }
        .into())
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        Err(match self.skip_whitespace() {
            b'{' => {
                let tmp = visitor.visit_enum(VariantAccess(self))?;

                match self.skip_whitespace() {
                    b'}' => return Ok(tmp),
                    _ => Kind::UnexpectedToken,
                }
            }
            b'"' => return visitor.visit_enum(UnitVariantAccess(self)),
            0 => Kind::Eof,
            _ => Kind::UnexpectedToken,
        }
        .into())
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        #[doc(hidden)]
        #[allow(non_local_definitions)]
        impl crate::value::builder::ErrorBuilder for Error {
            #[inline]
            fn eof() -> Self {
                Kind::Eof.into()
            }

            #[inline]
            fn expected_colon() -> Self {
                Kind::ExpectedColon.into()
            }

            #[inline]
            fn expected_value() -> Self {
                Kind::Eof.into()
            }

            #[inline]
            fn trailing_comma() -> Self {
                Kind::TrailingComma.into()
            }

            #[inline]
            fn unclosed_string() -> Self {
                Kind::UnclosedString.into()
            }

            #[inline]
            fn invalid_escape() -> Self {
                Kind::InvalidEscapeSequnce.into()
            }

            #[inline]
            fn control_character() -> Self {
                Kind::ControlCharacter.into()
            }

            #[inline]
            fn invalid_literal() -> Self {
                Kind::InvalidLiteral.into()
            }

            #[inline]
            fn trailing_decimal() -> Self {
                Kind::TrailingDecimal.into()
            }

            #[inline]
            fn leading_decimal() -> Self {
                Kind::LeadingDecimal.into()
            }

            #[inline]
            fn leading_zero() -> Self {
                Kind::LeadingZero.into()
            }

            #[inline]
            fn number_overflow() -> Self {
                Kind::NumberOverflow.into()
            }

            #[inline]
            fn unexpected_token() -> Self {
                Kind::UnexpectedToken.into()
            }

            #[inline]
            fn apply_span(&mut self, _: usize, _: usize) {}
        }

        self.skip_value()?;
        visitor.visit_unit()
    }

    // todo: dunno what they do
    forward_to_deserialize_any! {
        bytes byte_buf
    }

    deserialize_literal! {
        deserialize_i8,
        deserialize_i16,
        deserialize_i32,
        deserialize_i64,

        deserialize_u8,
        deserialize_u16,
        deserialize_u32,
        deserialize_u64,

        deserialize_f32,
        deserialize_f64,

        deserialize_bool,
        deserialize_unit,
    }
}

struct CommaSeparated<'a, 'de, S: Source, C: Config> {
    de: &'a mut Parser<'de, S, C>,
    flag: bool,
}

impl<'a, 'de, S: Source, C: Config> CommaSeparated<'a, 'de, S, C> {
    #[inline(always)]
    fn new(de: &'a mut Parser<'de, S, C>) -> Self {
        CommaSeparated { de, flag: true }
    }
}

impl<'a, 'de, S: Source, C: Config> MapAccess<'de> for CommaSeparated<'a, 'de, S, C> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        let mut wtf = true;

        loop {
            let tmp = self.de.skip_whitespace();
            let err = match tmp {
                b'"' if self.flag => {
                    self.de.dec();
                    self.flag = false;
                    return seed.deserialize(&mut *self.de).map(Some);
                }
                b',' => {
                    if !self.flag {
                        self.flag = true;
                        wtf = false;
                        continue;
                    }

                    Kind::UnexpectedToken
                }
                b'}' => match wtf || self.de.cfg.trailing_comma() {
                    true => return Ok(None),
                    _ => Kind::TrailingComma,
                },
                0 => Kind::Eof,
                _ if self.de.cfg.comma() => {
                    self.flag = true;
                    self.de.dec();
                    continue;
                }
                _ => Kind::UnexpectedToken,
            };

            return Err(err.into());
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        if self.de.skip_whitespace() != b':' {
            return Err(Kind::ExpectedColon.into());
        }

        seed.deserialize(&mut *self.de)
    }
}

impl<'a, 'de, S: Source, C: Config> SeqAccess<'de> for CommaSeparated<'a, 'de, S, C> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        let mut wtf = true;

        loop {
            let err = match self.de.skip_whitespace() {
                b']' => match wtf || self.de.cfg.trailing_comma() {
                    true => {
                        self.de.dec();
                        return Ok(None);
                    }
                    _ => Kind::TrailingComma,
                },
                _ if self.flag => {
                    self.de.dec();
                    self.flag = false;
                    return seed.deserialize(&mut *self.de).map(Some);
                }
                b',' if !self.flag => {
                    self.flag = true;
                    wtf = false;
                    continue;
                }
                0 => Kind::Eof,
                _ if self.de.cfg.comma() => {
                    self.flag = true;
                    self.de.dec();
                    continue;
                }
                _ => Kind::UnexpectedToken,
            };

            return Err(err.into());
        }
    }
}

struct VariantAccess<'a, 'de, S: Source, C: Config>(&'a mut Parser<'de, S, C>);

impl<'a, 'de, S: Source, C: Config> EnumAccess<'de> for VariantAccess<'a, 'de, S, C> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        let tmp = seed.deserialize(&mut *self.0)?;

        if self.0.skip_whitespace() == b':' {
            Ok((tmp, self))
        } else {
            Err(Kind::ExpectedColon.into())
        }
    }
}

impl<'a, 'de, S: Source, C: Config> de::VariantAccess<'de> for VariantAccess<'a, 'de, S, C> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Deserialize::deserialize(self.0)
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        seed.deserialize(self.0)
    }

    fn tuple_variant<V: Visitor<'de>>(self, _: usize, visitor: V) -> Result<V::Value> {
        de::Deserializer::deserialize_seq(self.0, visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        de::Deserializer::deserialize_struct(self.0, "", fields, visitor)
    }
}

struct UnitVariantAccess<'a, 'de, S: Source, C: Config>(&'a mut Parser<'de, S, C>);

impl<'a, 'de, S: Source, C: Config> EnumAccess<'de> for UnitVariantAccess<'a, 'de, S, C> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self)> {
        self.0.dec();
        Ok((seed.deserialize(&mut *self.0)?, self))
    }
}

impl<'a, 'de, S: Source, C: Config> de::VariantAccess<'de> for UnitVariantAccess<'a, 'de, S, C> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, _: T) -> Result<T::Value> {
        Err(de::Error::invalid_type(
            Unexpected::NewtypeVariant,
            &"newtype variant",
        ))
    }

    fn tuple_variant<V: Visitor<'de>>(self, _: usize, _: V) -> Result<V::Value> {
        Err(de::Error::invalid_type(
            Unexpected::UnitVariant,
            &"tuple variant",
        ))
    }

    fn struct_variant<V: Visitor<'de>>(self, _: &'static [&'static str], _: V) -> Result<V::Value> {
        Err(de::Error::invalid_type(
            Unexpected::UnitVariant,
            &"struct variant",
        ))
    }
}

/// Deserializes specified type from a JSON string input.
///
/// # Errors
/// Returns an error if the JSON is malformed or cannot be deserialized into type `T`.
///
/// # Example
/// ```
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Book {
///     name: String,
///     pages: u32,
/// }
///
/// let json = r#"{"name": "idk", "pages": 256}"#;
/// let person: Book = flexon::from_str(json)?;
/// # Ok::<(), flexon::serde::Error>(())
/// ```
#[inline]
pub fn from_str<'a, T: Deserialize<'a>>(s: &'a str) -> Result<T> {
    T::deserialize(&mut Parser::from_str(s))
}

/// Deserializes specified type from a JSON string input.
///
/// This will perform In-situ parsing. The provided input may be no longer valid JSON or even UTF-8.
///
/// # Errors
/// Returns an error if the JSON is malformed or cannot be deserialized into type `T`.
///
/// # Example
/// ```
/// use serde::Deserialize;
///
/// let mut json = String::from(r#""foo\/bar""#);
/// let res: &str = unsafe { flexon::from_mut_str(&mut json)? };
///
/// assert_eq!(res, "foo/bar");
/// # Ok::<(), flexon::serde::Error>(())
/// ```
#[inline]
pub unsafe fn from_mut_str<'a, T: Deserialize<'a>>(s: &'a mut str) -> Result<T> {
    T::deserialize(&mut Parser::from_mut_str(s))
}

/// Deserializes specified type from a JSON byte input.
///
/// Same as [`from_str`] but will perform UTF-8 validation.
#[inline]
pub fn from_slice<'a, T: Deserialize<'a>>(s: &'a [u8]) -> Result<T> {
    T::deserialize(&mut Parser::from_slice(s))
}

/// Deserializes specified type from a JSON byte input.
///
/// Same as [`from_mut_str`] but will perform UTF-8 validation.
#[inline]
pub fn from_mut_slice<'a, T: Deserialize<'a>>(s: &'a mut [u8]) -> Result<T> {
    T::deserialize(&mut Parser::from_mut_slice(s))
}

/// Deserializes specified type from a JSON byte input.
///
/// Same as [`from_str`] and will not perform UTF-8 validation.
#[inline]
pub unsafe fn from_slice_unchecked<'a, T: Deserialize<'a>>(s: &'a [u8]) -> Result<T> {
    T::deserialize(&mut Parser::from_slice_unchecked(s))
}

/// Deserializes specified type from a JSON byte input.
///
/// Same as [`from_mut_str`] and will not perform UTF-8 validation.
#[inline]
pub unsafe fn from_mut_slice_unchecked<'a, T: Deserialize<'a>>(s: &'a mut [u8]) -> Result<T> {
    T::deserialize(&mut Parser::from_mut_slice_unchecked(s))
}

/// Deserializes specified type from null padded JSON input.
///
/// Same as [`from_str`] and will not perform UTF-8 validation.
#[inline]
pub fn from_null_padded<'a, T: Deserialize<'a>>(buf: &'a NullPadded) -> Result<T> {
    T::deserialize(&mut Parser::new(buf))
}

/// Deserializes specified type from null padded JSON input.
///
/// Same as [`from_mut_str`] and will not perform UTF-8 validation.
#[inline]
pub fn from_mut_null_padded<'a, T: Deserialize<'a>>(buf: &'a mut NullPadded) -> Result<T> {
    T::deserialize(&mut Parser::new(buf))
}

/// Deserializes specified type from a streaming source.
///
/// Reads JSON data incrementally from any type implementing [`Read`].
///
/// # Errors
/// Returns an error if the JSON is malformed or cannot be deserialized into type `T`.
///
/// # Example
/// ```no_run
/// use serde::Deserialize;
/// use std::fs::File;
///
/// let file = File::open("names.json")?;
/// let config: Vec<String> = flexon::from_reader(file)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[inline]
pub fn from_reader<R: Read, T: DeserializeOwned>(rdr: R) -> Result<T> {
    T::deserialize(&mut Parser::from_reader(rdr))
}

/// Deserializes specified type from a streaming source.
///
/// Same as [`from_reader`] but will not perform UTF-8 validation.
#[inline]
pub unsafe fn from_reader_unchecked<R: Read, T: DeserializeOwned>(rdr: R) -> Result<T> {
    T::deserialize(&mut Parser::from_reader_unchecked(rdr))
}

/// Skips to the given path and deserializes the type using the provided parser.
///
/// Same as [`get_from`] but takes parser as an argument.
/// Useful in case you want to modify the default parsing behaviour.
///
/// # Example
/// ```
/// use flexon::{Parser, serde::*, config::CTConfig};
///
/// let src = r#"{"one": 1 "two": 2}"#;
/// let config = CTConfig::new().optional_comma();
/// let mut parser = Parser::new_with(src, config);
/// let val: u8 = get_with_parser(["two"], &mut parser).unwrap();
///
/// assert_eq!(val, 2);
/// ```
pub fn get_with_parser<'a, S, C, T, P>(path: P, parser: &mut Parser<'a, S, C>) -> Result<T>
where
    S: Source + 'a,
    C: Config,
    T: Deserialize<'a>,
    P: IntoIterator,
    P::Item: JsonPointer,
{
    parser.skip_to(path)?;
    parser.dec();
    T::deserialize(parser)
}

/// Skips to the given path and deserializes the type using the provided parser.
///
/// This function's behavior is undefined if any of the following conditions are not met:
///
/// - The JSON must be valid.
/// - The path must exist.
/// - The specified type must be deserializable from the provided JSON data.
pub unsafe fn get_with_parser_unchecked<'a, S, C, T, P>(path: P, parser: &mut Parser<'a, S, C>) -> T
where
    S: Source + 'a,
    C: Config,
    T: Deserialize<'a>,
    P: IntoIterator,
    P::Item: JsonPointer,
{
    parser.skip_to_unchecked(path);
    parser.dec();
    T::deserialize(&mut Unchecked(parser)).unwrap_unchecked()
}

/// Skips to the given path and deserializes the specified type.
///
/// Useful when you want to parse only a portion of the JSON data. This will
/// skip and validate the JSON as it moves forward and return early. As such,
/// any trailing data is ignored. Returns error if the path does not exist.
///
/// # Example
/// ```
/// use flexon::{jsonp, serde::error::Kind};
///
/// let src = r#"{"pair": [64,]}"#;
/// let num: u8 = flexon::get_from(src, jsonp!["pair", 0]).unwrap();
/// let invalid = flexon::get_from::<_, u8, _>(src, jsonp!["pair", 1]);
///
/// assert_eq!(num, 64);
/// assert_eq!(invalid.unwrap_err().kind(), &Kind::TrailingComma);
/// ```
#[inline]
pub fn get_from<'a, S, T, P>(src: S, path: P) -> Result<T>
where
    S: Source + 'a,
    T: Deserialize<'a>,
    P: IntoIterator,
    P::Item: JsonPointer,
{
    get_with_parser(path, &mut Parser::new(src))
}

/// Skips to the given path and deserializes the specified type.
///
/// Similar to [`get_from`], but without validation.
/// This function's behavior is undefined if any of the following conditions are not met:
///
/// - The JSON must be valid.
/// - The path must exist.
/// - The specified type must be deserializable from the provided JSON data.
///
/// # Example
/// ```
/// let src = r#"{"segfault?": 28526}"#;
/// let res: u16 = unsafe { flexon::get_from_unchecked(src, ["segfault?"]) };
///
/// assert_eq!(&res.to_le_bytes(), b"no");
/// ```
#[inline]
pub unsafe fn get_from_unchecked<'a, S, T, P>(src: S, path: P) -> T
where
    S: Source + 'a,
    T: Deserialize<'a>,
    P: IntoIterator,
    P::Item: JsonPointer,
{
    get_with_parser_unchecked(path, &mut Parser::new(src))
}
