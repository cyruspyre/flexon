use core::fmt::{self, Formatter};

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, Error, MapAccess, SeqAccess},
    ser::{SerializeMap, SerializeSeq},
};

use crate::misc::string_into_raw_parts;
use crate::value::{
    Array, Number, Object, OwnedValue,
    borrowed::{String, Value},
    builder::{ArrayBuilder, ObjectBuilder},
    number::Kind,
    owned,
};

#[cfg(feature = "span")]
use {crate::span::GenericValue, core::ops::Deref};

impl<'de> Deserialize<'de> for Value<'de> {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Value<'de>;

            fn expecting(&self, f: &mut Formatter) -> fmt::Result {
                f.write_str("a valid JSON value")
            }

            #[inline]
            fn visit_bool<E: Error>(self, v: bool) -> Result<Value<'de>, E> {
                Ok(Value::Boolean(v))
            }

            #[inline]
            fn visit_some<D: Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Value<'de>, D::Error> {
                Deserialize::deserialize(deserializer)
            }

            #[inline]
            fn visit_none<E: Error>(self) -> Result<Value<'de>, E> {
                Ok(Value::Null)
            }

            #[inline]
            fn visit_unit<E: Error>(self) -> Result<Value<'de>, E> {
                Ok(Value::Null)
            }

            #[inline]
            fn visit_u64<E: Error>(self, v: u64) -> Result<Value<'de>, E> {
                Ok(Value::Number(Number::from_u64(v)))
            }

            #[inline]
            fn visit_i64<E: Error>(self, v: i64) -> Result<Value<'de>, E> {
                Ok(Value::Number(Number::from_i64(v)))
            }

            #[inline]
            fn visit_f64<E: Error>(self, v: f64) -> Result<Value<'de>, E> {
                Ok(Number::from_f64(v).map_or(Value::Null, Value::Number))
            }

            #[inline]
            fn visit_str<E: Error>(self, v: &str) -> Result<Value<'de>, E> {
                self.visit_string(v.into())
            }

            #[inline]
            fn visit_string<E: Error>(self, v: std::string::String) -> Result<Value<'de>, E> {
                Ok(Value::String(v.into()))
            }

            #[inline]
            fn visit_borrowed_str<E: Error>(self, v: &'de str) -> Result<Value<'de>, E> {
                Ok(v.into())
            }

            #[inline]
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value<'de>, A::Error> {
                let mut arr = Array::new();
                while let Some(val) = seq.next_element()? {
                    arr.on_value(val)
                }
                Ok(arr.into())
            }

            #[inline]
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value<'de>, A::Error> {
                let mut obj = Object::new();
                while let Some((key, val)) = map.next_entry()? {
                    obj.on_value(key, val);
                }
                Ok(obj.into())
            }
        }

        de.deserialize_any(Visitor)
    }
}

impl<'de> Deserialize<'de> for OwnedValue {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = OwnedValue;

            fn expecting(&self, f: &mut Formatter) -> fmt::Result {
                f.write_str("a valid JSON value")
            }

            #[inline]
            fn visit_bool<E: Error>(self, v: bool) -> Result<OwnedValue, E> {
                Ok(OwnedValue::Boolean(v))
            }

            #[inline]
            fn visit_some<D: Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<OwnedValue, D::Error> {
                Deserialize::deserialize(deserializer)
            }

            #[inline]
            fn visit_none<E: Error>(self) -> Result<OwnedValue, E> {
                Ok(OwnedValue::Null)
            }

            #[inline]
            fn visit_unit<E: Error>(self) -> Result<OwnedValue, E> {
                Ok(OwnedValue::Null)
            }

            #[inline]
            fn visit_u64<E: Error>(self, v: u64) -> Result<OwnedValue, E> {
                Ok(OwnedValue::Number(Number::from_u64(v)))
            }

            #[inline]
            fn visit_i64<E: Error>(self, v: i64) -> Result<OwnedValue, E> {
                Ok(OwnedValue::Number(Number::from_i64(v)))
            }

            #[inline]
            fn visit_f64<E: Error>(self, v: f64) -> Result<OwnedValue, E> {
                Ok(Number::from_f64(v).map_or(OwnedValue::Null, OwnedValue::Number))
            }

            #[inline]
            fn visit_str<E: Error>(self, v: &str) -> Result<OwnedValue, E> {
                Ok(v.into())
            }

            #[inline]
            fn visit_string<E: Error>(self, v: std::string::String) -> Result<OwnedValue, E> {
                Ok(OwnedValue::String(v.into()))
            }

            #[inline]
            fn visit_borrowed_str<E: Error>(self, v: &'de str) -> Result<OwnedValue, E> {
                Ok(v.into())
            }

            #[inline]
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<OwnedValue, A::Error> {
                let mut arr = Array::new();
                while let Some(val) = seq.next_element()? {
                    arr.on_value(val)
                }
                Ok(arr.into())
            }

            #[inline]
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<OwnedValue, A::Error> {
                let mut obj = Object::new();
                while let Some((key, val)) = map.next_entry()? {
                    obj.on_value(key, val);
                }
                Ok(obj.into())
            }
        }

        de.deserialize_any(Visitor)
    }
}

#[cfg(feature = "span")]
impl<'de, S: Deserialize<'de>> Deserialize<'de> for GenericValue<S> {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        use core::marker::PhantomData;
        use serde::de::IntoDeserializer;

        struct Visitor<S>(PhantomData<S>);

        impl<'de, S: Deserialize<'de>> de::Visitor<'de> for Visitor<S> {
            type Value = GenericValue<S>;

            fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.write_str("a valid JSON value")
            }

            #[inline]
            fn visit_bool<E: Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(GenericValue::Boolean(v))
            }

            #[inline]
            fn visit_none<E: Error>(self) -> Result<Self::Value, E> {
                Ok(GenericValue::Null)
            }

            #[inline]
            fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
                Ok(GenericValue::Null)
            }

            #[inline]
            fn visit_some<D: Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                Deserialize::deserialize(deserializer)
            }

            #[inline]
            fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(GenericValue::Number(Number::from_u64(v)))
            }

            #[inline]
            fn visit_i64<E: Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(GenericValue::Number(Number::from_i64(v)))
            }

            #[inline]
            fn visit_f64<E: Error>(self, v: f64) -> Result<Self::Value, E> {
                Ok(Number::from_f64(v).map_or(GenericValue::Null, GenericValue::Number))
            }

            #[inline]
            fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                S::deserialize(v.into_deserializer()).map(GenericValue::String)
            }

            #[inline]
            fn visit_string<E: Error>(self, v: std::string::String) -> Result<Self::Value, E> {
                S::deserialize(v.into_deserializer()).map(GenericValue::String)
            }

            #[inline]
            fn visit_borrowed_str<E: Error>(self, v: &'de str) -> Result<Self::Value, E> {
                S::deserialize(de::value::BorrowedStrDeserializer::new(v)).map(GenericValue::String)
            }

            #[inline]
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut arr = Array::new();
                while let Some(val) = seq.next_element()? {
                    arr.on_value(val);
                }
                Ok(GenericValue::Array(arr))
            }

            #[inline]
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut obj = Object::new();
                while let Some((k, v)) = map.next_entry()? {
                    obj.on_value(k, v);
                }
                Ok(GenericValue::Object(obj))
            }
        }

        de.deserialize_any(Visitor(PhantomData))
    }
}

impl<'de> Deserialize<'de> for String<'de> {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = String<'de>;

            fn expecting(&self, f: &mut Formatter) -> fmt::Result {
                f.write_str("a string")
            }

            #[inline]
            fn visit_str<E: Error>(self, v: &str) -> Result<String<'de>, E> {
                self.visit_string(v.into())
            }

            #[inline]
            fn visit_string<E: Error>(self, v: std::string::String) -> Result<String<'de>, E> {
                let (buf, len, cap) = string_into_raw_parts(v);
                Ok(String::from_raw_parts(buf, len, cap))
            }

            #[inline]
            fn visit_borrowed_str<E: Error>(self, v: &'de str) -> Result<String<'de>, E> {
                Ok(String::from(v))
            }
        }

        de.deserialize_str(Visitor)
    }
}

impl<'de> Deserialize<'de> for owned::String {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = owned::String;

            fn expecting(&self, f: &mut Formatter) -> fmt::Result {
                f.write_str("a string")
            }

            #[inline]
            fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(v.into())
            }

            #[inline]
            fn visit_string<E: Error>(self, v: std::string::String) -> Result<Self::Value, E> {
                Ok(v.into())
            }

            #[inline]
            fn visit_borrowed_str<E: Error>(self, v: &'de str) -> Result<Self::Value, E> {
                Ok(v.into())
            }
        }

        de.deserialize_str(Visitor)
    }
}

impl<'de> Deserialize<'de> for Number {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Number;

            fn expecting(&self, f: &mut Formatter) -> fmt::Result {
                f.write_str("a JSON number")
            }

            #[inline]
            fn visit_u64<E: Error>(self, v: u64) -> Result<Number, E> {
                Ok(Number::from_u64(v))
            }

            #[inline]
            fn visit_i64<E: Error>(self, v: i64) -> Result<Number, E> {
                Ok(Number::from_i64(v))
            }

            #[inline]
            fn visit_f64<E: Error>(self, v: f64) -> Result<Number, E> {
                match Number::from_f64(v) {
                    Some(v) => Ok(v),
                    _ => Err(Error::custom("not a JSON number")),
                }
            }
        }

        de.deserialize_any(Visitor)
    }
}

impl Serialize for Value<'_> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        match self {
            Value::Null => ser.serialize_unit(),
            Value::Boolean(v) => ser.serialize_bool(*v),
            Value::Number(v) => v.serialize(ser),
            Value::String(v) => ser.serialize_str(v),
            Value::Array(v) => {
                let mut ser = ser.serialize_seq(Some(v.len()))?;
                for v in v.iter() {
                    ser.serialize_element(v)?
                }
                ser.end()
            }
            Value::Object(v) => {
                let mut ser = ser.serialize_map(Some(v.len()))?;
                for (k, v) in v.as_slice() {
                    ser.serialize_entry(k, v)?
                }
                ser.end()
            }
        }
    }
}

impl Serialize for OwnedValue {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        type Value = OwnedValue;

        match self {
            Value::Null => ser.serialize_unit(),
            Value::Boolean(v) => ser.serialize_bool(*v),
            Value::Number(v) => v.serialize(ser),
            Value::String(v) => ser.serialize_str(v),
            Value::Array(v) => {
                let mut ser = ser.serialize_seq(Some(v.len()))?;
                for v in v.iter() {
                    ser.serialize_element(v)?
                }
                ser.end()
            }
            Value::Object(v) => {
                let mut ser = ser.serialize_map(Some(v.len()))?;
                for (k, v) in v.as_slice() {
                    ser.serialize_entry(k, v)?
                }
                ser.end()
            }
        }
    }
}

#[cfg(feature = "span")]
impl<S> Serialize for GenericValue<S>
where
    S: Serialize + Deref<Target = str>,
{
    fn serialize<Ser: Serializer>(&self, ser: Ser) -> Result<Ser::Ok, Ser::Error> {
        match self {
            GenericValue::Null => ser.serialize_unit(),
            GenericValue::Boolean(v) => ser.serialize_bool(*v),
            GenericValue::Number(v) => v.serialize(ser),
            GenericValue::String(v) => ser.serialize_str(v),
            GenericValue::Array(v) => {
                let mut seq = ser.serialize_seq(Some(v.len()))?;
                for item in v.iter() {
                    seq.serialize_element(item.data())?;
                }
                seq.end()
            }
            GenericValue::Object(v) => {
                let mut map = ser.serialize_map(Some(v.len()))?;
                for (k, v) in v.as_slice() {
                    map.serialize_entry(k.data().deref(), v.data())?;
                }
                map.end()
            }
        }
    }
}

impl Serialize for String<'_> {
    #[inline]
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(self)
    }
}

impl Serialize for owned::String {
    #[inline]
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(self)
    }
}

impl Serialize for Number {
    #[inline]
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Kind::Unsigned(v) => ser.serialize_u64(v),
            Kind::Signed(v) => ser.serialize_i64(v),
            Kind::Float(v) => ser.serialize_f64(v),
        }
    }
}
