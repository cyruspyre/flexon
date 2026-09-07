#![no_main]

use flexon::{LazyValue, OwnedValue, Parser, Value, source::NullPadded};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|src: &[u8]| {
    _ = flexon::from_reader::<OwnedValue>(src);
    _ = flexon::from_source::<Value>(src);
    _ = flexon::from_source::<Value>(&mut *src.to_vec());
    _ = flexon::from_source::<Value>(&NullPadded::from_bytes(src));
    _ = flexon::from_source::<Value>(&mut NullPadded::from_bytes(src));

    _ = Parser::from_reader(src).parse::<OwnedValue>();
    _ = flexon::parse::<_, Value>(src);
    _ = flexon::parse::<_, LazyValue>(src);
    _ = flexon::parse::<_, Value>(&NullPadded::from_bytes(src));
    _ = flexon::parse::<_, LazyValue>(&NullPadded::from_bytes(src));
});
