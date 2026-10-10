//! Throughput of normalization across representative record sizes.
// Criterion generates a public runner without rustdoc.
#![allow(missing_docs)]

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn normalize(c: &mut Criterion) {
    let mut group = c.benchmark_group("normalize");
    let cases = [
        ("text", "Server ready".to_owned()),
        ("invalid", "{broken".to_owned()),
        ("small", r#"{"level":30,"time":1700000000123,"msg":"ready"}"#.to_owned()),
        ("nested", r#"{"level":"WARN","msg":"Request rejected","statusCode":401,"req":{"path":"/v1/check","tags":["a","b"]}}"#.to_owned()),
        ("exception", serde_json::json!({"level":"ERROR","message":"failed","error":{"stack":"frame\n".repeat(500)}}).to_string()),
        ("limit", "x".repeat(1024 * 1024 + 1)),
    ];
    for (name, line) in cases {
        if name == "limit" {
            // Rejected by length without scanning; byte throughput is misleading.
            group.throughput(Throughput::Elements(1));
        } else {
            group.throughput(Throughput::Bytes(line.len() as u64));
        }
        group.bench_with_input(BenchmarkId::from_parameter(name), &line, |b, line| {
            b.iter(|| lognorm::parse(black_box(line)));
        });
    }
    group.finish();
}

criterion_group!(benches, normalize);
criterion_main!(benches);
