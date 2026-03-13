use crate::{Error, source::Source, value::builder::*};

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

impl<'a, K, V> ObjectBuilder<'a, K, V> for _Object {
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

impl<S: Source> StringBuilder<'_, S, Error> for _String {
    const REJECT_CTRL_CHAR: bool = true;
    const REJECT_INVALID_ESCAPE: bool = true;

    #[inline]
    fn new() -> Self {
        Self
    }

    #[inline]
    fn on_escape(&mut self, _: &[u8]) {}

    #[inline]
    fn on_chunk(&mut self, _: &[u8]) {}

    #[inline]
    fn on_final_chunk(&mut self, _: &[u8]) {}

    #[inline]
    fn apply_span(&mut self, _: usize, _: usize) {}

    #[inline]
    fn on_complete(&mut self, _: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}
