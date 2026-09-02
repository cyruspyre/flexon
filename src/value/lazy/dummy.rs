use crate::{
    Error,
    source::{NonVolatile, Source},
    value::{
        builder::*,
        lazy::{Raw, Value},
    },
};

pub struct _Array;
pub struct _Object;
pub struct _String;

impl<V> ArrayBuilder<V> for _Array {
    #[inline]
    fn new() -> Self {
        Self
    }

    #[inline]
    fn with_capacity(_: usize) -> Self {
        Self
    }

    #[inline]
    fn len(&self) -> usize {
        0
    }

    #[inline]
    fn on_value(&mut self, _: V) {}

    #[inline]
    fn on_complete(&mut self) {}
}

impl<K, V> ObjectBuilder<K, V> for _Object {
    #[inline]
    fn new() -> Self {
        Self
    }

    #[inline]
    fn with_capacity(_: usize) -> Self {
        Self
    }

    #[inline]
    fn len(&self) -> usize {
        0
    }

    #[inline]
    fn on_value(&mut self, _: K, _: V) {}

    #[inline]
    fn on_complete(&mut self) {}
}

impl<S: Source> StringBuilder<'_, S> for _String {
    #[inline]
    fn new() -> Self {
        Self
    }

    #[inline]
    unsafe fn on_escape(&mut self, _: &[u8]) {}

    #[inline]
    unsafe fn on_chunk(&mut self, _: &[u8]) {}

    #[inline]
    unsafe fn on_final_chunk(&mut self, _: &[u8]) {}

    #[inline]
    fn apply_span(&mut self, _: usize, _: usize) {}
}

impl<'a, S: Source<Volatility = NonVolatile>> ValueBuilder<'a, S> for Value<'a> {
    const LAZY: bool = true;

    type Error = Error;
    type Array = _Array;
    type Object = _Object;
    type String = _String;

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

    #[inline]
    fn raw(s: &'a [u8]) -> Self {
        Self::Raw(Raw(unsafe { str::from_utf8_unchecked(s) }))
    }

    #[inline]
    fn apply_span(&mut self, _: usize, _: usize) {}
}

impl<'a> Into<Value<'a>> for _Array {
    #[inline]
    fn into(self) -> Value<'a> {
        unimplemented!()
    }
}

impl<'a> Into<Value<'a>> for _Object {
    #[inline]
    fn into(self) -> Value<'a> {
        unimplemented!()
    }
}

impl<'a> Into<Value<'a>> for _String {
    #[inline]
    fn into(self) -> Value<'a> {
        unimplemented!()
    }
}
