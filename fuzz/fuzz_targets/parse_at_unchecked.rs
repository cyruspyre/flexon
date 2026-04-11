#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use flexon::{Parser, source::NullPadded};
use flexon_fuzz::*;
use libfuzzer_sys::fuzz_target;

#[derive(Debug)]
struct Input<'a> {
    val: AValue<'a>,
    path: Vec<Pointer<'a>>,
}

impl<'a> Arbitrary<'a> for Input<'a> {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self, arbitrary::Error> {
        let mut root = AValue::arbitrary(u)?;
        let mut cur = &mut root;
        let mut path = Vec::new();

        loop {
            match cur {
                AValue::Array(v) => unsafe {
                    let idx = u.choose_index(v.len())?;
                    path.push(Pointer::Index(idx));
                    cur = v.get_unchecked_mut(idx);
                },
                AValue::Object(v) => unsafe {
                    let mut i = 0;
                    while i < v.len() {
                        if v[..i].iter().any(|u| u.0 == v[i].0) {
                            v.swap_remove(i);
                        } else {
                            i += 1;
                        }
                    }

                    let idx = u.choose_index(v.len())?;
                    let (k, v) = v.get_unchecked_mut(idx);
                    path.push(Pointer::Key(k.as_bytes()));
                    cur = v;
                },
                _ => return Ok(Input { val: root, path }),
            }
        }
    }
}

fuzz_target!(|data: Input| unsafe {
    let s0 = flexon::to_string(data.val).unwrap_unchecked();
    let mut s1 = NullPadded::from_str(&s0);

    _ = Parser::new(&*s0).parse_at_unchecked::<flexon::Value, _>(&data.path);
    _ = Parser::new(&s1).parse_at_unchecked::<flexon::Value, _>(&data.path);

    _ = flexon::get_from_unchecked::<_, flexon::Value, _>(&*s0, &data.path);
    _ = flexon::get_from_unchecked::<_, flexon::Value, _>(&mut *s0.clone(), &data.path);
    _ = flexon::get_from_unchecked::<_, flexon::Value, _>(&s1, &data.path);
    _ = flexon::get_from_unchecked::<_, flexon::Value, _>(&mut s1, &data.path);
});
