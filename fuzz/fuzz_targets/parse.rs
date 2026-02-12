#![no_main]

use flexon::{LazyValue, Parser, Value};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    _ = Parser::from_slice(data).parse::<Value>();
    _ = Parser::from_slice(data).parse::<LazyValue>();
});
