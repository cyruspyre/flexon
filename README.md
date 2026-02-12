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

Note: serde APIs don't support span information as of right now. However, comments and configurations are available.

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

## Features

`simd` (default): Enables hardware specific SIMD. Things like SWAR will still be used even if it is disabled. On `x86_64`, SSE2 is used regardless of this flag as it is a baseline feature.

`runtime-detection` (default): As of right now, it is used only for unchecked skipping APIs. Wider registers like AVX2 benefits in those cases. Can be disabled safely.

`comment`: Enables comment parsing. Follows JSONC specification.

`prealloc` (default): Pre-allocates object/array based on its previous length. Has no effect in serde APIs. This is pretty niche but works well when the object/array is uniform. Might become an overhead instead when using custom allocators.

`span`: Enables span information on the parsed JSON data. Performance overhead is minimal. Not available in lazy value and serde APIs.

`serde` (default): Implements serde specific APIs. Serialization not available. Probably in the future.

`nightly`: Uses nightly features. Currently only `likely_unlikely` and `cold_path` are used.

# Performance

The below benchmarks are run on an `x86_64` platform with `RUSTFLAGS="-C target-cpu=native"`. Respective crate's runtime detection features are also disabled.

## Deserialize Typed

This parses JSON into Rust struct using serde API. `Cow<str>` is used for string types. Here both `simd_json` and the ones with `mut` signature in their function perform in-situ parsing. As such, they can avoid heap allocation for strings even if they contain escape characters.

`cargo bench --bench deserialize_struct`

```
twitter:
    flexon::from_mut_str                    476.42 µs   1.2345 GiB/s
    flexon::from_mut_null_padded            504.22 µs   1.1664 GiB/s
    flexon::from_str                        548.12 µs   1.0730 GiB/s
    sonic_rs::from_str                      560.91 µs   1.0486 GiB/s
    flexon::from_null_padded                574.35 µs   1.0240 GiB/s
    simd_json::from_str                     698.88 µs   861.75 MiB/s
    serde_json::from_str                    837.95 µs   718.73 MiB/s

citm_catalog:
    flexon::from_mut_null_padded            906.87 µs   1.7738 GiB/s
    flexon::from_null_padded                921.01 µs   1.7465 GiB/s
    flexon::from_mut_str                    942.81 µs   1.7062 GiB/s
    flexon::from_str                        979.58 µs   1.6421 GiB/s
    sonic_rs::from_str                      1.2714 ms   1.2652 GiB/s
    serde_json::from_str                    1.9147 ms   860.29 MiB/s
    simd_json::from_str                     1.9293 ms   853.76 MiB/s

canada:
    flexon::from_mut_null_padded            2.6452 ms   811.57 MiB/s
    flexon::from_null_padded                2.6481 ms   810.68 MiB/s
    flexon::from_str                        2.8844 ms   744.28 MiB/s
    flexon::from_mut_str                    2.9535 ms   726.86 MiB/s
    sonic_rs::from_str                      3.2038 ms   670.07 MiB/s
    serde_json::from_str                    3.6377 ms   590.15 MiB/s
    simd_json::from_str                     5.0673 ms   423.65 MiB/s

github_events:
    flexon::from_str                        60.949 µs   1.0191 GiB/s
    flexon::from_mut_str                    60.502 µs   1011.8 MiB/s
    flexon::from_null_padded                65.231 µs   952.23 MiB/s
    flexon::from_mut_null_padded            65.615 µs   946.65 MiB/s
    simd_json::from_str                     71.653 µs   866.88 MiB/s
    sonic_rs::from_str                      79.947 µs   776.95 MiB/s
    serde_json::from_str                    98.387 µs   631.33 MiB/s
```

Parsing JSON from a streaming source. `sonic_rs` and `simd_json` are excluded as they read the whole source before parsing.

`cargo bench --bench deserialize_struct "from_reader"`

```
twitter:
    flexon::from_reader                     2.1314 ms   282.57 MiB/s
    serde_json::from_reader                 3.1992 ms   188.25 MiB/s

citm_catalog:
    flexon::from_reader                     4.1157 ms   400.22 MiB/s
    serde_json::from_reader                 5.4789 ms   300.64 MiB/s

canada:
    serde_json::from_reader                 7.6590 ms   280.29 MiB/s
    flexon::from_reader                     10.440 ms   205.63 MiB/s

github_events:
    flexon::from_reader                     341.73 µs   181.76 MiB/s
    serde_json::from_reader                 362.35 µs   171.42 MiB/s
```

## Deserialize Untyped

Crate APIs that are able to parse JSON without using serde API. Note that even though `sonic_rs` uses serde API, it still does its own things in the implementation.

`cargo bench --bench deserialize_value "value"`

```
twitter:
    sonic_rs::from_slice                    356.27 µs   1.6508 GiB/s
    flexon::from_slice                      691.11 µs   871.44 MiB/s
    simd_json::to_borrowed_value            1.0280 ms   585.85 MiB/s

citm_catalog:
    sonic_rs::from_slice                    960.23 µs   1.6752 GiB/s
    flexon::from_slice                      1.6369 ms   1006.3 MiB/s
    simd_json::to_borrowed_value            3.0852 ms   533.91 MiB/s

canada:
    sonic_rs::from_slice                    2.9747 ms   721.67 MiB/s
    flexon::from_slice                      6.6070 ms   324.92 MiB/s
    simd_json::to_borrowed_value            8.4256 ms   254.79 MiB/s
```

Deserializes into JSON value using `serde_json::Value` as a common ground.

`cargo bench --bench deserialize_value "common"`

```
twitter:
    sonic_rs::from_slice                    2.0770 ms   289.96 MiB/s
    flexon::from_slice                      2.0903 ms   288.12 MiB/s
    simd_json::from_slice                   2.2492 ms   267.76 MiB/s
    serde_json::from_slice                  2.9100 ms   206.96 MiB/s

citm_catalog:
    flexon::from_slice                      2.9417 ms   559.94 MiB/s
    sonic_rs::from_slice                    3.3677 ms   489.11 MiB/s
    simd_json::from_slice                   4.3780 ms   376.24 MiB/s
    serde_json::from_slice                  4.6329 ms   355.54 MiB/s

canada:
    flexon::from_slice                      12.253 ms   175.21 MiB/s
    simd_json::from_slice                   12.990 ms   165.26 MiB/s
    sonic_rs::from_slice                    13.231 ms   162.25 MiB/s
    serde_json::from_slice                  13.417 ms   160.00 MiB/s
```

### Lazy Value

JSON values that are built lazily but still perform validation while parsing.

`cargo bench --bench deserialize_value "lazy"`

```
twitter:
    sonic_rs::from_slice                    284.59 µs   2.0667 GiB/s
    flexon::from_slice                      293.04 µs   2.0071 GiB/s
    simd_json::to_tape                      516.89 µs   1.1379 GiB/s

citm_catalog:
    sonic_rs::from_slice                    617.59 µs   2.6046 GiB/s
    flexon::from_slice                      635.55 µs   2.5310 GiB/s
    simd_json::to_tape                      1.4192 ms   1.1335 GiB/s

canada:
    sonic_rs::from_slice                    1.4288 ms   1.4673 GiB/s
    flexon::from_slice                      2.7599 ms   777.84 MiB/s
    simd_json::to_tape                      4.5479 ms   472.03 MiB/s
```

Getting items from lazy values. There was lifetime issue so `simd_json` is excluded. Here the ones with the pointer tag use their respective JSON Pointer like APIs as a one time access/query to a certain field. Both `flexon` and `sonic_rs` in this case don't cache anything. Finally the ones without the tag are performing subsequent access to nearby fields. Unlike `sonic_rs`, `flexon` will store the accessed values for later use.

`cargo bench --bench get_lazy`

```
twitter:
    sonic_rs::get_lazy (pointer)            46.510 µs   12.646 GiB/s
    flexon::get_lazy (pointer)              50.658 µs   11.610 GiB/s
    flexon::get_lazy                        104.56 µs   5.6251 GiB/s
    sonic_rs::get_lazy                      364.78 µs   1.6123 GiB/s

citm_catalog:
    sonic_rs::get_lazy (pointer)            143.37 µs   11.220 GiB/s
    flexon::get_lazy (pointer)              181.61 µs   8.8574 GiB/s
    flexon::get_lazy                        355.75 µs   4.5217 GiB/s
    sonic_rs::get_lazy                      819.43 µs   1.9631 GiB/s

canada:
    sonic_rs::get_lazy (pointer)            315.34 µs   6.6483 GiB/s
    flexon::get_lazy (pointer)              348.53 µs   6.0152 GiB/s
    flexon::get_lazy                        422.89 µs   4.9575 GiB/s
    sonic_rs::get_lazy                      6.0449 ms   355.14 MiB/s
```

## Parsing a field from JSON

These benchmarks are about partial JSON parsing, extracting a specific field without deserializing the entire document. The ones with serde tag might seem slower here. But, serde shines when parsing into structs. And its much more intuitive to do so. Note that, all of them are just parsing a single numeric field in this case as `sonic_rs` return its lazy value only.

`cargo bench --bench get_from`

```
twitter:
    sonic_rs::get_from (lazy) (unchecked)   46.056 µs   12.770 GiB/s
    flexon::get_from (lazy) (unchecked)     51.314 µs   11.462 GiB/s
    sonic_rs::get_from (lazy)               253.63 µs   2.3189 GiB/s
    flexon::get_from (lazy)                 275.10 µs   2.1379 GiB/s
    flexon::get_from (serde)                409.70 µs   1.4355 GiB/s

citm_catalog:
    sonic_rs::get_from (lazy) (unchecked)   146.69 µs   10.966 GiB/s
    flexon::get_from (lazy) (unchecked)     183.86 µs   8.7488 GiB/s
    sonic_rs::get_from (lazy)               565.91 µs   2.8425 GiB/s
    flexon::get_from (lazy)                 628.27 µs   2.5603 GiB/s
    flexon::get_from (serde)                647.04 µs   2.4861 GiB/s

canada:
    sonic_rs::get_from (lazy) (unchecked)   321.34 µs   6.5242 GiB/s
    flexon::get_from (lazy) (unchecked)     337.32 µs   6.2150 GiB/s
    sonic_rs::get_from (lazy)               1.3613 ms   1.5401 GiB/s
    flexon::get_from (serde)                2.8571 ms   751.39 MiB/s
    flexon::get_from (lazy)                 2.9049 ms   739.02 MiB/s
```
