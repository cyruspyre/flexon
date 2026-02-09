use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use flexon::jsonp;
use sonic_rs::{JsonValueTrait, pointer};
use std::{fs::read_to_string, time::Duration};

fn twitter(c: &mut Criterion) {
    let src = read_to_string("data/twitter.json").unwrap();
    let mut group = c.benchmark_group("twitter");

    group.throughput(Throughput::Bytes(src.len() as _));
    group.measurement_time(Duration::from_secs(20));

    group.bench_with_input("flexon::get_lazy (pointer)", src.as_str(), |b, src| {
        b.iter_batched(
            || flexon::parse::<_, flexon::LazyValue>(src).unwrap(),
            |v| {
                v.pointer(jsonp!["search_metadata", "count"])
                    .unwrap()
                    .as_u64()
                    .unwrap()
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_with_input("flexon::get_lazy", src.as_str(), |b, src| {
        b.iter_batched(
            || flexon::parse::<_, flexon::LazyValue>(src).unwrap(),
            |mut v| {
                let tmp = v["search_metadata"].as_object().unwrap();
                let one = tmp["max_id"].as_u64().unwrap();
                let two = tmp["count"].as_u64().unwrap();

                one + two + v["statuses"].as_array().unwrap().actual_len() as u64
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_with_input("sonic_rs::get_lazy (pointer)", &src, |b, src| {
        b.iter_batched(
            || sonic_rs::from_str::<sonic_rs::LazyValue>(src).unwrap(),
            |v| {
                v.pointer(pointer!["search_metadata", "count"])
                    .unwrap()
                    .as_u64()
                    .unwrap()
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_with_input("sonic_rs::get_lazy", &src, |b, src| {
        b.iter_batched(
            || sonic_rs::from_str::<sonic_rs::LazyValue>(src).unwrap(),
            |v| {
                let tmp = v.get("search_metadata").unwrap();
                let one = tmp.get("max_id").unwrap().as_u64().unwrap();
                let two = tmp.get("count").unwrap().as_u64().unwrap();

                one + two
                    + v.get("statuses")
                        .unwrap()
                        .into_array_iter()
                        .unwrap()
                        .count() as u64
            },
            BatchSize::SmallInput,
        );
    });
}

fn citm_catalog(c: &mut Criterion) {
    let src = read_to_string("data/citm_catalog.json").unwrap();
    let mut group = c.benchmark_group("citm_catalog");

    group.throughput(Throughput::Bytes(src.len() as _));
    group.measurement_time(Duration::from_secs(20));

    group.bench_with_input("flexon::get_lazy (pointer)", src.as_str(), |b, src| {
        b.iter_batched(
            || flexon::parse::<_, flexon::LazyValue>(src).unwrap(),
            |v| {
                v.pointer(jsonp!["topicSubTopics", "107888604", 0])
                    .unwrap()
                    .as_u64()
                    .unwrap()
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_with_input("flexon::get_lazy", src.as_str(), |b, src| {
        b.iter_batched(
            || flexon::parse::<_, flexon::LazyValue>(src).unwrap(),
            |mut v| {
                let tmp = v["topicSubTopics"].as_object().unwrap();
                let one = tmp["107888604"][0].as_u64().unwrap();
                let two = tmp["324846098"][0].as_u64().unwrap();

                one + two + v["performances"].as_array().unwrap().actual_len() as u64
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_with_input("sonic_rs::get_lazy (pointer)", &src, |b, src| {
        b.iter_batched(
            || sonic_rs::from_str::<sonic_rs::LazyValue>(src).unwrap(),
            |v| {
                v.pointer(pointer!["topicSubTopics", "107888604", 0])
                    .unwrap()
                    .as_u64()
                    .unwrap()
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_with_input("sonic_rs::get_lazy", &src, |b, src| {
        b.iter_batched(
            || sonic_rs::from_str::<sonic_rs::LazyValue>(src).unwrap(),
            |v| {
                let tmp = v.get("topicSubTopics").unwrap();
                let one = tmp
                    .get("107888604")
                    .unwrap()
                    .get(0)
                    .unwrap()
                    .as_u64()
                    .unwrap();
                let two = tmp
                    .get("324846098")
                    .unwrap()
                    .get(0)
                    .unwrap()
                    .as_u64()
                    .unwrap();

                one + two
                    + v.get("performances")
                        .unwrap()
                        .into_array_iter()
                        .unwrap()
                        .count() as u64
            },
            BatchSize::SmallInput,
        );
    });
}

fn canada(c: &mut Criterion) {
    let src = read_to_string("data/canada.json").unwrap();
    let mut group = c.benchmark_group("canada");

    group.throughput(Throughput::Bytes(src.len() as _));
    group.measurement_time(Duration::from_secs(20));

    group.bench_with_input("flexon::get_lazy (pointer)", src.as_str(), |b, src| {
        b.iter_batched(
            || flexon::parse::<_, flexon::LazyValue>(src).unwrap(),
            |v| {
                v.pointer(jsonp![
                    "features",
                    0,
                    "geometry",
                    "coordinates",
                    479,
                    5275,
                    0
                ])
                .unwrap()
                .as_f64()
                .unwrap()
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_with_input("flexon::get_lazy", src.as_str(), |b, src| {
        b.iter_batched(
            || flexon::parse::<_, flexon::LazyValue>(src).unwrap(),
            |mut v| {
                // in case someone wants to complain ive tried chaining
                // like in the `sonic_rs::get_lazy` the performance regression was quite insignificant
                // you may try it yourself but i wont type it out here. its too verbose.
                let root = v["features"][0]["geometry"]["coordinates"][479]
                    .as_array()
                    .unwrap();
                let pair = root[5275].as_array().unwrap();
                let one = pair[0].as_f64().unwrap();
                let two = pair[1].as_f64().unwrap();

                one + two + root.actual_len() as f64
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_with_input("sonic_rs::get_lazy (pointer)", src.as_str(), |b, src| {
        b.iter_batched(
            || sonic_rs::from_str::<sonic_rs::LazyValue>(src).unwrap(),
            |v| {
                v.pointer(pointer![
                    "features",
                    0,
                    "geometry",
                    "coordinates",
                    479,
                    5275,
                    0
                ])
                .unwrap()
                .as_f64()
                .unwrap()
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_with_input("sonic_rs::get_lazy", src.as_str(), |b, src| {
        b.iter_batched(
            || sonic_rs::from_str::<sonic_rs::LazyValue>(src).unwrap(),
            |v| {
                // lifetime error
                let tmp = v.get("features").unwrap();
                let tmp = tmp.get(0).unwrap();
                let tmp = tmp.get("geometry").unwrap();
                let tmp = tmp.get("coordinates").unwrap();

                let root = tmp.get(479).unwrap();
                let pair = root.get(5275).unwrap();
                let one = pair.get(0).unwrap().as_f64().unwrap();
                let two = pair.get(1).unwrap().as_f64().unwrap();

                one + two + root.into_array_iter().unwrap().count() as f64
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, twitter, citm_catalog, canada);
criterion_main!(benches);
