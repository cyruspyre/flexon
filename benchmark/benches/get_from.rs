use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use flexon::jsonp;
use sonic_rs::{JsonValueTrait, pointer};
use std::{fs::read_to_string, time::Duration};

fn twitter(c: &mut Criterion) {
    let src = read_to_string("data/twitter.json").unwrap();
    let mut group = c.benchmark_group("twitter");

    group.throughput(Throughput::Bytes(src.len() as _));
    group.measurement_time(Duration::from_secs(20));

    group.bench_with_input("flexon::get_from (lazy)", src.as_str(), |b, src| {
        b.iter(|| {
            flexon::parse_at::<_, flexon::LazyValue, _>(src, ["search_metadata", "count"])
                .unwrap()
                .as_u64()
                .unwrap()
        });
    });

    group.bench_with_input(
        "flexon::get_from (lazy) (unchecked)",
        src.as_str(),
        |b, src| {
            b.iter(|| unsafe {
                flexon::Parser::from_str(src)
                    .parse_at_unchecked::<flexon::LazyValue, _>(["search_metadata", "count"])
                    .as_u64()
                    .unwrap_unchecked()
            });
        },
    );

    group.bench_with_input("flexon::get_from (serde)", src.as_str(), |b, src| {
        b.iter(|| flexon::get_from::<_, u64, _>(src, ["search_metadata", "count"]).unwrap());
    });

    group.bench_with_input("sonic_rs::get_from (lazy)", &src, |b, src| {
        b.iter(|| {
            sonic_rs::get_from_str(src, ["search_metadata", "count"])
                .unwrap()
                .as_u64()
                .unwrap()
        });
    });

    group.bench_with_input("sonic_rs::get_from (lazy) (unchecked)", &src, |b, src| {
        b.iter(|| unsafe {
            sonic_rs::get_from_str_unchecked(src, ["search_metadata", "count"])
                .unwrap_unchecked()
                .as_u64()
                .unwrap_unchecked()
        });
    });
}

fn citm_catalog(c: &mut Criterion) {
    let src = read_to_string("data/citm_catalog.json").unwrap();
    let mut group = c.benchmark_group("citm_catalog");

    group.throughput(Throughput::Bytes(src.len() as _));
    group.measurement_time(Duration::from_secs(20));

    group.bench_with_input("flexon::get_from (lazy)", src.as_str(), |b, src| {
        b.iter(|| {
            flexon::parse_at::<_, flexon::LazyValue, _>(
                src,
                jsonp!["topicSubTopics", "107888604", 0],
            )
            .unwrap()
            .as_u64()
            .unwrap()
        });
    });

    group.bench_with_input(
        "flexon::get_from (lazy) (unchecked)",
        src.as_str(),
        |b, src| unsafe {
            b.iter(|| {
                flexon::Parser::from_str(src)
                    .parse_at_unchecked::<flexon::LazyValue, _>(jsonp![
                        "topicSubTopics",
                        "107888604",
                        0
                    ])
                    .as_u64()
                    .unwrap_unchecked()
            });
        },
    );

    group.bench_with_input("flexon::get_from (serde)", src.as_str(), |b, src| {
        b.iter(|| {
            flexon::get_from::<_, u64, _>(src, jsonp!["topicSubTopics", "107888604", 0]).unwrap()
        });
    });

    group.bench_with_input("sonic_rs::get_from (lazy)", &src, |b, src| {
        b.iter(|| {
            sonic_rs::get_from_str(src, pointer!["topicSubTopics", "107888604", 0])
                .unwrap()
                .as_u64()
                .unwrap()
        });
    });

    group.bench_with_input("sonic_rs::get_from (lazy) (unchecked)", &src, |b, src| {
        b.iter(|| unsafe {
            sonic_rs::get_from_str_unchecked(src, pointer!["topicSubTopics", "107888604", 0])
                .unwrap_unchecked()
                .as_u64()
                .unwrap_unchecked()
        });
    });
}

fn canada(c: &mut Criterion) {
    let src = read_to_string("data/canada.json").unwrap();
    let mut group = c.benchmark_group("canada");

    group.throughput(Throughput::Bytes(src.len() as _));
    group.measurement_time(Duration::from_secs(20));

    group.bench_with_input("flexon::get_from (lazy)", src.as_str(), |b, src| {
        b.iter(|| {
            flexon::parse_at::<_, flexon::LazyValue, _>(
                src,
                jsonp!["features", 0, "geometry", "coordinates", 479, 5275, 0],
            )
            .unwrap()
            .as_f64()
            .unwrap()
        });
    });

    group.bench_with_input(
        "flexon::get_from (lazy) (unchecked)",
        src.as_str(),
        |b, src| {
            b.iter(|| unsafe {
                flexon::Parser::from_str(src)
                    .parse_at_unchecked::<flexon::LazyValue, _>(jsonp![
                        "features",
                        0,
                        "geometry",
                        "coordinates",
                        479,
                        5275,
                        0
                    ])
                    .as_f64()
                    .unwrap_unchecked()
            });
        },
    );

    group.bench_with_input("flexon::get_from (serde)", src.as_str(), |b, src| {
        b.iter(|| {
            flexon::get_from::<_, f64, _>(
                src,
                jsonp!["features", 0, "geometry", "coordinates", 479, 5275, 0],
            )
            .unwrap()
        });
    });

    group.bench_with_input("sonic_rs::get_from (lazy)", src.as_str(), |b, src| {
        b.iter(|| {
            sonic_rs::get_from_str(
                src,
                pointer!["features", 0, "geometry", "coordinates", 479, 5275, 0],
            )
            .unwrap()
            .as_f64()
            .unwrap()
        });
    });

    group.bench_with_input(
        "sonic_rs::get_from (lazy) (unchecked)",
        src.as_str(),
        |b, src| {
            b.iter(|| unsafe {
                sonic_rs::get_from_str_unchecked(
                    src,
                    pointer!["features", 0, "geometry", "coordinates", 479, 5275, 0],
                )
                .unwrap_unchecked()
                .as_f64()
                .unwrap_unchecked()
            });
        },
    );
}

criterion_group!(benches, twitter, citm_catalog, canada);
criterion_main!(benches);
