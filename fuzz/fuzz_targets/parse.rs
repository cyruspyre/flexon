#![no_main]

use flexon::{LazyValue, OwnedValue, Parser, Value};
use flexon_fuzz::*;
use libfuzzer_sys::fuzz_target;
use serde::Deserialize;

fuzz_target!(|src: &[u8]| unsafe {
    // use of `from_(mut_)slice_unchecked` doesn't matter here.
    // cause they are no different than checking for utf-8 beforehand in the sense that all of them
    // will follow the same path after utf-8 validation and this crate doesn't rely on utf-8 guarantees anywhere.

    _ = flexon::from_reader::<_, OwnedValue>(src);
    _ = flexon::from_slice_unchecked::<Value>(src);
    _ = flexon::from_mut_slice_unchecked::<Value>(&mut src.to_vec());
    _ = Value::deserialize(&mut Parser::new(Slice(src)));
    _ = Value::deserialize(&mut Parser::new(SliceMut(&mut src.to_vec())));

    _ = Parser::new(Slice(src)).parse::<Value>();
    _ = Parser::new(Slice(src)).parse::<LazyValue>();
    _ = Parser::from_reader(src).parse::<OwnedValue>();
    _ = Parser::from_slice_unchecked(src).parse::<Value>();
    _ = Parser::from_slice_unchecked(src).parse::<LazyValue>();
});
