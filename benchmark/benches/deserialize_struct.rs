use benchmark::*;
use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use serde::Deserialize;
use std::{
    fs::{File, read_to_string},
    io::BufReader,
    time::Duration,
};

macro_rules! bench {
    ($($name:ident: $type:ident),* $(,)?) => {
        $(
            fn $name(c: &mut Criterion) {
                let path = format!("data/{}.json", stringify!($name));
                let src = read_to_string(&path).unwrap();
                let mut group = c.benchmark_group(stringify!($name));

                group.throughput(Throughput::Bytes(src.len() as _));
                group.measurement_time(Duration::from_secs(20));

                group.bench_with_input("flexon::from_mut_null_padded", &src, |b, src| unsafe {
                    let buf = core::cell::Cell::new(flexon::source::NullPadded::from_str(src));

                    b.iter_batched(
                        || (*buf.as_ptr()).write_str(src),
                        |_| {
                            flexon::from_mut_null_padded::<$type>(&mut *buf.as_ptr()).unwrap();
                        },
                        BatchSize::PerIteration
                    )
                });

                group.bench_with_input("flexon::from_null_padded", &src, |b, src| {
                    let src = flexon::source::NullPadded::from_str(src);
                    b.iter_batched(
                        || &src,
                        |src| {
                            flexon::from_null_padded::<$type>(src).unwrap();
                        },
                        BatchSize::SmallInput
                    )
                });

                group.bench_with_input("flexon::from_mut_str", &src, |b, src| {
                    b.iter_batched_ref(
                        || src.clone(),
                        |src| unsafe {
                            flexon::from_mut_str::<$type>(src).unwrap();
                        },
                        BatchSize::SmallInput
                    )
                });

                group.bench_with_input("flexon::from_str", &src, |b, src| {
                    b.iter_batched(
                        || src,
                        |src| {
                            flexon::from_str::<$type>(src).unwrap();
                        },
                        BatchSize::SmallInput
                    )
                });

                group.bench_with_input("sonic_rs::from_str", &src, |b, src| {
                    b.iter_batched(
                        || src,
                        |src| {
                            sonic_rs::from_str::<$type>(src).unwrap();
                        },
                        BatchSize::SmallInput
                    )
                });

                group.bench_with_input("serde_json::from_str", &src, |b, src| {
                    b.iter_batched(
                        || src,
                        |src| {
                            serde_json::from_str::<$type>(src).unwrap();
                        },
                        BatchSize::SmallInput
                    )
                });

                group.bench_with_input("simd_json::from_str", &src, |b, src| {
                    b.iter_batched(
                        || src.clone(),
                        |mut src| unsafe {
                            simd_json::from_str::<$type>(&mut src).unwrap();
                        },
                        BatchSize::SmallInput
                    )
                });

                group.bench_function("flexon::from_reader_unchecked", |b| {
                    b.iter_batched(
                        || BufReader::new(File::open(&path).unwrap()),
                        |rdr| unsafe {
                            $type::deserialize(&mut flexon::Parser::from_reader_unchecked(rdr)).unwrap();
                        },
                        BatchSize::SmallInput
                    )
                });

                group.bench_function("flexon::from_reader", |b| {
                    b.iter_batched(
                        || BufReader::new(File::open(&path).unwrap()),
                        |rdr| {
                            $type::deserialize(&mut flexon::Parser::from_reader(rdr)).unwrap();
                        },
                        BatchSize::SmallInput
                    )
                });

                group.bench_function("serde_json::from_reader", |b| {
                    b.iter_batched(
                        || BufReader::new(File::open(&path).unwrap()),
                        |rdr| {
                            $type::deserialize(&mut serde_json::Deserializer::from_reader(rdr)).unwrap();
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
    twitter: Twitter,
    citm_catalog: CitmCatalog,
    canada: Canada,
    // github_events: GithubEvents,
}

criterion_main!(benches);
