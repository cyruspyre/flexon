#![no_main]

use flexon::{LazyValue, Parser, Value, pointer::JsonPointer};
use libfuzzer_sys::{arbitrary::Arbitrary, fuzz_target};

#[derive(Arbitrary, Debug)]
struct Input<'a> {
    src: &'a [u8],
    path: Vec<Pointer<'a>>,
}

#[derive(Arbitrary, Debug)]
enum Pointer<'a> {
    Index(usize),
    Key(&'a [u8]),
}

impl JsonPointer for &Pointer<'_> {
    #[inline]
    fn as_key(&self) -> Option<&str> {
        match self {
            Pointer::Key(v) => unsafe { Some(str::from_utf8_unchecked(v)) },
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

fuzz_target!(|data: Input| {
    _ = Parser::from_slice(data.src).parse_at::<Value, _>(&data.path);
    _ = Parser::from_slice(data.src).parse_at::<LazyValue, _>(&data.path);
});
