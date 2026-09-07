use arbitrary::Arbitrary;
use flexon::pointer::JsonPointer;
use serde::{Serialize, ser::SerializeMap};

#[derive(Arbitrary, Debug)]
pub enum AValue<'a> {
    U64(u64),
    I64(i64),
    F64(f64),
    Null(()),
    Bool(bool),
    String(&'a str),
    Array(Vec<Self>),
    Object(Vec<(&'a str, Self)>),
}

impl Serialize for AValue<'_> {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            AValue::U64(v) => s.serialize_u64(*v),
            AValue::I64(v) => s.serialize_i64(*v),
            AValue::F64(v) => s.serialize_f64(*v),
            AValue::Null(_) => s.serialize_unit(),
            AValue::Bool(v) => s.serialize_bool(*v),
            AValue::String(v) => s.serialize_str(v),
            AValue::Array(v) => v.serialize(s),
            AValue::Object(v) => {
                let mut map = s.serialize_map(Some(v.len()))?;
                for (k, v) in v {
                    map.serialize_entry(k, v)?
                }
                map.end()
            }
        }
    }
}

#[derive(Arbitrary, Debug)]
pub enum Pointer<'a> {
    Index(usize),
    Key(&'a str),
}

impl JsonPointer for &Pointer<'_> {
    #[inline]
    fn as_key(&self) -> Option<&str> {
        match self {
            Pointer::Key(v) => Some(v),
            _ => None,
        }
    }

    #[inline]
    fn as_index(&self) -> Option<usize> {
        match self {
            Pointer::Index(v) => Some(*v),
            _ => None,
        }
    }
}
