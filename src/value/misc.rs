macro_rules! define_value {
    (
        $(#[$meta:meta])+
        name: $name:ident $(<$name_lt:lifetime>)?,
        string: $str:ty,
        lifetime: $lt:lifetime,
        $(volatility: $volatility:tt,)?
    ) => {
        use core::fmt::{self, Debug, Formatter};
        use crate::{
            Error,
            source::Source,
            value::{Array, Number, Object, builder::ValueBuilder},
        };

        $(#[$meta])*
        #[derive(Clone, PartialEq, Eq)]
        pub enum $name $(<$name_lt>)? {
            /// Represents a JSON null value.
            Null,

            /// Represents a JSON array.
            Array(Array<Self>),

            /// Represents a JSON object.
            Object(Object<$str, Self>),

            /// Represents a JSON string.
            String($str),

            /// Represents a JSON number.
            Number(Number),

            /// Represents a JSON boolean.
            Boolean(bool),
        }

        impl<$($name_lt,)? S: Source $(< Volatility = crate::source::$volatility >)?> ValueBuilder<$lt, S> for $name $(<$name_lt>)? {
            const LAZY: bool = false;
            const CUSTOM_LITERAL: bool = false;

            type Error = Error;
            type Array = Array<Self>;
            type Object = Object<$str, Self>;
            type String = $str;

            #[inline]
            fn literal(_: &[u8]) -> Result<Self, Self::Error> {
                unimplemented!()
            }

            #[inline]
            fn integer(val: u64, neg: bool) -> Self {
                Self::Number(match neg {
                    true => Number::from_i64(val as _),
                    _ => Number::from_u64(val),
                })
            }

            #[inline]
            fn float(val: f64) -> Self {
                unsafe { Self::Number(Number::from_f64(val).unwrap_unchecked()) }
            }

            #[inline]
            fn bool(val: bool) -> Self {
                Self::Boolean(val)
            }

            #[inline]
            fn null() -> Self {
                Self::Null
            }

            #[inline]
            fn raw(_: &[u8]) -> Self {
                unimplemented!()
            }

            #[inline]
            fn apply_span(&mut self, _: usize, _: usize) {}
        }

        impl $(<$name_lt>)? From<bool> for $name $(<$name_lt>)? {
            #[inline(always)]
            fn from(val: bool) -> $name $(<$name_lt>)? {
                $name::Boolean(val)
            }
        }

        impl $(<$name_lt>)? From<Number> for $name $(<$name_lt>)? {
            #[inline(always)]
            fn from(val: Number) -> $name $(<$name_lt>)? {
                $name::Number(val)
            }
        }

        impl $(<$name_lt>)? From<$str> for $name $(<$name_lt>)? {
            #[inline(always)]
            fn from(val: $str) -> $name $(<$name_lt>)? {
                $name::String(val)
            }
        }

        impl $(<$name_lt>)? From<Array<$name $(<$name_lt>)?>> for $name $(<$name_lt>)? {
            #[inline(always)]
            fn from(val: Array<$name $(<$name_lt>)?>) -> $name $(<$name_lt>)? {
                $name::Array(val)
            }
        }

        impl $(<$name_lt>)? From<Object<$str, $name $(<$name_lt>)?>> for $name $(<$name_lt>)? {
            #[inline(always)]
            fn from(val: Object<$str, $name $(<$name_lt>)?>) -> $name $(<$name_lt>)? {
                $name::Object(val)
            }
        }

        impl $(<$name_lt>)? From<& $($name_lt)? str> for $name $(<$name_lt>)? {
            #[inline(always)]
            fn from(val: & $($name_lt)? str) -> $name $(<$name_lt>)? {
                $name::String(val.into())
            }
        }

        impl $(<$name_lt>)? Debug for $name $(<$name_lt>)? {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                match self {
                    Self::Null => f.write_str("null"),
                    Self::Array(v) => v.fmt(f),
                    Self::Boolean(v) => v.fmt(f),
                    Self::Number(v) => v.fmt(f),
                    Self::String(v) => v.fmt(f),
                    Self::Object(v) => v.fmt(f),
                }
            }
        }
    };
}

macro_rules! string_impl {
    ($type:ty) => {
        string_impl!($type, 'b);
    };

    ($type:ty, 'a) => {
        string_impl!($type, 'a, crate::source::NonVolatile);
    };

    ($type:ty, $lt:lifetime $(,$volatility:ty)?) => {
        use core::fmt::{self, Debug, Display, Formatter};

        impl<$lt, S: crate::source::Source$(< Volatility = $volatility >)?> ValueBuilder<$lt, S> for $type {
            const LAZY: bool = false;
            const CUSTOM_LITERAL: bool = true;

            type Error = Error;
            type Array = Self;
            type Object = Self;
            type String = Self;

            #[inline]
            fn literal(_: &[u8]) -> Result<Self, Self::Error> {
                unimplemented!()
            }

            #[inline]
            fn integer(_: u64, _: bool) -> Self {
                unimplemented!()
            }

            #[inline]
            fn float(_: f64) -> Self {
                unimplemented!()
            }

            #[inline]
            fn bool(_: bool) -> Self {
                unimplemented!()
            }

            #[inline]
            fn null() -> Self {
                unimplemented!()
            }

            #[inline(always)]
            fn raw(_: &[u8]) -> Self {
                unimplemented!()
            }

            #[inline]
            fn apply_span(&mut self, _: usize, _: usize) {}
        }

        impl<$lt> ArrayBuilder<Self> for $type {
            #[inline]
            fn new() -> Self {
                unimplemented!()
            }

            #[inline]
            fn with_capacity(_: usize) -> Self {
                unimplemented!()
            }

            #[inline]
            fn len(&self) -> usize {
                unimplemented!()
            }

            #[inline]
            fn on_value(&mut self, _: Self) {
                unimplemented!()
            }

            #[inline]
            fn on_complete(&mut self) {
                unimplemented!()
            }
        }

        impl<$lt, K, V> ObjectBuilder<K, V> for $type {
            #[inline]
            fn new() -> Self {
                unimplemented!()
            }

            #[inline]
            fn with_capacity(_: usize) -> Self {
                unimplemented!()
            }

            #[inline]
            fn len(&self) -> usize {
                unimplemented!()
            }

            #[inline]
            fn on_value(&mut self, _: K, _: V) {
                unimplemented!()
            }

            #[inline]
            fn on_complete(&mut self) {
                unimplemented!()
            }
        }

        impl<$lt> PartialEq for $type {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.as_str() == other.as_str()
            }
        }

        impl<$lt> Eq for $type {}

        impl<$lt> core::ops::Deref for $type {
            type Target = str;

            #[inline]
            fn deref(&self) -> &Self::Target {
                self.as_str()
            }
        }

        impl<$lt> Debug for $type {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                Debug::fmt(self.as_str(), f)
            }
        }

        impl<$lt> Display for $type {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                Display::fmt(self.as_str(), f)
            }
        }
    };
}

pub(crate) use {define_value, string_impl};
