use core::fmt::{self, Formatter};

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, Error, MapAccess, SeqAccess},
    ser::{SerializeMap, SerializeSeq},
};

use crate::value::{
    Array, Number, Object,
    borrowed::{String, Value},
    builder::{ArrayBuilder, ObjectBuilder},
};

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
                let (buf, len, cap) = v.into_raw_parts();
                Ok(Value::String(String::from_raw_parts(buf, len, cap)))
            }

            #[inline]
            fn visit_borrowed_str<E: Error>(self, v: &'de str) -> Result<Value<'de>, E> {
                Ok(Value::String(String::from_str(v)))
            }

            #[inline]
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value<'de>, A::Error> {
                let mut arr = Array::new();
                while let Some(val) = seq.next_element()? {
                    arr.on_value(val)
                }
                Ok(Value::Array(arr))
            }

            #[inline]
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value<'de>, A::Error> {
                let mut obj = Object::new();
                while let Some((key, val)) = map.next_entry()? {
                    obj.on_value(key, val);
                }
                Ok(Value::Object(obj))
            }
        }

        de.deserialize_any(Visitor)
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
                let (buf, len, cap) = v.into_raw_parts();
                Ok(String::from_raw_parts(buf, len, cap))
            }

            #[inline]
            fn visit_borrowed_str<E: Error>(self, v: &'de str) -> Result<String<'de>, E> {
                Ok(String::from_str(v))
            }
        }

        de.deserialize_str(Visitor)
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

impl Serialize for String<'_> {
    #[inline]
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(self)
    }
}
