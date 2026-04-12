#![no_main]

use flexon::{
    Parser,
    source::{NullPadded, Reader},
};
use flexon_fuzz::AValue;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: AValue| unsafe {
    let s0 = flexon::to_string(data).unwrap_unchecked();
    let s1 = NullPadded::from_str(&s0);
    let s2 = Reader::new_unchecked(s0.as_bytes());

    let r0 = Parser::from_str(&s0).parse_unchecked::<flexon::Value>();
    let r1 = Parser::new(&s1).parse_unchecked::<flexon::Value>();
    let r2 = Parser::new(s2).parse_unchecked::<flexon::OwnedValue>();

    assert_eq!(flexon::to_string(&r0).unwrap_unchecked(), s0);
    assert_eq!(r0, r1);
    assert_eq!(r1, r2);
});
