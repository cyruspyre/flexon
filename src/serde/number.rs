use core::fmt::{self, Formatter};

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, Error},
};

use crate::value::{Number, number::Kind};

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
