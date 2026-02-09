use core::{
    alloc::Layout,
    fmt::{self, Debug, Display, Formatter},
    hint::unreachable_unchecked,
    ops::{Deref, DerefMut},
    ptr::dangling_mut,
    slice::from_raw_parts,
    str::from_utf8_unchecked,
};
use std::alloc::{alloc, realloc};

use serde::{
    Deserialize, Deserializer,
    de::{self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor},
    forward_to_deserialize_any,
};

use crate::{
    Parser,
    config::Config,
    misc::{ESC_LUT, NUM_LUT},
    source::{Source, Volatility},
};

// so cringe. doing all these things for some insignificant gains

type Result<T> = core::result::Result<T, Error>;

#[repr(transparent)]
pub struct Unchecked<'a, 'de, S: Source, C: Config>(pub &'a mut Parser<'de, S, C>);

impl<S: Source, C: Config> Unchecked<'_, '_, S, C> {
    fn skip_whitespace_alt(&mut self) {
        loop {
            self.inc(1);
            let tmp = self.cur();

            if !matches!(tmp, b' ' | b'\t' | b'\n' | b'\r') {
                return;
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

            let start = self.idx();
            let (val, is_int) = self.parse_u64();
            let num = match is_int {
                true => match neg {
                    true => visitor.visit_i64(val.wrapping_neg() as _),
                    _ => visitor.visit_u64(val),
                },
                _ => visitor.visit_f64(self.parse_f64(val, neg, start).unwrap_unchecked()),
            };

            self.dec();
            return num;
        }

        let tmp = match self.cur() {
            b'n' => visitor.visit_unit(),
            c => {
                let val = c == b't';
                self.inc(!val as _);
                visitor.visit_bool(val)
            }
        };

        self.inc(3);
        tmp
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

impl<'de, S: Source, C: Config> Deserializer<'de> for &mut Unchecked<'_, 'de, S, C> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let tmp = self.skip_whitespace();
        self.dec();

        match tmp {
            b'"' => self.deserialize_str(visitor),
            b'{' => self.deserialize_map(visitor),
            b'[' => self.deserialize_seq(visitor),
            _ => unsafe { self.parse_literal(visitor) },
        }
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.skip_whitespace();

        if S::Volatility::IS_VOLATILE {
            let tmp = self.idx();
            self.src.trim(tmp);
        }

        // hmm...
        if !S::Volatility::IS_VOLATILE & S::INSITU {
            let start = unsafe { self.cur_ptr_mut().add(1) };
            let mut offset = start;
            let mut len = 0;

            loop {
                if self.simd_str_unchecked() {
                    continue;
                }

                self.inc(1);
                break match self.cur() {
                    b'"' => unsafe {
                        let count = self.cur_ptr().offset_from_unsigned(offset);
                        if len != 0 {
                            start.add(len).copy_from(offset, count);
                        }

                        len += count;
                        let tmp = from_raw_parts(start, len);
                        visitor.visit_borrowed_str(from_utf8_unchecked(tmp))
                    },
                    b'\\' => unsafe {
                        let count = self.cur_ptr().offset_from_unsigned(offset);

                        start.add(len).copy_from(offset, count);
                        self.inc(1);

                        len += count;
                        offset = self.cur_ptr_mut().add(1);

                        let ptr = start.add(len);
                        let tmp = self.cur();
                        let esc = ESC_LUT[tmp as usize];

                        if esc != 0 {
                            ptr.write(esc);
                            len += 1;
                            continue;
                        }

                        let mut tmp = [0; 4];
                        let esc = self.unicode_escape(&mut tmp).unwrap_unchecked();

                        ptr.copy_from_nonoverlapping(esc.as_ptr(), esc.len());
                        offset = self.cur_ptr_mut().add(1);
                        len += esc.len();
                        continue;
                    },
                    _ => continue,
                };
            }
        } else if !S::Volatility::IS_VOLATILE & !S::INSITU {
            let mut offset = unsafe { self.cur_ptr().add(1) };
            let mut buf = dangling_mut();
            let mut cap = 0;
            let mut len = 0;

            loop {
                if self.simd_str_unchecked() {
                    continue;
                }

                self.inc(1);
                match self.cur() {
                    b'"' => break,
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

                        let tmp = self.cur();
                        let buf = buf.add(len);
                        let esc = ESC_LUT[tmp as usize];

                        if esc != 0 {
                            buf.write(esc);
                            len += 1;
                            continue;
                        }

                        let mut tmp = [0; 4];
                        let esc = self.unicode_escape(&mut tmp).unwrap_unchecked();

                        buf.copy_from_nonoverlapping(esc.as_ptr(), esc.len());
                        offset = self.cur_ptr().add(1);
                        len += esc.len();
                        continue;
                    },
                    _ => continue,
                };
            }

            if len == 0 {
                return unsafe {
                    visitor.visit_borrowed_str(from_utf8_unchecked(from_raw_parts(
                        offset,
                        self.cur_ptr().offset_from_unsigned(offset),
                    )))
                };
            }

            let count = unsafe { self.cur_ptr().offset_from_unsigned(offset) };
            let new_len = len + count;

            if cap < new_len {
                buf = unsafe {
                    realloc(
                        buf,
                        Layout::array::<u8>(cap).unwrap_unchecked(),
                        Layout::array::<u8>(new_len).unwrap_unchecked().size(),
                    )
                };
                cap = new_len;
            }

            unsafe {
                buf.add(len).copy_from_nonoverlapping(offset, count);
                visitor.visit_string(String::from_raw_parts(buf, new_len, cap))
            }
        } else {
            let mut offset = self.idx() + 1;
            let mut buf = dangling_mut();
            let mut cap = 0;
            let mut len = 0;
            loop {
                if self.simd_str_unchecked() {
                    continue;
                }

                self.inc(1);
                match self.cur() {
                    b'"' => break,
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

                        let tmp = self.cur();
                        let buf = buf.add(len);
                        let esc = ESC_LUT[tmp as usize];

                        if esc != 0 {
                            buf.write(esc);
                            len += 1;
                            continue;
                        }

                        let mut tmp = [0; 4];
                        let esc = self.unicode_escape(&mut tmp).unwrap_unchecked();

                        buf.copy_from_nonoverlapping(esc.as_ptr(), esc.len());
                        offset = self.idx() + 1;
                        len += esc.len();
                        continue;
                    },
                    _ => continue,
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
                visitor.visit_string(String::from_raw_parts(buf, new_len, cap))
            }
        }
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.skip_whitespace() {
            b'n' => {
                self.inc(3);
                visitor.visit_none()
            }
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

        self.skip_whitespace();
        let tmp = visitor.visit_seq(&mut *self);
        self.skip_whitespace_alt();
        tmp
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

        self.skip_whitespace();
        visitor.visit_map(self)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        match self.skip_whitespace() {
            b'{' => visitor.visit_map(self),
            b'[' => {
                let tmp = visitor.visit_seq(&mut *self);
                self.skip_whitespace_alt();
                tmp
            }
            _ => unsafe { unreachable_unchecked() },
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        match self.skip_whitespace() {
            b'{' => {
                let tmp = visitor.visit_enum(&mut *self);
                self.skip_whitespace();
                tmp
            }
            b'"' => visitor.visit_enum(UnitVariantAccess(self)),
            _ => unsafe { unreachable_unchecked() },
        }
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.skip_value_unchecked();
        visitor.visit_unit()
    }

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

impl<'de, S: Source, C: Config> MapAccess<'de> for Unchecked<'_, 'de, S, C> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        loop {
            return Ok(match self.skip_whitespace() {
                b'"' => unsafe {
                    self.dec();
                    Some(seed.deserialize(self).unwrap_unchecked())
                },
                b',' => continue,
                b'}' => None,
                _ => unsafe { unreachable_unchecked() },
            });
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        self.skip_whitespace();
        seed.deserialize(self)
    }
}

impl<'de, S: Source, C: Config> SeqAccess<'de> for Unchecked<'_, 'de, S, C> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        loop {
            return Ok(match self.skip_whitespace() {
                b',' => continue,
                b']' => {
                    self.dec();
                    None
                }
                _ => unsafe {
                    self.dec();
                    Some(seed.deserialize(self).unwrap_unchecked())
                },
            });
        }
    }
}

impl<'de, S: Source, C: Config> EnumAccess<'de> for &mut Unchecked<'_, 'de, S, C> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        unsafe {
            let tmp = seed.deserialize(&mut *self.0).unwrap_unchecked();
            self.skip_whitespace();
            Ok((tmp, self))
        }
    }
}

impl<'de, S: Source, C: Config> VariantAccess<'de> for &mut Unchecked<'_, 'de, S, C> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Deserialize::deserialize(self)
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        seed.deserialize(self)
    }

    fn tuple_variant<V: Visitor<'de>>(self, _: usize, visitor: V) -> Result<V::Value> {
        Deserializer::deserialize_seq(self, visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        Deserializer::deserialize_struct(self, "", fields, visitor)
    }
}

struct UnitVariantAccess<'a, 'de, S: Source, C: Config>(&'a mut Parser<'de, S, C>);

impl<'a, 'de, S: Source, C: Config> EnumAccess<'de> for UnitVariantAccess<'a, 'de, S, C> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self)> {
        unsafe {
            self.0.dec();
            Ok((seed.deserialize(&mut *self.0).unwrap_unchecked(), self))
        }
    }
}

impl<'a, 'de, S: Source, C: Config> de::VariantAccess<'de> for UnitVariantAccess<'a, 'de, S, C> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, _: T) -> Result<T::Value> {
        unsafe { unreachable_unchecked() }
    }

    fn tuple_variant<V: Visitor<'de>>(self, _: usize, _: V) -> Result<V::Value> {
        unsafe { unreachable_unchecked() }
    }

    fn struct_variant<V: Visitor<'de>>(self, _: &'static [&'static str], _: V) -> Result<V::Value> {
        unsafe { unreachable_unchecked() }
    }
}

pub struct Error;

impl Debug for Error {
    #[inline]
    fn fmt(&self, _: &mut Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl Display for Error {
    #[inline]
    fn fmt(&self, _: &mut Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl de::Error for Error {
    #[inline]
    fn custom<T: Display>(_: T) -> Self {
        Self
    }
}

impl core::error::Error for Error {}

impl<'de, S: Source, C: Config> Deref for Unchecked<'_, 'de, S, C> {
    type Target = Parser<'de, S, C>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<S: Source, C: Config> DerefMut for Unchecked<'_, '_, S, C> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}
