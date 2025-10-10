# flexon
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
assert!(comments.find_comment_by_line(7).unwrap());
```

## Features

`comment`: Enable comment parsing. The performance overhead is significant.

`line-count`: Include line information and allow searching comments by line index. Performance overhead is somewhat minimal.

## Performance
This was created solely for parsing JSONC with span support, so it has overhead than crates like `serde-json` or `simd-json`. If you compare this with them, you’ll see that they’re more than 2× faster, even with all the features disabled.

I can’t really justify thinking it’s slow *just* because of span support... but, but! This crate is also more than 2× faster than other crates that do similar thing, e.g. `spanned_json_parser`, `jsonc-parser`, or any other crates in general. **Source? Trust me, bro.**

Even though it can parse standard, strict JSON, you shouldn’t use it for that. Unless you need to parse JSON with comments or span support, don’t torture yourself and just use one of the faster crates mentioned earlier.