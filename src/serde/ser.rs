//! Serialize JSON using serde.

use super::format::*;
use core::fmt::{self, Display, Formatter};
use serde_core::ser::{
    self, Impossible, Serialize, SerializeMap, SerializeSeq, SerializeStruct,
    SerializeStructVariant, SerializeTuple, SerializeTupleStruct, SerializeTupleVariant,
};
use std::io::Write;

pub type Result<T> = core::result::Result<T, Error>;

/// JSON serializing structure.
pub struct Serializer<W: Write, F: Format> {
    writer: W,
    format: F,
}

impl<W: Write, F: Format> Serializer<W, F> {
    #[inline(always)]
    pub(super) fn write(&mut self, v: u8) -> Result<()> {
        match self.writer.write_all(&[v]) {
            Ok(_) => Ok(()),
            _ => Err(Error),
        }
    }

    #[inline(always)]
    pub(super) fn write_n(&mut self, v: &[u8]) -> Result<()> {
        match self.writer.write_all(v) {
            Ok(_) => Ok(()),
            _ => Err(Error),
        }
    }

    #[inline(always)]
    fn comma(&mut self, flag: &mut bool) -> Result<()> {
        if *flag {
            match self.writer.write_all(b",") {
                Ok(_) => Ok(()),
                _ => Err(Error),
            }
        } else {
            *flag = true;
            Ok(())
        }
    }
}

impl<'a, W: Write, F: Format> ser::Serializer for &'a mut Serializer<W, F> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Container<'a, W, F>;
    type SerializeTuple = Container<'a, W, F>;
    type SerializeTupleStruct = Container<'a, W, F>;
    type SerializeTupleVariant = Container<'a, W, F>;
    type SerializeMap = Container<'a, W, F>;
    type SerializeStruct = Container<'a, W, F>;
    type SerializeStructVariant = Container<'a, W, F>;

    #[inline]
    fn serialize_bool(self, v: bool) -> Result<()> {
        self.write_n(if v { b"true" } else { b"false" })
    }

    #[inline]
    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i64(v as _)
    }

    #[inline]
    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i64(v as _)
    }

    #[inline]
    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_i64(v as _)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.write_n(itoa::Buffer::new().format(v).as_bytes())
    }

    #[inline]
    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_u64(v as _)
    }

    #[inline]
    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_u64(v as _)
    }

    #[inline]
    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_u64(v as _)
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.write_n(itoa::Buffer::new().format(v).as_bytes())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        let mut tmp;

        self.write_n(if v.is_finite() {
            tmp = zmij::Buffer::new();
            tmp.format_finite(v).as_bytes()
        } else {
            b"null"
        })
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        let mut tmp;

        self.write_n(if v.is_finite() {
            tmp = zmij::Buffer::new();
            tmp.format_finite(v).as_bytes()
        } else {
            b"null"
        })
    }

    #[inline]
    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_str(v.encode_utf8(&mut [0; 4]))
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        const ESC: [u8; 256] = {
            let mut tmp = [0; 256];
            let mut idx = 0;

            while idx != 32 {
                tmp[idx] = b'u';
                idx += 1;
            }

            tmp[b'\x08' as usize] = b'b';
            tmp[b'\x0C' as usize] = b'f';
            tmp[b'\\' as usize] = b'\\';
            tmp[b'\n' as usize] = b'n';
            tmp[b'\t' as usize] = b't';
            tmp[b'\r' as usize] = b'r';
            tmp[b'"' as usize] = b'"';

            tmp
        };
        const CTRL: [u8; 64] = *b"000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F";

        self.write(b'"')?;
        if v.len() <= 8 {
            let mut rem = v.len();
            loop {
                if rem == 0 {
                    self.write_n(v.as_bytes())?;
                    return self.write(b'"');
                }

                if unsafe { ESC[*v.as_bytes().get_unchecked(v.len() - rem) as usize] != 0 } {
                    break;
                }
                rem -= 1;
            }
        }

        let mut tmp = crate::Parser::new(v);
        let mut offset = 0;

        loop {
            if tmp.simd_str() {
                continue;
            }

            tmp.inc(1);
            if tmp.idx() == v.len() {
                break;
            }

            let cur = tmp.cur();
            let esc = ESC[cur as usize];

            if esc == 0 {
                continue;
            }

            unsafe { self.write_n(v.get_unchecked(offset..tmp.idx()).as_bytes())? }
            offset = tmp.idx() + 1;

            if esc != b'u' {
                self.write_n(&[b'\\', esc])
            } else {
                unsafe {
                    let esc = CTRL.as_ptr().add(cur as usize * 2);
                    let seq = [b'\\', b'u', b'0', b'0', *esc, *esc.add(1)];

                    self.write_n(&seq)
                }
            }?
        }

        unsafe { self.write_n(v.get_unchecked(offset..).as_bytes())? }
        self.write(b'"')
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        self.write(b'[')?;
        let mut flag = false;

        for &v in v {
            self.comma(&mut flag)?;
            self.write_n(itoa::Buffer::new().format(v).as_bytes())?;
        }

        self.write(b']')
    }

    #[inline]
    fn serialize_none(self) -> Result<()> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<()> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        self.write_n(b"null")
    }

    #[inline]
    fn serialize_unit_struct(self, _: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(self, _: &'static str, _: u32, variant: &'static str) -> Result<()> {
        self.serialize_str(variant)
    }

    #[inline]
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<()> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()> {
        self.write(b'{')?;
        self.serialize_str(variant)?;
        self.write(b':')?;
        value.serialize(&mut *self)?;
        self.write(b'}')
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Container<'a, W, F>> {
        self.write(b'[')?;
        self.format.inc();
        Ok(Container {
            ser: self,
            flag: false,
        })
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Container<'a, W, F>> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_struct(self, _: &'static str, len: usize) -> Result<Container<'a, W, F>> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Container<'a, W, F>> {
        self.write(b'{')?;
        self.serialize_str(variant)?;
        self.write(b':')?;
        self.serialize_seq(Some(len))
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Container<'a, W, F>> {
        self.write(b'{')?;
        self.format.inc();
        Ok(Container {
            ser: self,
            flag: false,
        })
    }

    #[inline]
    fn serialize_struct(self, _: &'static str, len: usize) -> Result<Container<'a, W, F>> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Container<'a, W, F>> {
        self.write(b'{')?;
        self.serialize_str(variant)?;
        self.write(b':')?;
        self.serialize_map(Some(len))
    }
}

#[doc(hidden)]
pub struct Container<'a, W: Write, F: Format> {
    ser: &'a mut Serializer<W, F>,
    flag: bool,
}

impl<W: Write, F: Format> SerializeSeq for Container<'_, W, F> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.ser.comma(&mut self.flag)?;
        self.ser.format.indent(&mut self.ser.writer)?;
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<()> {
        self.ser.format.dec();
        if self.flag {
            self.ser.format.indent(&mut self.ser.writer)?
        }
        self.ser.write(b']')
    }
}

impl<W: Write, F: Format> SerializeTuple for Container<'_, W, F> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<()> {
        SerializeSeq::end(self)
    }
}

impl<W: Write, F: Format> SerializeTupleStruct for Container<'_, W, F> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<()> {
        SerializeSeq::end(self)
    }
}

impl<W: Write, F: Format> SerializeTupleVariant for Container<'_, W, F> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<()> {
        match self.ser.writer.write(b"]}") {
            Ok(_) => Ok(()),
            _ => Err(Error),
        }
    }
}

impl<W: Write, F: Format> SerializeMap for Container<'_, W, F> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<()> {
        self.ser.comma(&mut self.flag)?;
        self.ser.format.indent(&mut self.ser.writer)?;
        key.serialize(MapKey(self.ser))
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.ser.write(b':')?;
        self.ser.format.sep(&mut self.ser.writer)?;
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<()> {
        self.ser.format.dec();
        if self.flag {
            self.ser.format.indent(&mut self.ser.writer)?
        }
        self.ser.write(b'}')
    }
}

impl<W: Write, F: Format> SerializeStruct for Container<'_, W, F> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        SerializeMap::serialize_entry(self, key, value)
    }

    #[inline]
    fn end(self) -> Result<()> {
        SerializeMap::end(self)
    }
}

impl<W: Write, F: Format> SerializeStructVariant for Container<'_, W, F> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<()> {
        match self.ser.writer.write(b"}}") {
            Ok(_) => Ok(()),
            _ => Err(Error),
        }
    }
}

#[repr(transparent)]
struct MapKey<'a, W: Write, F: Format>(&'a mut Serializer<W, F>);

impl<W: Write, F: Format> ser::Serializer for MapKey<'_, W, F> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = Impossible<(), Error>;
    type SerializeStruct = Impossible<(), Error>;
    type SerializeStructVariant = Impossible<(), Error>;

    #[inline]
    fn serialize_bool(self, v: bool) -> Result<()> {
        self.0.write_n(match v {
            true => br#""true""#,
            _ => br#""false""#,
        })
    }

    #[inline]
    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i64(v as _)
    }

    #[inline]
    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i64(v as _)
    }

    #[inline]
    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_i64(v as _)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.0.write(b'"')?;
        self.0.write_n(itoa::Buffer::new().format(v).as_bytes())?;
        self.0.write(b'"')
    }

    #[inline]
    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_u64(v as _)
    }

    #[inline]
    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_u64(v as _)
    }

    #[inline]
    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_u64(v as _)
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.0.write(b'"')?;
        self.0.write_n(itoa::Buffer::new().format(v).as_bytes())?;
        self.0.write(b'"')
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.0.write(b'"')?;
        if let Err(_) = match v.is_finite() {
            true => {
                let mut tmp = zmij::Buffer::new();
                self.0.write_n(tmp.format_finite(v).as_bytes())
            }
            _ => self.0.write_n(b"null"),
        } {
            return Err(Error);
        }
        self.0.write(b'"')
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.0.write(b'"')?;
        if let Err(_) = match v.is_finite() {
            true => {
                let mut tmp = zmij::Buffer::new();
                self.0.write_n(tmp.format_finite(v).as_bytes())
            }
            _ => self.0.write_n(b"null"),
        } {
            return Err(Error);
        }
        self.0.write(b'"')
    }

    #[inline]
    fn serialize_char(self, v: char) -> Result<()> {
        self.0.serialize_char(v)
    }

    #[inline]
    fn serialize_str(self, v: &str) -> Result<()> {
        self.0.serialize_str(v)
    }

    #[inline]
    fn serialize_bytes(self, _: &[u8]) -> Result<()> {
        Err(Error)
    }

    #[inline]
    fn serialize_none(self) -> Result<()> {
        Err(Error)
    }

    #[inline]
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<()> {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> Result<()> {
        Err(Error)
    }

    #[inline]
    fn serialize_unit_struct(self, _: &'static str) -> Result<()> {
        Err(Error)
    }

    #[inline]
    fn serialize_unit_variant(self, _: &'static str, _: u32, variant: &'static str) -> Result<()> {
        self.0.serialize_str(variant)
    }

    #[inline]
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<()> {
        value.serialize(self)
    }

    #[inline]
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<()> {
        Err(Error)
    }

    #[inline]
    fn serialize_seq(self, _: Option<usize>) -> Result<Impossible<(), Error>> {
        Err(Error)
    }

    #[inline]
    fn serialize_tuple(self, _: usize) -> Result<Impossible<(), Error>> {
        Err(Error)
    }

    #[inline]
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Impossible<(), Error>> {
        Err(Error)
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Impossible<(), Error>> {
        Err(Error)
    }

    #[inline]
    fn serialize_map(self, _: Option<usize>) -> Result<Impossible<(), Error>> {
        Err(Error)
    }

    #[inline]
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Impossible<(), Error>> {
        Err(Error)
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Impossible<(), Error>> {
        Err(Error)
    }
}

/// Represents error occurred while serializing.
#[derive(Debug)]
pub struct Error;

impl ser::Error for Error {
    fn custom<T: Display>(_: T) -> Self {
        Self
    }
}

impl Display for Error {
    fn fmt(&self, _: &mut Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl core::error::Error for Error {}

/// Serializes the given data into byte vector as a JSON.
///
/// # Errors
///
/// Returns error if `T`'s `Serialize` implementation fails or `T` contains non-string map keys.
#[inline]
pub fn to_vec<T: Serialize>(v: T) -> Result<Vec<u8>> {
    let mut tmp = Vec::new();
    v.serialize(&mut Serializer {
        writer: &mut tmp,
        format: Compact,
    })?;
    Ok(tmp)
}

/// Serializes the given data into byte vector as a pretty-printed JSON.
///
/// # Errors
///
/// Returns error if `T`'s `Serialize` implementation fails or `T` contains non-string map keys.
#[inline]
pub fn to_vec_pretty<T: Serialize>(v: T) -> Result<Vec<u8>> {
    let mut tmp = Vec::new();
    v.serialize(&mut Serializer {
        writer: &mut tmp,
        format: Pretty::new(),
    })?;
    Ok(tmp)
}

/// Serializes the given data into string as a JSON.
///
/// # Errors
///
/// Returns error if `T`'s `Serialize` implementation fails or `T` contains non-string map keys.
#[inline]
pub fn to_string<T: Serialize>(v: T) -> Result<String> {
    let mut tmp = String::new();
    unsafe {
        v.serialize(&mut Serializer {
            writer: tmp.as_mut_vec(),
            format: Compact,
        })?
    }
    Ok(tmp)
}

/// Serializes the given data into string as a pretty-printed JSON.
///
/// # Errors
///
/// Returns error if `T`'s `Serialize` implementation fails or `T` contains non-string map keys.
#[inline]
pub fn to_string_pretty<T: Serialize>(v: T) -> Result<String> {
    let mut tmp = String::new();
    unsafe {
        v.serialize(&mut Serializer {
            writer: tmp.as_mut_vec(),
            format: Pretty::new(),
        })?
    }
    Ok(tmp)
}

/// Serializes the given data into the provided writer as a JSON.
///
/// # Errors
///
/// Returns error if `T`'s `Serialize` implementation fails, `T` contains
/// non-string map keys, or an I/O error occurs while writing.
#[inline]
pub fn to_writer<T: Serialize>(w: impl Write, v: T) -> Result<()> {
    v.serialize(&mut Serializer {
        writer: w,
        format: Compact,
    })
}

/// Serializes the given data into the provided writer as a pretty-printed JSON.
///
/// # Errors
///
/// Returns error if `T`'s `Serialize` implementation fails, `T` contains
/// non-string map keys, or an I/O error occurs while writing.
#[inline]
pub fn to_writer_pretty<T: Serialize>(w: impl Write, v: T) -> Result<()> {
    v.serialize(&mut Serializer {
        writer: w,
        format: Pretty::new(),
    })
}
