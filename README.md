# flexon

[![crates.io](https://img.shields.io/crates/v/flexon.svg)](https://crates.io/crates/flexon)
[![Documentation](https://docs.rs/flexon/badge.svg)](https://docs.rs/flexon)

SIMD accelerated JSON parser with optional comment and span support.

## Usage

### Comments and spans

Comments are often seen in config files and such. So is the need for span information. There is not much to say about them. Here is an example.

```rust
use flexon::{Parser, Value, config::CTConfig};

let src = r#"{
  "server": {
    // listens on
    "host": "localhost",
    "port": 8080,
    /*
      Cache configuration
      TTL in seconds
    */
    "cache": {
      "enabled": true,
      "ttl": 3600,
    }
  }
}"#;
// here CTConfig means compile time configuration
// doing niche things like this adds runtime overhead and slows down
// JSON parsing for general cases. so omitting such unwanted options at compile
// time saves us from that. you may use RTConfig instead if you want.
let config = CTConfig::new().allow_trailing_comma().allow_comments();
let mut parser = Parser::from_str(src).with_config(config);
let val: Value = parser.parse().ok()?;

println!(
    "The server{} port {:?}",
    parser.take_comments()[0],
    val["server"]["port"]
);
```

### Parsing only a portion of JSON

There might be cases where you want to parse only a portion of the JSON. Say no more, you can do them quite easily. It is faster to do so than parsing the whole thing and getting the value you want. Given, you don't care about the trailing data. There are both checked and unchecked APIs for this. The former will validate the JSON as it goes forward and return early once the value has been parsed. As such the trailing data is ignored.

```rust
use flexon::jsonp;

#[derive(Deserialize)]
struct Customer {
    name: String,
    email: String,
}

let src = r#"{
  "order": {
    "id": 1001,
    "items": [
      {
        "name": "Stuff",
        "price": 29.99
      }
    ],
    "customer": {
      "name": "Walter White",
      "email": "dummy@example.com"
    }
  }
}"#;
let customer: Customer = flexon::get_from(src, ["order", "customer"])?;
let item_price: f64 = flexon::get_from(src, jsonp!["order", "items", 0, "price"])?;

println!("{} bought an item that costs {}!!", customer.name, item_price);
```

### Lazy values and raw numbers

If you need JSON values to be parsed lazily, then they are available. Nothing is parsed until they are queried/accessed (They are actually parsed and validated but not materialized). As a side effect, you can get raw numbers using them.

### Serialization

Same as other crates, nothing fancy.

## Features

`simd` (default): Enables hardware specific SIMD. Things like SWAR will still be used even if it is disabled. On `x86_64`, SSE2 is used regardless of this flag as it is a baseline feature.

`runtime-detection` (default): As of right now, it is used only for unchecked skipping APIs. Wider registers like AVX2 benefits in those cases. Can be disabled safely.

`comment`: Enables comment parsing. Follows JSONC specification.

`prealloc`: Pre-allocates object/array based on its previous length. Has no effect in serde APIs. This is pretty niche but works well when the object/array is uniform. Might become an overhead instead when using custom allocators.

`span`: Enables span information on the parsed JSON data.

`serde` (default): Implements serde specific APIs.

`nightly`: Uses nightly features. Currently only `likely_unlikely` is used.

`std` (default): Enables functionality that depends on the standard library.

`alloc` (default): Quite a lot of things depend on it, but the crate remains usable either way. When disabled, strings can only be parsed in place.

## Performance

Expect it to be more or less faster than all the other JSON parser crates out there, at least for deserialization. The benchmark section has been removed, as keeping up with crate version updates is a hassle. For third-party benchmarks, refer to [rust_serialization_benchmark](https://github.com/djkoloski/rust_serialization_benchmark).