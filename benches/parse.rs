use std::fs::read_to_string;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

fn parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");

    for id in ["canada", "twitter", "citm_catalog"] {
        let src = read_to_string(format!("data/{id}.json")).unwrap();

        group.throughput(Throughput::Bytes(src.len() as _));
        group.bench_with_input(BenchmarkId::from_parameter(id), &src, |b, src| {
            b.iter(|| flexon::parse(src));
        });
    }
}

criterion_group!(benches, parse);
criterion_main!(benches);
