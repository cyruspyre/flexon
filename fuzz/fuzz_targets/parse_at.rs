#![no_main]

use flexon::{LazyValue, OwnedValue, Parser, Value, source::Reader};
use flexon_fuzz::Pointer;
use libfuzzer_sys::{arbitrary::Arbitrary, fuzz_target};

#[derive(Arbitrary, Debug)]
struct Input<'a> {
    src: &'a [u8],
    path: Vec<Pointer<'a>>,
}

fuzz_target!(|data: Input| {
    _ = flexon::get_from::<OwnedValue, _>(Reader::new(data.src), &data.path);
    _ = flexon::get_from::<Value, _>(data.src, &data.path);
    _ = flexon::get_from::<Value, _>(&mut *data.src.to_vec(), &data.path);

    _ = Parser::new(data.src).parse_at::<Value, _>(&data.path);
    _ = Parser::new(data.src).parse_at::<LazyValue, _>(&data.path);
    _ = Parser::from_reader(data.src).parse_at::<OwnedValue, _>(&data.path);
});
