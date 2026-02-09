use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use std::{fs::read_to_string, time::Duration};

macro_rules! bench {
    ($($name:ident),* $(,)?) => {
        $(
            fn $name(c: &mut Criterion) {
                let path = format!("data/{}.json", stringify!($name));
                let src = read_to_string(&path).unwrap();
                let mut group = c.benchmark_group(stringify!($name));

                group.throughput(Throughput::Bytes(src.len() as _));
                group.measurement_time(Duration::from_secs(20));

                group.bench_with_input("flexon::from_slice (lazy)", &src, |b, src| {
                    b.iter_batched(
                        || src,
                        |src| {
                            flexon::Parser::from_slice(src.as_bytes()).parse::<flexon::LazyValue>().unwrap();
                        },
                        BatchSize::SmallInput
                    )
                });

                group.bench_with_input("flexon::from_slice (value)", &src, |b, src| {
                    b.iter_batched(
                        || src,
                        |src| {
                            flexon::Parser::from_slice(src.as_bytes()).parse::<flexon::Value>().unwrap();
                        },
                        BatchSize::SmallInput
                    )
                });

                group.bench_with_input("flexon::from_slice (common)", &src, |b, src| {
                    b.iter_batched(
                        || src,
                        |src| {
                            flexon::from_slice::<serde_json::Value>(src.as_bytes()).unwrap();
                        },
                        BatchSize::SmallInput
                    )
                });

                group.bench_with_input("sonic_rs::from_slice (lazy)", &src, |b, src| {
                    b.iter_batched(
                        || src,
                        |src| {
                            sonic_rs::from_slice::<sonic_rs::LazyValue>(src.as_bytes()).unwrap();
                        },
                        BatchSize::SmallInput
                    )
                });

                group.bench_with_input("sonic_rs::from_slice (value)", &src, |b, src| {
                    b.iter_batched(
                        || src,
                        |src| {
                            sonic_rs::from_slice::<sonic_rs::Value>(src.as_bytes()).unwrap();
                        },
                        BatchSize::SmallInput
                    )
                });

                group.bench_with_input("sonic_rs::from_slice (common)", &src, |b, src| {
                    b.iter_batched(
                        || src,
                        |src| {
                            sonic_rs::from_slice::<serde_json::Value>(src.as_bytes()).unwrap();
                        },
                        BatchSize::SmallInput
                    )
                });

                group.bench_with_input("serde_json::from_slice (common)", &src, |b, src| {
                    b.iter_batched(
                        || src,
                        |src| {
                            serde_json::from_slice::<serde_json::Value>(src.as_bytes()).unwrap();
                        },
                        BatchSize::SmallInput
                    )
                });

                group.bench_with_input("simd_json::to_tape (lazy)", &src, |b, src| {
                    b.iter_batched(
                        || src.clone(),
                        |mut s| unsafe {
                            simd_json::to_tape(s.as_bytes_mut()).unwrap();
                        },
                        BatchSize::SmallInput
                    )
                });

                group.bench_with_input("simd_json::to_borrowed_value (value)", &src, |b, src| {
                    b.iter_batched(
                        || src.clone(),
                        |mut s| unsafe {
                            simd_json::to_borrowed_value(s.as_bytes_mut()).unwrap();
                        },
                        BatchSize::SmallInput
                    )
                });

                group.bench_with_input("simd_json::from_slice (common)", &src, |b, src| {
                    b.iter_batched(
                        || src.clone(),
                        |mut src| unsafe {
                            simd_json::from_slice::<serde_json::Value>(src.as_bytes_mut()).unwrap();
                        },
                        BatchSize::SmallInput
                    )
                });
            }
        )*

        criterion_group!(benches, $($name),*);
    }
}

bench! {
    twitter,
    citm_catalog,
    canada,
    // github_events,
}

criterion_main!(benches);
