#![no_main]

use core::str::{from_utf8_unchecked, from_utf8_unchecked_mut};
use flexon::{LazyValue, OwnedValue, Parser, Value, source::Reader};
use flexon_fuzz::*;
use libfuzzer_sys::{arbitrary::Arbitrary, fuzz_target};

#[derive(Arbitrary, Debug)]
struct Input<'a> {
    src: &'a [u8],
    path: Vec<Pointer<'a>>,
}

fuzz_target!(|data: Input| unsafe {
    // use of `from_slice_unchecked` and `from_utf8_unchecked*` doesn't matter here.
    // cause they are no different than checking for utf-8 beforehand in the sense that all of them
    // will follow the same path after utf-8 validation and this crate doesn't rely on utf-8 guarantees anywhere.

    _ = flexon::get_from::<_, OwnedValue, _>(Reader::new(data.src), &data.path);
    _ = flexon::get_from::<_, Value, _>(Slice(data.src), &data.path);
    _ = flexon::get_from::<_, Value, _>(SliceMut(&mut data.src.to_vec()), &data.path);
    _ = flexon::get_from::<_, Value, _>(from_utf8_unchecked(data.src), &data.path);
    _ = flexon::get_from::<_, Value, _>(
        from_utf8_unchecked_mut(&mut data.src.to_vec()),
        &data.path,
    );

    _ = Parser::new(Slice(data.src)).parse_at::<Value, _>(&data.path);
    _ = Parser::new(Slice(data.src)).parse_at::<LazyValue, _>(&data.path);
    _ = Parser::from_reader(data.src).parse_at::<OwnedValue, _>(&data.path);
    _ = Parser::from_slice_unchecked(data.src).parse_at::<Value, _>(&data.path);
    _ = Parser::from_slice_unchecked(data.src).parse_at::<LazyValue, _>(&data.path);
});
