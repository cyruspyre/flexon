# flexon [![crates.io](https://img.shields.io/crates/v/flexon.svg)](https://crates.io/crates/flexon)
A JSON parser with span tracking and optional comment support.

## Example
For most use cases, all you need is the actual JSON data:
```rs
use flexon::parse;

let src = r#"{"blah": "was it really necessary?"}"#;
let val: &Span<Value> = parse(src).unwrap()["blah"];

println!("{val:#?} at {}..={}", val.start(), val.end());
```
But wait, what about comments? For that you need to enable `comment` feature
```rs
use flexon::Parser;

let src = r#"
{
    // A single-line comment
    "nested": {
        "one": "hello world"
    }
    /*
     * A multi-line comment
     */
    "mixed": [true, null, 1]
}
"#;

let parser = Parser::new(
    src,
    false, // Require commas
    false, // Allow trailing commas (has no effect when commas are optional)
);
let (_, comments) = parser.parse().unwrap();
let (cmnt, is_multi_line) = comments[0].data();

println!("{cmnt:?}");
assert!(!is_multi_line);

let (cmnt, is_multi_line) = comments[1].data();

println!("{cmnt:?}");
assert!(is_multi_line);

// Index 11 falls within the single-line comment's range.
assert!(comments.find_comment(11).is_some());

// With the `line-count` feature enabled, you can find a comment by its line index.
// In that case, the parser returns `Metadata` instead of `Vec<..>`.
assert!(comments.find_comment_by_line(7).is_some());
```

## Features

`comment`: Enable comment parsing. The performance overhead is significant.

`line-count`: Include line information and allow searching comments by line index. Performance overhead is somewhat minimal.

`prealloc` (default): Pre-allocate memory for array/object when parsing based on the length of previously parsed array/object. Can improve performance at the cost of potentially increased/reduced memory usage. Works particularly well when the JSON has a relatively uniform and repetitive structure.

`span`: Include span information on the parsed JSON data. Performance overhead is minimal and memory usage will increase roughly by 33%.

## Performance
This was created solely for parsing JSON with span support and comments, so it has overhead than other crates like [`serde-json`](https://crates.io/crates/serde_json), [`jzon`](https://crates.io/crates/jzon) or [`simd-json`](https://crates.io/crates/simd-json). The performance is somewhat close to [`serde-json`](https://crates.io/crates/serde_json) or sometimes even better, depending on the case. For reference, here are their benchmark on x86_64:
```
serde-json:
  canada        14.36 ms  149.42 MiB/s
  twitter        2.46 ms  244.20 MiB/s
  citm_catalog   4.46 ms  369.00 MiB/s

flexon:
  canada         9.87 ms  217.50 MiB/s
  twitter        2.75 ms  218.69 MiB/s
  citm_catalog   4.24 ms  388.56 MiB/s

flexon (without span):
  canada         9.31 ms  230.50 MiB/s
  twitter        2.60 ms  230.99 MiB/s
  citm_catalog   4.16 ms  396.08 MiB/s
```
Even though it can parse standard, strict JSON, you shouldn’t use it for that unless you need to parse JSON with comments or span support. Don’t torture yourself and just use one of the faster crates mentioned earlier. Fyi, this crate is faster than others that serve a somewhat similar purpose. *Source?* **Trust me, bro.**

Other similar crates: [`spanned-json-parser`](https://crates.io/crates/spanned_json_parser) | [`jsonc-parser`](https://crates.io/crates/jsonc-parser)
